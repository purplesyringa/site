#![feature(core_intrinsics)]

use core::intrinsics;
use core::mem::ManuallyDrop;
use criterion::{criterion_group, criterion_main, Criterion};

#[inline(always)]
fn catch<R, E, F: FnOnce() -> R>(f: F) -> Result<R, E> {
    union Data<F, R, E> {
        f: ManuallyDrop<F>,
        r: ManuallyDrop<R>,
        p: ManuallyDrop<E>,
    }

    let mut data = Data {
        f: ManuallyDrop::new(f),
    };

    let data_ptr = core::ptr::addr_of_mut!(data) as *mut u8;
    unsafe {
        return if intrinsics::catch_unwind(do_call::<F, R, E>, data_ptr, do_catch::<F, R, E>) == 0 {
            Ok(ManuallyDrop::into_inner(data.r))
        } else {
            Err(ManuallyDrop::into_inner(data.p))
        };
    }

    #[inline]
    fn do_call<F: FnOnce() -> R, R, E>(data: *mut u8) {
        // SAFETY: this is the responsibility of the caller, see above.
        unsafe {
            let data = data as *mut Data<F, R, E>;
            let data = &mut (*data);
            let f = ManuallyDrop::take(&mut data.f);
            data.r = ManuallyDrop::new(f());
        }
    }

    #[inline]
    fn do_catch<F: FnOnce() -> R, R, E>(data: *mut u8, payload: *mut u8) {
        // SAFETY: this is the responsibility of the caller, see above.
        //
        // When `__rustc_panic_cleaner` is correctly implemented we can rely
        // on `obj` being the correct thing to pass to `data.p` (after wrapping
        // in `ManuallyDrop`).
        unsafe {
            let data = data as *mut Data<F, R, E>;
            let data = &mut (*data);
            data.p = ManuallyDrop::new(cleanup(payload.cast()));
        }
    }
}

#[repr(C)]
struct UwException {
    class: u64,
    destructor: Option<extern "C" fn(u32, *mut Self)>,
    private: [*const (); 2],
}

#[repr(C)]
struct Exception<E> {
    uw: UwException,
    cause: ManuallyDrop<E>,
}

const CLASS: u64 = u64::from_ne_bytes(*b"RUSTpurp");

#[inline(always)]
fn throw<E>(cause: E) {
    let exception = unsafe {
        local_write(Exception {
            uw: UwException {
                class: CLASS,
                destructor: Some(destructor),
                private: [core::ptr::null(); 2],
            },
            cause: ManuallyDrop::new(cause),
        })
    };
    unsafe {
        _Unwind_RaiseException(exception.cast());
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
    ManuallyDrop::take(&mut (*exception.cast::<Exception<E>>()).cause)
}

extern "C-unwind" {
    fn _Unwind_RaiseException(exception: *mut UwException) -> u32;
}

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;

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

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("panic!", |b| {
        b.iter(|| {
            let _ = catch::<_, &'static str, _>(|| throw::<&'static str>("Hello, world!"));
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
