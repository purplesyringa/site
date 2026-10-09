---
title: Vectorized CLZ and CTZ
time: October 9, 2026
intro: |
    `clz` and `ctz` are instructions that compute the number of leading (trailing) zero bits in a fixed-size integer. They are natively supported by modern CPUs, though they are not always fast, e.g. `tzcnt` has a latency of `3` on Arrow Lake.

    I used `ctz` in an FPU emulator I'm working on, but figured out how to avoid it with floating-point trickery, and I just realized that this generalizes to a vectorizable `ctz` polyfill in a round-about way. I also implemented `clz` for completeness.
---

`clz` and `ctz` are instructions that compute the number of leading (trailing) zero bits in a fixed-size integer. They are natively supported by modern CPUs, though they are not always fast, e.g. `tzcnt` has a latency of `3` on Arrow Lake.

I used `ctz` in an FPU emulator I'm working on, but figured out how to avoid it with floating-point trickery, and I just realized that this generalizes to a vectorizable `ctz` polyfill in a round-about way. I also implemented `clz` for completeness.


### clz

Let's start with `clz`, the easier of the two:

```rust
fn clz(x: u32) -> u32 {
    let a = 2.0f64.powi(-970);
    32 - ((f64::from_bits(a.to_bits() | x as u64) - a).to_bits() >> 52) as u32
}
```

The general idea goes like this:

Floating-point exponents are biased logarithms of their values. By substituting a 32-bit number $x$ into the mantissa of some value $2^k$, we get a double representing $2^k (1 + 2^{-52} x)$. We can then subtract $2^k$ as a double to get $2^{k-52} x$. Extracting the exponent gives $k - 52 + 31 - \mathrm{clz}(x)$, from which `clz` can be computed with a bitwise subtraction. From that, $k$ can be chosen such that `clz` behaves correctly for $x = 0$.

We need $64$-bit doubles to handle $32$-bit inputs; unfortunately, this means that this trick can't work for arbitrary $64$-bit inputs, only up to $52$ bits.

Assuming the inputs and outputs are stored in `u64x4`, this compiles to:

```x86asm
    vpbroadcastq ymm1, [rip + bias]

    vorpd ymm0, ymm0, [rip + a]
    vsubpd ymm0, ymm0, [rip + a]
    vpsrlq ymm0, ymm0, 52
    vpsubq ymm0, ymm1, ymm0

a:
    .quad 0x350000000000000, 0x350000000000000, 0x350000000000000, 0x350000000000000
bias:
    .quad 32
```

On my Haswell, this runs at $0.45$ ns/iteration, compared to $1$ ns for the scalar version. When latency-bound, the numbers rise to $2$ ns vs $1$ ns (but if you're latency-bound on vectorized `ctz`, you're probably doing something wrong).

Ian Qvist tested this on Alder Lake (thanks!) and got $0.29$ ns/iteration, compared to $0.85$ ns for the scalar version, and $1.3$ ns vs $0.85$ ns when latency-bound. On modern Intel CPUs, the numbers should be the same or better.

AMD CPUs make `lzcnt` so cheap that a scalar version will likely win. Though keep in mind that Zen CPUs support AVX-512, which has `vplzcntd`, so that's an option, too.


### ctz

Now for `ctz`:

```rust
fn ctz(x: u32) -> u32 {
    let a = f64::from_bits((0x340000100000001 ^ x as u64) ^ (x as u64 + u32::MAX as u64))
        - f64::from_bits(0x340000000000000);
    (a.to_bits() >> 52) as u32
}
```

We start with $x \oplus (x - 1)$ to isolate the lowest set bit. `ctz` equals the logarithm of that value, which we determine by adding $2^k$ bitwise and subtracting $2^k$ as a double, then inspecting the exponent, which with a well-chosen $k$ contains the unbiased `ctz`. We pre-mix $2^{32}$ into the mantissa and use $x + 2^{32} - 1$ instead of $x - 1$ to handle $x = 0$ correctly, and pre-mix $1$ into the mantissa to ensure odd $x$ generate $a = 0$ and not a slow subnormal. (Can you *imagine* how much time I spent arranging this?)

This function compiles to:

```x86asm
    vpxor ymm1, ymm0, [rip + c1]
    vpaddq ymm0, ymm0, [rip + u32_max]
    vpxor ymm0, ymm1, ymm0
    vsubpd ymm0, ymm0, [rip + c2]
    vpsrlq ymm0, ymm0, 52

c1:
    .qword 0x340000100000001, 0x340000100000001, 0x340000100000001, 0x340000100000001
u32_max:
    .qword 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff
c2:
    .qword 0x340000000000000, 0x340000000000000, 0x340000000000000, 0x340000000000000
```

On Haswell, this runs at $0.49$ ns/iteration and $2.3$ ns when latency-bound. On Alder Lake, it's $0.35$ ns/iteration and $1.3$ ns when latency-bound. The slowdown compared to `clz` is due to using one more instruction. It can be avoided by using `vpternlogq` if AVX-512 is present, but at that point you might as well run `vpopcntd` on `(x - 1) & !x`. The scalar version behaves no differently from `clz`.

> **Added later:**

[Nikolay Malkovsky](https://t.me/a_zachem_eto_nuzhno) pointed out that [de Bruijn sequences](https://en.wikipedia.org/wiki/De_Bruijn_sequence) offer another vectorizable approach. After some testing, I arrived at the following code:

```c
const char table[32] = {
    0, 4, 5, 6, 11, 9, 7, 12, 15, 3, 10, 8, 14, 2, 13, 1,
    0, 4, 5, 6, 11, 9, 7, 12, 15, 3, 10, 8, 14, 2, 13, 1,
};
__m256i bit = _mm256_andnot_si256(x, _mm256_sub_epi32(x, _mm256_set1_epi32(1)));
__m256i high = _mm256_madd_epi16(
    _mm256_cmpeq_epi16(bit, _mm256_set1_epi16(-1)),
    _mm256_set1_epi16(-16)
);
__m256i index = _mm256_srli_epi32(_mm256_mullo_epi32(bit, _mm256_set1_epi32(0xf0a6f0a7)), 28);
__m256i low = _mm256_shuffle_epi8(_mm256_loadu_si256((__m256i*)table), index);
return _mm256_add_epi32(low, high);
```

We can't use a true 32-byte LUT because `vpshufb` cannot cross 16-byte lanes. The approach I used instead is tricky to explain, but essentially we use a 16-bit de Bruijn sequence repeated twice to compute bits 0-3 of the `ctz`, and then add $16$ or $32$ depending on which halves are zeroes. ~~Six seven~~ `0xf0a6f0a7` is one of only four magic constants that make this work.

This takes $1$ ns on Haswell ($0.7$ ns on Alder Lake), but has twice the throughput, so it may be a little faster than the FP-based approach if it helps avoid shuffling.

If you don't need to deal with $x = 0$ (or want $\mathrm{ctz}(0)$ to be $0$ and not $32$), using

```c
__m256i high = _mm256_and_si256(
    _mm256_cmpgt_epi32(bit, _mm256_set1_epi32(0x7fff)),
    _mm256_set1_epi32(16)
);
```

brings the time down to $0.82$ ns.
