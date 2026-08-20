---
title: accept() and socket inheritance
time: August 20, 2026
intro: |
    This post is about Windows.

    [Last time](../duplicatehandle-works-on-sockets-mostly/) we saw that sockets are pretty much just handles, and as such, they can be inherited. If a process creates an inheritable socket and runs `CreateProcess` with the appropriate parameters, its children will retain access to the socket. This has obvious security and functional implications, so it's important to be able to control which sockets are inheritable and which ones aren't.

    The most obvious way to create a socket is the BSD-style [`socket` function](https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-socket). The docs for `socket` don't mention this, but it creates an inheritable socket. If you want to make a non-inheritable one, [use `WSASocketW`](https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-wsasocketw). It extends `socket` with multiple parameters, including `dwFlags`, which controls, among other things, inheritance:

    > `WSA_FLAG_NO_HANDLE_INHERIT` -- Create a socket that is non-inheritable. A socket handle created by the `WSASocket` or the `socket` function is inheritable by default. When this flag is set, the socket handle is non-inheritable.
---

This post is about Windows.

[Last time](../duplicatehandle-works-on-sockets-mostly/) we saw that sockets are pretty much just handles, and as such, they can be inherited. If a process creates an inheritable socket and runs `CreateProcess` with the appropriate parameters, its children will retain access to the socket. This has obvious security and functional implications, so it's important to be able to control which sockets are inheritable and which ones aren't.

The most obvious way to create a socket is the BSD-style [`socket` function](https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-socket). The docs for `socket` don't mention this, but it creates an inheritable socket. If you want to make a non-inheritable one, [use `WSASocketW`](https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-wsasocketw). It extends `socket` with multiple parameters, including `dwFlags`, which controls, among other things, inheritance:

> `WSA_FLAG_NO_HANDLE_INHERIT` -- Create a socket that is non-inheritable. A socket handle created by the `WSASocket` or the `socket` function is inheritable by default. When this flag is set, the socket handle is non-inheritable.

Ideally `socket` would accept POSIX-style `SOCK_CLOEXEC`/`SOCK_CLOFORK`, but for now this is a good alternative.

On that note, if you use `WSASocketW`, make sure to pass `WSA_FLAG_OVERLAPPED` too. Unlike other APIs, overlapped sockets support non-overlapped operations just fine, so `socket` sets `WSA_FLAG_OVERLAPPED` by default, and your `WSASocketW` call likely needs to do so too.

But let's talk about the subject of this post, `accept`. It's another API that creates a socket, and this time, `WSAAccept` doesn't take a `dwFlags` argument, so it's not clear whether the socket is inheritable. This is completely undocumented, but I ran a couple experiments, and the result turns out to be incredibly strange:

**Whether the new socket is inheritable depends on whether the listener socket was inheritable *at creation time*.**

If you create an inheritable socket with `WSASocketW` and run `accept`, the result is inheritable. If you create a non-inheritable socket and run `accept`, the result is non-inheritable. That much is reasonable.

If you create an inheritable socket and then make its handle non-inheritable [with `SetHandleInformation`](https://learn.microsoft.com/en-us/windows/win32/api/handleapi/nf-handleapi-sethandleinformation), the accepted socket will *still* be inheritable -- and vice versa.

Similarly, if you use `DuplicateHandle` to produce a socket handle with different inheritability, the original value takes place.

Since `WSADuplicateSocketW` [runs `DuplicateHandle` under the hood](../duplicatehandle-works-on-sockets-mostly/), we can expect the `WSA_FLAG_NO_HANDLE_INHERIT` option on a duplicated listener to make no impact either. In fact, [the docs for `WSADuplicateSocketW`](https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-wsaduplicatesocketw) are the only place where we get a hint about this oddity:

> Both the source process and the destination process should pass the same flags to their respective `WSASocket` function calls.

They go on to talk about `WSA_FLAG_OVERLAPPED`, but it seems like it applies just as much to `WSA_FLAG_NO_HANDLE_INHERIT`, and that it's just as immutable.

Reading between the lines, it seems like `WSA_FLAG_NO_HANDLE_INHERIT` is not a property of a *socket handle*, but rather the kernel *socket object*.

Wine currently handles this incorrectly and always generates inheritable handles on `accept`, which is arguably more reasonable than whatever Afd is doing...

If you want to control inheritance on `accept` explicitly, you have to use [the low-level `AcceptEx` API](https://learn.microsoft.com/en-us/windows/win32/api/mswsock/nf-mswsock-acceptex), which offers a way to accept a connection into a pre-existing empty socket. `AcceptEx` forces overlapped I/O, so it's quite a cumbersome replacement for `accept`, [but maybe this template will help](https://github.com/purplesyringa/crossmist/blob/38b1ced/src/platform/windows/internals.rs#L121-L165).
