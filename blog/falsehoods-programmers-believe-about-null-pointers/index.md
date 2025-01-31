---
title: Falsehoods programmers believe about null pointers
time: January 30, 2025
discussion:
  - https://www.reddit.com/r/programming/comments/1ieagxg/falsehoods_programmers_believe_about_null_pointers/
  - https://news.ycombinator.com/item?id=42886189
intro: |
    Null pointers look simple on the surface, and that's why they're so dangerous. As compiler optimizations, intuitive but incorrect simplifications, and platform-specific quirks have piled on, the odds of making a wrong assumption have increased, leading to the proliferation of bugs and vulnerabilities.

    This article explores common misconceptions about null pointers held by many programmers, starting with simple fallacies and working our way up to the weirdest cases. Some of them will be news only to beginners, while others may lead experts down the path of meticulous fact-checking. Without further ado, let's dive in.
---

Null pointers look simple on the surface, and that's why they're so dangerous. As compiler optimizations, intuitive but incorrect simplifications, and platform-specific quirks have piled on, the odds of making a wrong assumption have increased, leading to the proliferation of bugs and vulnerabilities.

This article explores common misconceptions about null pointers held by many programmers, starting with simple fallacies and working our way up to the weirdest cases. Some of them will be news only to beginners, while others may lead experts down the path of meticulous fact-checking. Without further ado, let's dive in.


### 1.

**Dereferencing a null pointer immediately crashes the program.**

Everyone's first attempt to dereference a null pointer in C, C++, or Rust results either in `STATUS_ACCESS_VIOLATION` or a dreaded `Segmentation fault (core dumped)` message, which gives this misconception some credibility. However, higher-level languages and libraries like [Crashpad](https://chromium.googlesource.com/crashpad/crashpad) can handle the error and print a nice message and a backtrace before the crash. This is implemented by installing a [vectored exception handler](https://learn.microsoft.com/en-us/windows/win32/debug/vectored-exception-handling) on Windows and a [signal handler](https://en.wikipedia.org/wiki/C_signal_handling) on Unix-like platforms.


### 2.

**Dereferencing a null pointer eventually leads to program termination.**

