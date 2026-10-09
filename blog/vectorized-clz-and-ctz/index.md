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

On my Haswell, this runs at $0.45$ ns/iteration, compared to $1$ ns for the scalar version. When latency-bound, the numbers rise to $2$ ns vs $1$ ns (but if you're latency-bound on vectorized `ctz`, you're probably doing something wrong). On modern Intel CPUs, the numbers should be the same or better, while AMD CPUs make `lzcnt` so cheap that a scalar version will likely win. Though mind that Zen CPUs support AVX-512, which has `vplzcntd`, so that's an option, too.


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

This runs at $0.49$ ns/iteration and $2.3$ ns when latency-bound. The slowdown compared to `clz` is due to using one more instruction. It can be avoided by using `vpternlogq` if AVX-512 is present, but at that point you might as well run `vpopcntd` on `x ^ (x - 1)`. The scalar version behaves no differently from `clz`.
