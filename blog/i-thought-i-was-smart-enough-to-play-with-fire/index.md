---
title: I thought I was smart enough to play with fire
time: June 20, 2024
discussion: https://codeforces.com/blog/entry/130661
intro: |
    [blazingio](https://codeforces.com/blog/entry/126390) cuts corners by design. It keeps the constant factor small and uses long forgotten algorithms people used before processors supported SIMD and integer division. But another limitation made this task much harder.

    Size.

    Professional libraries start exceeding the [Codeforces](https) limit of 64 KiB really fast. Code minification barely helps, and neither does resorting to ugly code. So I cut a corner I don't typically cut.

    Undefined Behavior.

    These two words make a seasoned programmer shudder. But sidestepping UB increases code size so much the library can hardly be used on CF. So I took a gamble. I meticulously scanned every instance of UB I used intentionally and made sure the compiler had absolutely no reason to miscompile it. I wrote excessive tests and run them on CI on all architecture and OS combinations I could think of. I released the library without so much as a flaw. It worked like clockwork.

    And then, 3 months later, I updated README, and all hell broke loose.
---

[blazingio](https://codeforces.com/blog/entry/126390) cuts corners by design. It keeps the constant factor small and uses long forgotten algorithms people used before processors supported SIMD and integer division. But another limitation made this task much harder.

Size.

Professional libraries start exceeding the [Codeforces](https://codeforces.com) limit of 64 KiB really fast. Code minification barely helps, and neither does resorting to ugly code. So I cut a corner I don't typically cut.

Undefined Behavior.

These two words make a seasoned programmer shudder. But sidestepping UB increases code size so much the library can hardly be used on CF. So I took a gamble. I meticulously scanned every instance of UB I used intentionally and made sure the compiler had absolutely no reason to miscompile it. I wrote excessive tests and run them on CI on all architecture and OS combinations I could think of. I released the library without so much as a flaw. It worked like clockwork.

And then, 3 months later, I updated README, and all hell broke loose.

---

CI retested the code, and two compiler/OS/architecture combinations failed: MinGW on 64-bit and 32-bit Windows. Both failed on a very particular test: `round-trip-bitset` on blazingio using SSE 4.1, file I/O, built without interactive problem support.

It took some time to figure what the problem even was, but eventually it turned out reading into bitsets didn't modify them. Here's how the bitset input tight loop looked like on SSE 4.1:

```cpp
// This is actually 0x0001020304050607
uint64_t a = ~0ULL / 65025;
((uint16_t*)&value)[i / 16] = _mm_movemask_epi8(
    _mm_shuffle_epi8(
        _mm_slli_epi32(_mm_loadu_si128(p++), 7),
        _mm_set_epi64x(a, a + ONE_BYTES * 8)
    )
);
```

`p` is a `const __m128i*` pointer to the input characters. We read in 16 consecutive characters which are expected to be `'0'` (code 0x30) or `'1'` (code 0x31), parse them into 16 bits, and then put the bits directly into the bitset object.

This is a clear strict aliasing violation. `value` stores a reference to a `std::bitset` instance, and we access that object via a `uint16_t*` pointer. This is UB.

My hope was that the compiler wouldn't _know_ `value` points at a `std::bitset` instance. After all, nothing stops you from safely casting `T*` to `U*` for any combinations of `T` and `U`, so it could be argued that it's _possible_ that the variable `value` (of type `std::bitset<N>&`) actually contains the address not of a `std::bitset<N>` object, but of a `uint16_t` array.

Besides, I've seen many people cast pointers to `std::bitset` like this for efficiency, and while I always thought the code was fishy, I couldn't argue against people's experience.

But apparently, for whatever reason, the compiler inlined the call to the bitset input function, TBAA activated _and_ managed to figure out this is UB.

---

blazingio doesn't contain any intentional UB anymore*. It is now also stress-tested under sanitizers, which couldn't been done when intentional UB was present. The library grew by several hundred bytes, and if you think that's a small increase, consider that blazingio takes 9405 bytes, and that's already 14% of the total code size limit. The slim version got a lot less slim.

That's the price you have to pay, I guess.

There's just one iffy UB-based optimization left. It couldn't be removed like others, because it matters for performance: omitting it causes GCC to spill registers to memory in a tight loop. I've put a lot of thought into the correctness of the workaround than I ever did regarding the `bitset` trick, so I'm reasonably certain it's going to stand the test of time.

You'll hear from me when it backfires, and the post is going to be titled "I made the mistake I warned people about".
