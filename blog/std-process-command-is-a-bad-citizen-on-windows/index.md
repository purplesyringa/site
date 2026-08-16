---
title: std::process::Command is a bad citizen on Windows
time: August 16, 2026
intro: |
    Say you're writing a Windows IPC library for Rust, and you want to pass a handle to a child process. Maybe it's a pipe, maybe it's a file, maybe it's a broker process handle. It turns out that there's no correct way to do so due to Rust's idiosyncrasies.

    WinAPI trivia time! The `CreateProcess` function, which is used to spawn a process, takes two parameters that determine which handles are inherited by the child process. The possible combinations are:

    1. `bInheritHandles = FALSE`: no handles are inherited, except for stdio.
    2. `bInheritHandles = TRUE` and `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` is absent: all handles with the "inheritable" flag set are inherited.
    3. `bInheritHandles = TRUE` and `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` is present: all handles with the "inheritable" flag set that are listed in the handle list attribute are inherited.

    As long as every call to `CreateProcess` in your program follows (1) or (3), everything is fine: handles are allow-listed and no handle can be inherited by accident.
---

<aside-start-here />

Say you're writing a Windows IPC library for Rust, and you want to pass a handle to a child process. Maybe it's a pipe, maybe it's a file, maybe it's a broker process handle. It turns out that there's no correct way to do so due to Rust's idiosyncrasies.

:::aside
A Windows handle is roughly synonymous with a Unix file descriptor for the purposes of this post.
:::

WinAPI trivia time! The `CreateProcess` function, which is used to spawn a process, takes two parameters that determine which handles are inherited by the child process. The possible combinations are:

1. `bInheritHandles = FALSE`: no handles are inherited, except for stdio.
2. `bInheritHandles = TRUE` and `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` is absent: all handles with the "inheritable" flag set are inherited.
3. `bInheritHandles = TRUE` and `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` is present: all handles with the "inheritable" flag set that are listed in the handle list attribute are inherited.

As long as every call to `CreateProcess` in your program follows (1) or (3), everything is fine: handles are allow-listed and no handle can be inherited by accident.

But if there's a call following option (2), there's a problem. Let's say you want to pass a handle `H` to a child process. To do that, you need to raise its "inheritable" flag, at least for the duration of `CreateProcess`. But since handles are per-process, simultaneous calls to `CreateProcess` from other threads will also treat `H` as inheritable. If some of those calls follow (2), `H` will be inherited by the wrong process. Oopsie!

Can accidentally inherited handles cause issues? Why, yes:

- If you drop privileges, inherited handles can grant untrusted code access to protected resources.
- Keeping a handle alive longer than intended can leak memory.
- Files are usually undeletable while a handle to them exists, causing issues both to the user and your own program.

So, quiz time! Which option, out of the three, do you think Rust's `stdlib` uses?

To the surprise of absolutely no one, it's (2).

To be clear: you can't call `CreateProcess` manually as a workaround, because *everyone* in the process needs to avoid option (2), and you can't control how other libraries use `std::process::Command`. So you have to deal with the race condition.

The obvious solution to races is putting a mutex around the critical section -- in this case, the set "inheritable" + `CreateProcess` + clear "inheritable" code. And what do you know, apparently `std` [already does that](https://github.com/rust-lang/rust/blob/67854e511de21d881bb16426996cd4259d44aa2e/library/std/src/sys/process/windows.rs#L343)! So what's the issue?

Well, the only thing `std` does in the critical section is set "inheritable" for redirected stdio handles. It doesn't offer a way to set "inheritable" for your own handles, even on nightly, and it also doesn't expose the mutex publicly. So the lock is completely useless for anything `std` doesn't already support.

The best solution would be to follow option (3) in `std`. It's a breaking change, but [GHC did this in 2008](https://gitlab.haskell.org/ghc/ghc/-/work_items/2650), [JDK in 2013](https://bugs.openjdk.org/browse/JDK-6428742), and [Python in 2017](https://github.com/python/cpython/issues/63963), so apparently it's considered worthwhile. There were many false starts to fixing this in Rust as well, but I opened [an&nbsp;issue advocating for this solution today](https://github.com/rust-lang/rust/issues/161158) -- here's hoping for a good outcome.
