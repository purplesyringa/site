---
title: Race-free handle inheritance, maybe
time: August 18, 2026
intro: |
    [Last time](/blog/std-process-command-is-a-bad-citizen-on-windows/), I found out that Rust's implementation of subprocesses on Windows makes it impossible to spawn children that inherit selected handles without race conditions. Even if you do everything by the book, a simultaneous call to `std::process::Command` can inherit handles meant for another process. Since [this problem cannot be solved at the root](https://github.com/rust-lang/rust/issues/161158#issuecomment-5306592901) due to backwards compatibility hazards, at least not universally, I started looking for a different option.

    As a reminder, the problem is that to spawn a child that inherits a handle `h`, you have to temporarily set the "inheritable" flag for `h`, but that causes `std::process::Command` to inherit it as well. So we can't set the "inheritable" flag for any handle, ever.
---

[Last time](../std-process-command-is-a-bad-citizen-on-windows/), I found out that Rust's implementation of subprocesses on Windows makes it impossible to spawn children that inherit selected handles without race conditions. Even if you do everything by the book, a simultaneous call to `std::process::Command` can inherit handles meant for another process. Since [this problem cannot be solved at the root](https://github.com/rust-lang/rust/issues/161158#issuecomment-5306592901) due to backwards compatibility hazards, at least not universally, I started looking for a different option.

As a reminder, the problem is that to spawn a child that inherits a handle `h`, you have to temporarily set the "inheritable" flag for `h`, but that causes `std::process::Command` to inherit it as well. So we can't set the "inheritable" flag for any handle, ever.

Initially I tried to implement [an approach by Raymond Chen](https://devblogs.microsoft.com/oldnewthing/20260511-00/?p=112313). The idea behind it is that WinAPI allows you to spawn a child with a designated process as a "parent", which causes handles to be inherited from that process rather than yours. So the plan is:

- Spawn a temporary process (broker) storing the handles.
- Duplicate the handle `h` into the broker, and make it inheritable *there*.
- Spawn the real process with the broker as a parent, inheriting the copy of `h`.
- Bring down the broker.

This works because `h` never becomes inheritable in the current process, only in the temporary one, and we don't ever run `std::process::Command` in the broker. In fact, the broker doesn't have to run a single line of code, and so it can start suspended.

There are some details Raymond doesn't mention, though. First, you have to put the broker into a kill-on-close job so that it doesn't linger around if your process crashes at the wrong moment. This also messes up process trees (by design) and seems to be subtly broken on Wine.

Reusing the broker to avoid spawning twice as many processes has its own issues:

- stdio handles are inherited from the broker, so the child will start with stdio from when the broker was started, not the current one.
- You have to remember to delete the clone of `h` from the broker, and synchronize with that, lest the child gets "busy" errors.
- The broker cannot be used by multiple spawning processes, because passing the job handle via the broker creates a refcycle! Not to mention that if the parent crashes before deleting the cloned handles, the broker may keep them alive for longer than intended.

All in all, in practice this is too unreliable to use unless you control the entire program.

Next thing I tried was letting the child steal handles from the parent. There issue here is that if the parent crashes and its PID gets reused, you'll steal handles from the wrong guy. (Normally you'd pass a `HANDLE` instead of a PID, but here it's a Catch-22.) This is particularly dangerous in a privileged context, since you can be tricked into wrongly manipulating a handle of another process. [The official solution to detecting PID reuse](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-process#:~:text=Unique,created,-%2E) is to check if the process start time has changed, but `GetProcessTimes` uses a non-monotonic clock, so this check can give a false negative. Thanks MS!

I ended up spawning the child as a suspended process, duplicating `h` into it, passing the resulting handle with `WriteProcessMemory`, and resuming the process. To ensure that the suspended process doesn't remain if I crash during this procedure, I put it in a kill-on-close job, and disable kill-on-close after resumption to let it live independently. I set `JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK` to avoid deeply nested jobs. To close the race window where the child starts execution before becoming independent, I synchronize with it by passing a pipe as one of the handles. I know if the parent crashed before completing the protocol by watching the pipe.

You can find the code I wrote for this [here](https://github.com/purplesyringa/crossmist/blob/19e5ee410b38b19d240983f0bec4de7eb21b866b/src/platform/windows/subprocess.rs).

I have a feeling that AV software has a lot to say about `CREATE_SUSPEND` + `WriteProcessMemory`, but if you want me to give a shit about that, I'd rather like to see a working alternative to this mess first, because apparently Microsoft is so woke that they haven't discovered the concept of races, or something.
