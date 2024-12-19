---
title: Bringing faster exceptions to Rust
time: November 6, 2024
discussion:
    - https://www.reddit.com/r/rust/comments/1gl050z/bringing_faster_exceptions_to_rust/
    - https://news.ycombinator.com/item?id=42072750
intro: |
  Three months ago, I wrote about why [you might want to use panics for error handling](../you-might-want-to-use-panics-for-error-handling/). Even though it's a catchy title, panics are hardly suited for this goal, even if you try to hack around with macros and libraries. The real star is *the unwinding mechanism*, which powers panics. This post is the first in a series exploring what unwinding is, how to speed it up, and how it can benefit Rust and C++ programmers.
---

Three months ago, I wrote about why [you might want to use panics for error handling](../you-might-want-to-use-panics-for-error-handling/). Even though it's a catchy title, panics are hardly suited for this goal, even if you try to hack around with macros and libraries. The real star is *the unwinding mechanism*, which powers panics. This post is the first in a series exploring what unwinding is, how to speed it up, and how it can benefit Rust and C++ programmers.


### TL;DR

Check out the [Lithium](https://lib.rs/lithium) crate for faster exceptions and unwinding in Rust.


### Alternate returns

Typically, a function returns to the statement immediately following the call:

```rust
fn f() {
    let x = g();
    dbg!(x); // x = 123
}

fn g() -> i32 {
    return 123;
}
```

Now imagine that calls could specify *alternate* return points, letting the callee decide the statement to return to:

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

At first glance, this looks straightforward. Returning to an alternate address shouldn't be significantly more expensive than returning to the default address, so this has to be cheap.

But wait. This alternate return mechanism reminds me of something...

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

The core of the alternate return mechanism is *the unwinder*, a system library responsible for mapping default return addresses to alternate return addresses, passing alternate return values across calls, and consuming the return values. The specific API differs between operating systems, but on Linux, the main parts are these two functions:

- `_Unwind_RaiseException(Exception)`: Perform an alternate return, assuming we're currently in a default return point.
- `_Unwind_Resume(Exception)`: Perform an alternate return, assuming we're currently in an alternate return point.

So, what implementation detail makes panics and exceptions so slow? We'll uncover this in the series, and today, we'll try to speed up the Rust side of panic handling without modifying the unwinder.


## Digging deeper

### Benchmark

Let's start by measuring Rust's current panic performance with [criterion](https://docs.rs/criterion):

```rust
// Prevent spamming stderr with panic messages
std::panic::set_hook(Box::new(|_| {}));

b.iter(|| {
    let _ = std::panic::catch_unwind(|| panic!("Hello, world!"));
})
```

Result: `2.3814 µs`. That's less than a million panics per second. Why is it this slow?


### Macro

Let's see what happens when you call `panic!()`. After passing arguments through some macro calls, we land on `core::panic::panic_fmt`:

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

    unsafe { panic_impl(&pi) }
}
```

The format arguments are type-erased, which prevents some optimizations.

In addition, many Rust builtins panic, so `panic!` is defined in `core`, but the panic mechanism is OS-dependent, so panicking is implemented in `std`. Therefore, `panic_impl` is an extern function crossing crate boundaries, which prevents inlining without LTO.

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

Here, we generate a type-erased panic payload object that wraps the format arguments in another type-erased box, and then we invoke the panic hook -- before unwinding even starts!

Luckily, we can skip most of this logic by calling `std::panic::resume_unwind` instead of `panic!`. This function ignores the panic hook and takes a `Box<dyn Any + Send>` argument instead of an arbitrary format string, which lets us shed some load:

```rust
b.iter(|| {
    let _ = std::panic::catch_unwind(|| std::panic::resume_unwind(Box::new("Hello, world!")));
})
```

Result: `1.8379 µs`, a 24% improvement. Not bad for simply removing indirection!


### Direct calls

`resume_unwind` forwards calls to [rust_panic_without_hook](https://doc.rust-lang.org/1.82.0/src/std/panicking.rs.html#828-850):

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

There's still type-erasure here: firstly, the payload is `Box<dyn Any + Send>`, and secondly, we cast `&mut RewrapBox` to `&mut dyn PanicPayload`. None of this is necessary for statically typed alternate returns. The double-panic protection (`panic_count`) wouldn't be required in this context either.

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

Result: `580.44 ns`. That's a 68% improvement! It's not *sound*, as we're now messing with the panic counter, but we'll fix this soon.


### Catching

Let's figure out how to bypass the mirroring decrement of the panic count. We're looking for `std::panic::catch_unwind`, which merely forwards the call [here](https://doc.rust-lang.org/1.82.0/src/std/panicking.rs.html#474-584). After adding `#[inline(always)]`, removing `#[cold]`, and removing the panic count decrement, we restore soundness without affecting performance.


### panic_unwind

The next layer of abstraction to peel is these two functions:

```rust
extern "Rust" fn __rust_start_panic(payload: &mut dyn PanicPayload) -> u32;
extern "C" fn __rust_panic_cleanup(payload: *mut u8) -> *mut (dyn Any + Send + 'static);
```

Depending on the `-C panic="unwind/abort"` rustc flag, different crates providing these functions are linked. The crate we are interested in is `panic_unwind`. Its sources are available [on GitHub](https://github.com/rust-lang/rust/tree/1.82.0/library/panic_unwind).

Here we finally enter platform-specific code. I'm using Linux, so we're interested in the Itanium exception handling ABI (called `GCC` in Rust code). The implementation is [quite simple](https://github.com/rust-lang/rust/blob/1.82.0/library/panic_unwind/src/gcc.rs#L61-L106):

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

To throw a panic, we allocate *yet another* object on the heap and pass it to `_Unwind_RaiseException`. Catching a panic involves casting it back to a `Box` and retrieving the `cause` field.

To simplify this code for our statically annotated code, we can embed the cause directly in the exception object without wrapping it in `Box` beforehand. To separate our exceptions from Rust panics, we'll use a custom exception class:

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

Result: `562.69 ns`, or a 3% improvement. This isn't much, but every bit matters here.


### Allocations

We only have one heap allocation remaining now, containing the exception cause next to the `_Unwind_Exception` header for the system unwinder.

Why can't we put it on the stack? When `throw` performs an alternate return, its call frame can be overwritten by the catch handlers. We could store it inside the `catch` call frame, but then we'd need to pass a pointer to it to `throw`, complicating the API.

Thread-locals are the perfect middle ground, as they are almost as cheap as stack allocation:

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

While this is just a proof-of-concept (it doesn't work with nested or greater than 4 KiB exceptions), it indicates the resulting performance: `556.32 ns`, or a 1.5% improvement.


## Conclusions

### Gains

Starting at `2.3814 µs`, we've optimized down to `556.32 ns` -- a $4.3 \times$ speedup without loss in functionality. We secured this win without modifying the Rust compiler or the system unwinder by applying the following optimizations:

- Remove the hook invocation
- Remove type erasure of format arguments
- Remove panic counters
- Get rid of `dyn PanicPayload`
- Add inlining and mark `catch` code as hot
- Remove various non-inlined cross-crate invocations
- Avoid boxing the exception cause
- Store the exception object in a thread-local


### Beyond EH

While unwinding is popular for exception propagation, that's not the only use case. For example, if success is more rare than an error, success could be the alternate path rather than the error. Another use of lightweight unwinding is coroutines. Thinking outside the box might help you find other applications in your projects.


### Lithium

To make these optimizations accessible, I have released the [Lithium](https://lib.rs/lithium) crate, which supports efficient unwinding in Rust. It's light as `Li` and includes features beyond the ones supported by this prototype:

- Efficient rethrowing
- Nested exceptions
- Large exception objects
- Exceptions that aren't `Send + 'static`
- Broad target support, including Windows, macOS, Emscripten, and WASI
- Compatibility with the stable compiler, falling back to panics
- Support for native Rust panics inside `catch`
- `#![no_std]` support

Check out [the GitHub repository](https://github.com/iex-rs/lithium) and feel free to open issues!

### Limitations

There are some caveats:

- Using `lithium::throw` inside `std::panic::catch_unwind` (rather than `lithium::catch`) is unsound.
- On nightly, Lithium relies on the implementation details of std and rustc. I monitor changes to unwinding, so this should not be a significant issue.
- Lithium's API may evolve incompatibly with semver due to interoperability if unsound is discovered in Lithium. I do not expect this to be problematic past the first month.


### Stay tuned

In the following posts, we'll explore Itanium and SEH designs, dive into unwinder implementations, and and figure out how to speed up exceptions significantly based on this knowledge. [Subscribe to RSS](/blog/feed.rss) if you are interested.