While dereferencing a null pointer is a Bad Thing, it is by no means unrecoverable. Vectored exception and signal handlers can resume the program (perhaps from a different code location) instead of bringing the process down. For example, Go translates nil pointer dereferences to panics, which can be caught in user code with [recover](https://go.dev/blog/defer-panic-and-recover), and Java translates them to `NullPointerException`, which can also be caught by user code like any other exception.

In both cases, asking for forgiveness (dereferencing a null pointer and then recovering) instead of permission (checking if the pointer is null before dereferencing it) is an optimization. Comparing all pointers with null would slow down execution when the pointer *isn't* null, i.e. in the majority of cases. In contrast, signal handling is zero-cost until the signal is generated, which happens exceedingly rarely in well-written programs.


### 3.

**Dereferencing a null pointer always causes a signal, an exception, or is otherwise rejected by hardware.**

Let's ignore undefined behavior for now and assume that the dereference is not optimized out.

Before virtual memory was a thing, almost all memory was accessible. For example, x86 in real mode stored interrupt tables at addresses from `0` to `1024`. From the hardware point of view, dereferencing a null pointer is no different from dereferencing other pointers, and as such, it simply accessed memory at address `0`.

This is still the case on many embedded platforms. Dereferencing a null pointer is still considered UB, so if, for whatever reason, you need to access address `0`, there are two major ways to do this:

1. You can write the relevant code in assembly, which does not have UB.
2. If the hardware ignores the topmost bits of the address, you can access `0x80000000` (or similar) from C instead.


### 4.

**On modern conventional platforms, dereferencing a null pointer always causes a signal, an exception, or is otherwise rejected by hardware.**

Linux supports a [personality flag](https://man7.org/linux/man-pages/man2/personality.2.html) called `MMAP_PAGE_ZERO` for compatibility with programs developed for System V. Running a program under `setarch -Z` executes it with address `0` to `4096` (or whatever your page size is) mapped to a page of zeroes. Alternatively, you can use `mmap` to place memory at address `0` by hand. Many years ago, Wine used this trick (among others, like patching LDT) to run DOS applications without DOSBox.

This no longer works by default for security reasons. One man's treasure is another man's trash: if the kernel accidentally dereferences a null pointer while the memory at address `0` is mapped, it might interpret user-supplied data as a kernel data structure, which facilitates exploits. However, you can still enable this explicitly by running `sudo sysctl vm.mmap_min_addr=0`.

Despite this, there's a very modern and common platform that still maps memory at address `0`. It's WebAssembly. Isolation within a wasm container is unnecessary, so this does not ease security exploits, and as such, dereferencing a null pointer still works here.


### 5.

**Dereferencing a null pointer always triggers "UB".**

This one's tricky. The standard does say this triggers Undefined Behavior, but what this phrase *means* has significantly changed over time.

In ye olden times, the C standard was considered guidelines rather than a ruleset, *undefined behavior* was closer to *implementation-defined behavior* than dark magic, and optimizers were stupid enough to make that distinction irrelevant. On a majority of platforms, dereferencing a null pointer compiled and behaved exactly like dereferencing a value at address `0`.

For all intents and purposes, UB as we understand it today with spooky action at a distance didn't exist.

For example, the [HP-UX C compiler](https://stackoverflow.com/questions/58843458/hp-ux-cc-uses-a-default-setting-to-allow-null-dereferences-is-that-possible-in) had a CLI option to map a page of zeroes at address `0`, so that `*(int*)NULL` would return `0`. Certain programs relied on this behavior and had to be patched to run correctly on modern operating systems -- or be executed with a personality flag.

---

Now we enter the cursed territory.


### 6.

**The null pointer has address `0`.**

The C standard does not require the null pointer to have address `0`. The only requirement it imposes is for `(void*)x` to evaluate to a null pointer, where `x` is *a compile-time constant equal to zero*. Such patterns can easily be matched in compile time, so null pointers can have addresses other than `0`. Similarly, casting a pointer to a boolean (as in `if (p)` and `!p`) is required to produce `false` for null pointers, not for zero pointers.

This is not a hypothetical: [some real architectures](https://c-faq.com/null/machexamp.html) and C interpreters use non-zero null pointers. `fullptr` is not really a joke.

If you're wondering, Rust and other modern languages usually don't support this case.


### 7.

**The null pointer has address `0` on modern platforms.**

On GPU architectures like [AMD GCN](https://reviews.llvm.org/D26196) and [NVIDIA Fermi](https://what.thedailywtf.com/topic/8661/sometimes-checking-for-null-pointers-is-a-mistake-nvidia-cuda/13), `0` points to accessible memory. At least on AMD GCN, the null pointer is represented as `-1`. (I'm not sure if that holds for Fermi, but that would be reasonable.)


### 8.

**Since `(void*)0` is a null pointer, `int x = 0; (void*)x` must be a null pointer, too.**

In `int x = 0; (void*)x`, `x` is not a constant expression, so the standard does not require it to produce a null pointer. Runtime integer-to-pointer casts are often no-ops, so adding `if (x == 0) x = ACTUAL_NULL_POINTER_ADDRESS;` to every cast would be very inefficient, and generating a null pointer conditional on optimizations seeing through runtime values would be unnecessarily inconsistent.

Obviously, `void *p; memset(&p, 0, sizeof(p)); p` is not guaranteed to produce a null pointer either.


### 9.

**On platforms where the null pointer has address `0`, C objects may not be placed at address `0`.**

A pointer to an object is not a null pointer, even if it has the same address.

If you know what pointer provenance is, pointers with the same bitwise representation behaving differently shouldn't be news to you:

```c
int x[1];
int y = 0;
int *p = x + 1;
// This may evaluate to true
if (p == &y) {
    // But this will be UB even though p and &y are equal
    *p;
}
```

Similarly, objects can be placed at address `0` even though pointers to them will be indistinguishable from `NULL` in runtime:

```c
int tmp = 123; // This can be placed at address 0
int *p = &tmp; // Just a pointer to 0, does not originate from a constant zero
int *q = NULL; // A null pointer because it originates from a constant zero
// p and q will have the same bitwise representation, but...
int x = *p; // produces 123
int y = *q; // UB
```


### 10.

**On platforms where the null pointer has address `0`, `int x = 0; (void*)x` is a null pointer.**

The result of an integer-to-pointer conversion is implementation-defined. While a null pointer is an obvious candidate, this can also produce an invalid pointer or even a dereferenceable pointer to an object at address `0`. Certain compilers [encouraged](https://c-faq.com/.xx/q5.19.html) this pattern for accessing memory at address `0` soundly:

```c
int *p = (void*)0; // Must produce a NULL pointer
int x = *p; // UB

int zero = 0;
int *q = (void*)zero; // May produce a dereferenceable pointer on some compilers
int y = *q; // Not necessarily UB
```

This is mostly a C legacy: most languages don't differentiate between runtime and compile-time integer-to-pointer casts and will exhibit consistent behavior.


### 11.

**On platforms where the null pointer has address `0`, `int x = 0; (void*)x` will compare equal to `NULL`.**

In C, pointers to objects are documented to compare as unequal to `NULL`, even if the object is at address `0`. In other words, knowing the addresses of pointers is not enough to compare them. This is one of the rare cases where provenance affects program execution in a way that does not cause UB.

The following asserts hold:

```c
extern int tmp; // Suppose this is at address 0
int *p = &tmp;
assert(p != NULL); // Pointer to object compares unequal to NULL

int *q = (void*)(uintptr_t)p;
assert(p == q); // Round-tripping produces a possibly invalid, but equal pointer
assert(q != NULL); // By transitivity

int x = 0;
int *r = (void*)x; // This is still round-tripping, lack of data dependency on p is irrelevant
assert(r != NULL);
```

As provenance is not accessible in runtime, such comparisons can only be resolved in compile time. So if a pointer to an object might cross an FFI boundary or be passed to complex code, that object can't be realistically placed at address `0`.

Even if there is no object at address `0`, `int x = 0; (void*)x` is still allowed to produce a pointer that compares unequal to `NULL`, as the conversion is implementation-defined.

In Rust, objects are not allowed to be placed at address `0` explicitly.


### 12.

**On platforms where the null pointer has address `0`, null pointers are stored as zeroes.**

The address of a pointer as revealed by integer casts and the bitwise representation of a pointer don't have to be equal, much like casting an integer to a float does not retain the bits.

Segmented addressing is a common example, but pointer authentication is a more modern instance of this effect. On ARM, the top byte of a pointer can be configured to store a cryptographic signature, which is then verified at dereference. Pointers inside [\__ptr_auth](https://github.com/swiftlang/llvm-project/blob/65e6c0eccdc1b63a0598b735dabaccf0d575a6b4/clang/docs/PointerAuthentication.rst#ptrauth-qualifier) regions are signed, storing the signature in addition to the address. Apple decided against signing null pointers, as this would make their values unpredictable during compile time. Still, this was a deliberate decision rather than an implication of the standard.

CHERI is even weirder. CHERI pointers store $128$-bit capabilities in addition to the $64$-bit address we're used to to protect against UAF and OOB accesses. Any pointer with address `0` is considered a null pointer, so there are effectively $2^{128}$-ish different null pointers, only one of which is all-zero. (This also means that comparing pointers for equality can yield different results than comparing their binary representations.)

If you extend the definition of pointers to include pointers to class members, this gets even more realistic. Pointers to members are, in effect, offsets to fields (at least if we aren't taking methods into account), and `0` is a valid offset, so `(int Class::*)nullptr` is usually stored as `-1`.


### Conclusion

Null pointers are even more cursed than pointers in general, and provenance already makes pointers quite complicated. Being aware of edge cases like these is valuable to prevent accidentally non-portable code and interpret other people's code correctly.

But if this sounds like an awful lot to keep in mind all the time, you're missing the point. Tailoring rules and programs to new environments as more platforms emerged and optimizing compilers got smarter is what got us into this situation in the first place.

Many people call C a "portable assembler". This is emphatically not the case. C *looks* close to hardware, but in reality this language has its own abstract machine and operational semantics. Optimization passes, code-generating backends, and libraries need to speak a platform-independent language to work in tandem, and that language is not "whatever hardware does". Instead of translating what you'd like the hardware to perform to C literally, treat C as a higher-level language, because it *is* one.

Python does not suffer from horrible memory safety bugs and non-portable behavior not only because it's an interpreted language, but also because software engineers don't try to outsmart the compiler or the runtime. Consider applying the same approach to C.

- Do you *need* to `memset` this structure, or will `= {0}` do the trick?
- Why are you casting pointers to `size_t`? Use `uintptr_t` instead.
- Why are you even round-tripping through integers? Use `void*` as an untyped/unaligned pointer type.
- Instead of crafting branchless code like `(void*)((uintptr_t)p * flag)` by hand, let the compiler optimize `flag ? p : NULL` for you.
- Can you store flags next to the pointer instead of abusing its low bits? If not, can you insert flags with `(char*)p + flags` instead of `(uintptr_t)p | flags`?

If your spider sense tingles, consult the C standard, then your compiler's documentation, then ask compiler developers. Don't assume there are no long-term plans to change the behavior and certainly don't trust common sense.

When all else fails, do the next best thing: document the assumptions. This will make it easier for users to understand the limits of your software, for developers to port your application to a new platform, and for you to debug unexpected problems.

*Next up: an architecture that stores memory addresses in IEEE-754 floats.*
