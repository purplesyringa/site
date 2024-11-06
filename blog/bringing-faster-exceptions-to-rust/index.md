---
title: Bringing faster exceptions to Rust
time: November 6, 2024
intro: |
    Three months ago, I wrote a post on why [you might want to use panics for error handling](../you-might-want-to-use-panics-for-error-handling/). That's a clickbaity title: panics are badly suited for this goal, even if you try to hack around with macros and libraries. The real treasure is *the unwinding mechanism*. This is the first of a series of posts on what unwinding is, how to speed it up, and how it can help Rust and C++ programmers
---

Three months ago, I wrote a post on why [you might want to use panics for error handling](../you-might-want-to-use-panics-for-error-handling/). That's a clickbaity title: panics are badly suited for this goal, even if you try to hack around with macros and libraries. The real treasure is *the unwinding mechanism*. This is the first of a series of posts on what unwinding is, how to speed it up, and how it can help Rust and C++ programmers.


### TL;DR

Check out the [Lithium](https://lib.rs/lithium) crate.


### The what?

Normally, when you call a function, execution proceeds to the statement after the call when the function returns:

```rust
fn f() {
    let x = g();
    dbg!(x); // x = 123
}

fn g() -> i32 {
    return 123;
}
```

However, suppose that some calls could specify *alternate* return points, and the callee could decide whether it wishes to return to the main return point or the alternate one:

```rust
// Dreamed-up syntax
fn f() {
    g() alternate |x| {
        dbg!(x); // x = 123
    };
}

fn g() -> () alternate i32 {
    return_alternate 123;
}
```

This looks simple. Returning to an alternate address shouldn't be significantly more expensive than returning to the normal address, so this has to be cheap.

But wait. If the function returns either of two values, it's as if it returned a success value or an error. This reminds me of something...

```rust
// Dreamed-up syntax
fn f() {
    g() catch |x| {
        dbg!(x); // x = 123
    };
}

fn g() -> () throws i32 {
    throw 123;
}
```

That's just exceptions! And we all know exceptions are slow. How did we get from alternate return addresses to something you should avoid at all costs in performant code?


### Dramatis personae

The core of the alternate return mechanism is *the unwinder*. This is a system library that knows how to map main return addresses to alternate return addresses, how to pass alternate return values from the callee to the caller, and how to consume this return value. The details differ between operating systems and runtimes, but on Linux, the main parts of the unwinder API are these two functions:

- `_Unwind_RaiseException(Exception)`: Perform an alternate return, assuming we're current in a normal return point.
- `_Unwind_Resume(Exception)`: Perform an alternate return, assuming we're current in an alternate return point.

So, what is it about the implementation of exceptions and panics that makes them so inefficient? This is what we'll be digging into over the course of this post series, and today we'll see if we can speed up the Rust side of panic handling in particular, without touching the unwinder implementation itself.


## Digging deeper

### Benchmark

Before we start optimizing stuff, let's see where we stand at the moment. I'm going to use the [criterion](https://docs.rs/criterion) framework for benchmarking, and the first attempt is:

```rust
// Prevent spamming stderr with panic messages
std::panic::set_hook(Box::new(|_| {}));

b.iter(|| {
    let _ = std::panic::catch_unwind(|| panic!("Hello, world!"));
})
```

`2.3814 µs`. That's less than 1 millions panics per second, huh. Okay, so why does that happen?


### Macro

Let's see what happens when you call `panic!()`. After passing arguments through some macro calls, we arrive at `core::panic::panic_fmt`:

```rust
pub const fn panic_fmt(fmt: fmt::Arguments<'_>) -> ! {
    // snip

    extern "Rust" {
        #[lang = "panic_impl"]
        fn panic_impl(pi: &PanicInfo<'_>) -> !;
    }

    let pi = PanicInfo::new(
        fmt,
        Location::caller(),
        /* can_unwind */ true,
        /* force_no_backtrace */ false,
    );

    // SAFETY: `panic_impl` is defined in safe Rust code and thus is safe to call.
    unsafe { panic_impl(&pi) }
}
```

The first thing we need to know is that the formatting arguments are necessarily type-erased, as `panic_impl` is an extern function defined elsewhere, so no optimizations are likely to occur here (barring LTO).

Where is the lang item `panic_impl` defined? It's eventually resolved to `std::panicking::begin_panic_handler`, defined in [std](https://doc.rust-lang.org/1.82.0/src/std/panicking.rs.html#595-679) as opposed to `core`. This is reasonable: while formatting is OS-independent, the actual panicking mechanism is OS-dependent, so we have to enter `std` eventually. Why is the `panic!` macro in `core` in the first place? That's because many Rust builtins panic, and making them work in `#![no_std]` requires the macro to be in `core`.

```rust
pub fn begin_panic_handler(info: &core::panic::PanicInfo<'_>) -> ! {
    struct FormatStringPayload<'a> { /* snip */ }

    // snip

    unsafe impl PanicPayload for FormatStringPayload<'_> {
        fn take_box(&mut self) -> *mut (dyn Any + Send) {
            // We do two allocations here, unfortunately. But (a) they're required with the current
            // scheme, and (b) we don't handle panic + OOM properly anyway (see comment in
            // begin_panic below).
            let contents = mem::take(self.fill());
            Box::into_raw(Box::new(contents))
        }

        // snip
    }

    // snip

    crate::sys::backtrace::__rust_end_short_backtrace(move || {
        if let Some(s) = msg.as_str() {
            // snip
        } else {
            rust_panic_with_hook(
                &mut FormatStringPayload { inner: &msg, string: None },
                loc,
                info.can_unwind(),
                info.force_no_backtrace(),
            );
        }
    })
}


fn rust_panic_with_hook(
    payload: &mut dyn PanicPayload,
    location: &Location<'_>,
    can_unwind: bool,
    force_no_backtrace: bool,
) -> ! {
    // snip
    match *HOOK.read().unwrap_or_else(PoisonError::into_inner) {
        // snip
        Hook::Custom(ref hook) => {
            hook(&PanicHookInfo::new(location, payload.get(), can_unwind, force_no_backtrace));
        }
    }
    // snip
    rust_panic(payload)
}
```

Okay, that's a lot of code, but the gist of it is that we generate a type-erased panic payload object that wraps the format arguments in another type-erased box, among other things, and then we invoke the panic hook that prints the traceback (or does nothing, in our benchmark). All of that before unwinding even starts!

Luckily, we can skip most of this machinery by calling `std::panic::resume_unwind` instead of `panic!`. This function skips calling the panic hook and takes a `Box<dyn Any + Send>` argument instead of an arbitrary format string, so we can shed some load:

```rust
b.iter(|| {
    let _ = std::panic::catch_unwind(|| std::panic::resume_unwind(Box::new("Hello, world!")));
})
```

`1.8379 µs (-23.711%)`. That's better! We've only removed indirection and it's already working significantly better.


### More indirection

`resume_unwind` calls directly into [rust_panic_without_hook](https://doc.rust-lang.org/1.82.0/src/std/panicking.rs.html#828-850):

```rust
pub fn rust_panic_without_hook(payload: Box<dyn Any + Send>) -> ! {
    panic_count::increase(false);

    struct RewrapBox(Box<dyn Any + Send>);

    unsafe impl PanicPayload for RewrapBox {
        fn take_box(&mut self) -> *mut (dyn Any + Send) {
            Box::into_raw(mem::replace(&mut self.0, Box::new(())))
        }
        // snip
    }
    // snip
    rust_panic(&mut RewrapBox(payload))
}

fn rust_panic(msg: &mut dyn PanicPayload) -> ! {
    let code = unsafe { __rust_start_panic(msg) };
    rtabort!("failed to initiate panic, error {code}")
}

extern "Rust" {
    /// `PanicPayload` lazily performs allocation only when needed (this avoids
    /// allocations when using the "abort" panic runtime).
    fn __rust_start_panic(payload: &mut dyn PanicPayload) -> u32;
}
```

All we need to know is that there's still quite a bit of type-erasure here: firstly, the payload is `Box<dyn Any + Send>`, and secondly, we cast `&mut RewrapBox` to `&mut dyn PanicPayload`. None of this would be necessary if we just wanted to perform statically checked alternate returns. The double-panic protection (`panic_count`) wouldn't be necessary in this context either.

So what do you say we call `__rust_start_panic` directly?

```rust
#![feature(std_internals)]

use core::any::Any;
use core::panic::PanicPayload;

struct RewrapBox(Box<dyn Any + Send>);

unsafe impl PanicPayload for RewrapBox {
    fn take_box(&mut self) -> *mut (dyn Any + Send + 'static) {
        Box::into_raw(core::mem::replace(&mut self.0, Box::new(())))
    }

    fn get(&mut self) -> &(dyn Any + Send + 'static) {
        &*self.0
    }
}

impl core::fmt::Display for RewrapBox {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Box<dyn Any>")
    }
}

unsafe extern "Rust" {
    safe fn __rust_start_panic(payload: &mut dyn PanicPayload) -> u32;
}

b.iter(|| {
    let _ = std::panic::catch_unwind(|| {
        __rust_start_panic(&mut RewrapBox(Box::new("Hello, world!")))
    });
})
```

`580.44 ns (-68.481%)`. That's better. It's not quite *sound*, as we're now messing with the panic counter, but this'll suffice for a benchmark.


### Catching

We've just removed the panic counter increment, so let's figure out how to remove the mirroring decrement to restore the balance and keep our code sound. We're looking for `std::panic::catch_unwind`, which just forwards the call [here](https://doc.rust-lang.org/1.82.0/src/std/panicking.rs.html#474-584). I won't duplicate all the code, but all I did was add `#[inline(always)]`, remove `#[cold]`, and remove the panic count decrement. This results in `578.96 ns (-0.2550%)`, which is just noise.


### panic_unwind

The next layer of abstraction to peel is these two functions:

```rust
extern "Rust" fn __rust_start_panic(payload: &mut dyn PanicPayload) -> u32;
extern "C" fn __rust_panic_cleanup(payload: *mut u8) -> *mut (dyn Any + Send + 'static);
```

And the reason they are extern is yet another indirection. Depending on the configuration, Rust panics can either trigger unwinding or abort the program. This behavior is controlled by the `-C panic="unwind/abort"` rustc flag. Depending on its value, different crates providing these two functions are linked in. The crate we are interested in is `panic_unwind`. It's sources aren't available on rust-lang.org, so we'll have to use [GitHub](https://github.com/rust-lang/rust/tree/1.82.0/library/panic_unwind).

This is where we finally enter platform-dependent code. I'm using Linux, so we're interested in the Itanium exception handling ABI (called `GCC` in Rust code). The implementation here is [quite simple](https://github.com/rust-lang/rust/blob/1.82.0/library/panic_unwind/src/gcc.rs#L61-L106):

```rust
pub unsafe fn panic(data: Box<dyn Any + Send>) -> u32 {
    let exception = Box::new(Exception {
        _uwe: uw::_Unwind_Exception {
            exception_class: rust_exception_class(),
            exception_cleanup: Some(exception_cleanup),
            private: [core::ptr::null(); uw::unwinder_private_data_size],
        },
        canary: &CANARY,
        cause: data,
    });
    let exception_param = Box::into_raw(exception) as *mut uw::_Unwind_Exception;
    return uw::_Unwind_RaiseException(exception_param) as u32;

    // snip
}

pub unsafe fn cleanup(ptr: *mut u8) -> Box<dyn Any + Send> {
    let exception = ptr as *mut uw::_Unwind_Exception;
    if (*exception).exception_class != rust_exception_class() {
        // snip
    }

    let exception = exception.cast::<Exception>();
    // snip
    let exception = Box::from_raw(exception as *mut Exception);
    exception.cause
}
```

To throw a panic, we allocate *yet another* object on the heap and pass it to `_Unwind_RaiseException`. To catch a panic, we cast it back to a `Box` and retrieve the `cause` field.

To simplify this code for out statically annotated code, we can embed the cause directly in the exception object, without wrapping it in `Box` beforehand. To separate our exceptions from Rust panics, we'll use our own exception class, too:

```rust
#[repr(C)]
struct UwException {
    class: u64,
    destructor: Option<extern "C" fn(u32, *mut Self)>,
    private: [*const (); 2],
}

#[repr(C)]
struct Exception<E> {
    uw: UwException,
    cause: E,
}

const CLASS: u64 = u64::from_ne_bytes(*b"RUSTpurp");

#[inline(always)]
fn throw<E>(cause: E) {
    let exception = Box::new(Exception {
        uw: UwException {
            class: CLASS,
            destructor: Some(destructor),
            private: [core::ptr::null(); 2],
        },
        cause,
    });
    unsafe {
        _Unwind_RaiseException(Box::into_raw(exception).cast());
    }
    std::process::abort();
}

extern "C" fn destructor(_code: u32, _exception: *mut UwException) {
    std::process::abort();
}

#[inline(always)]
unsafe fn cleanup<E>(exception: *mut UwException) -> E {
    if (*exception).class != CLASS {
        std::process::abort();
    }
    Box::from_raw(exception.cast::<Exception<E>>()).cause
}

extern "C-unwind" {
    fn _Unwind_RaiseException(exception: *mut UwException) -> u32;
}

b.iter(|| {
    let _ = catch::<_, &'static str, _>(|| throw::<&'static str>("Hello, world!"));
})
```

`562.69 ns (-3.058%)`. This isn't much, but every bit matters here.


### Allocations

We only have one heap allocation remaining now, storing the exception cause along with the `_Unwind_Exception` header for the system unwinder.

Why can't we put it on the stack? When `throw` performs an alternate return, its callframe is popped from the call stack and can be overridden by the catch handlers, such as destructors of locals. So storing the exception object inside the `throw` callframe will fail.

We could store it inside the `catch` callframe, but then we'd need to pass a pointer to it to `throw`, which would a) make exceptions non-zero-cost in the happy path, which might be less than ideal, b) complicates the API, requiring the pointer to be passed through the callstack.

So instead, we'll use thread-locals. These are cheaper than heap allocations and not significantly more expensive than stack allocation.

```rust
thread_local! {
    static LOCAL: UnsafeCell<MaybeUninit<[u8; 4096]>> = const {
        UnsafeCell::new(MaybeUninit::uninit())
    };
}

unsafe fn local_write<T>(x: T) -> *mut T {
    let p = LOCAL.with(|local| local.get().cast::<T>());
    unsafe {
        p.write(x);
    }
    p
}
```

This is just a proof-of-concept that doesn't work with nested exceptions, larger than 4K exception objects, and so on. The result is `556.32 ns (-1.4666%)`.


## Conclusions

### Comparison

We started with `2.3814 µs` and arrived at `556.32 ns`: $4.3$ times faster without loss in functionality. We managed to secure this win without modifying the Rust compiler or the system unwinder. We have figured out that the Rust panic runtime is unoptimized for performance (perhaps by design), but there's nothing stopping us from performing that optimization.


### Use cases

While unwinding is usually used for exception propagation, that's not the only use case. For example, if a successful return is more rare than an error, success could be the alternate path, rather than the error. Another application of lightweight unwinding is coroutines. Think outside the box and try to mentally separate unwinding from exceptions.


### Projects

I have released a crate named [Lithium](https://lib.rs/lithium) to support efficient unwinding in Rust (light as `Li`) using the method described in this post. While the API of the crate is mostly suited for exceptions, it's generic enough to be usable for any unwinding.

Compared to the implementation in this post, Lithium also handles:

- Efficient rethrowing
- Nested exceptions
- Large exception objects
- Targets other than x86-64 Linux, such as Windows, macOS, Emscripten, and WASM
- Stable compilers with a fallback to (slightly optimized) panics
- Native Rust panics inside `catch`

Please feel free to check it out and open issues [on GitHub](https://github.com/iex-rs/lithium)!

Some pitfalls to be aware of are:

- Using `lithium::throw` inside `std::panic::catch_unwind` (rather than `lithium::catch`) is unsound.
- Nightly Lithium relies on implementation details of std and rustc, so it might theoretically break in nightly if the unwinding implementation is updated. I have a CI set up and monitor changes to rustc, so this should not be a significant issue.
- The API is in flux and might theoretically undergo non-semver-compatible changes for a while despite the crate version being `1.0`. This is for interoperability reasons and will not happen unless soundness bugs are found in the very near time, so you should be safe to play around with it.
