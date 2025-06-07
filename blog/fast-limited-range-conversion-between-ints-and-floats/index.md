---
title: Fast limited-range conversion between ints and floats
time: Jun 7, 2025
intro: |
  This post is about a popular but niche technique I can never find a succinct reference for. I didn't invent it, I just need a page I can link when giving optimization advice.

  Integer $\leftrightarrow$ float casts that utilize specialized processor instructions, i.e. those that compilers use by default, typically have worse throughput and higher latency than alternatives based on applying bit tricks to the IEEE-754 format. (Please benchmark them anyway, I've seen them decrease performance occasionally.) Unfortunately, these bit tricks only work over a reduced range, e.g. numbers up to $2^{23}$ or $2^{52}$ as opposed to the full $2^{32}$ or $2^{64}$ range. Still, they can be very useful in specialized tasks.
---

This post is about a popular but niche technique I can never find a succinct reference for. I didn't invent it, I just need a page I can link when giving optimization advice.

Integer $\leftrightarrow$ float casts that utilize specialized processor instructions, i.e. those that compilers use by default, typically have worse throughput and higher latency than alternatives based on applying bit tricks to the IEEE-754 format. (Please benchmark them anyway, I've seen them decrease performance occasionally.) Unfortunately, these bit tricks only work over a reduced range, e.g. numbers up to $2^{23}$ or $2^{52}$ as opposed to the full $2^{32}$ or $2^{64}$ range. Still, they can be very useful in specialized tasks.


### Snippets

All functions are vectorizable.

```rust
/// Convert an integer in range [0; 2^23) to a float exactly.
///
/// Produces an incorrect result for integers outside the range.
fn u23_to_f32(x: u32) -> f32 {
    let magic = ((1u32 << 23) as f32).to_bits();
    f32::from_bits(x ^ magic) - f32::from_bits(magic)
}

/// Convert an integer in range [0; 2^52) to a double exactly.
///
/// Produces an incorrect result for integers outside the range.
fn u52_to_f64(x: u64) -> f64 {
    let magic = ((1u64 << 52) as f64).to_bits();
    f64::from_bits(x ^ magic) - f64::from_bits(magic)
}

/// Convert a float in range [-0.25; 2^23] to the nearest integer, rounding ties to even.
///
/// Produces an incorrect result for floats outside the range or `NaN`s. Rounds just like
/// `x.round_ties_even()`.
fn f32_to_u23_rounding(x: f32) -> u32 {
    let magic = (1u32 << 23) as f32;
    (x + magic).to_bits() ^ magic.to_bits()
}

/// Convert a double in range [-0.25; 2^52] to the nearest integer, rounding ties to even.
///
/// Produces an incorrect result for doubles outside the range or `NaN`s. Rounds just like
/// `x.round_ties_even()`.
fn f64_to_u52_rounding(x: f64) -> u64 {
    let magic = (1u64 << 52) as f64;
    (x + magic).to_bits() ^ magic.to_bits()
}

/// Convert a double in range [-0.25; 2^32 - 0.5) to the nearest integer, rounding ties to even.
///
/// Produces an incorrect result for doubles outside the range or `NaN`s. Rounds just like
/// `x.round_ties_even()`.
fn f64_to_u32_rounding(x: f64) -> u32 {
    let magic = (1u64 << 52) as f64;
    (x + magic).to_bits() as u32
}
```

No alternatives for flooring are explicitly provided; if you has access to AVX-512, changing the rounding of addition in the last two methods to flooring should work.


### How this works

`(1u32 << 23) as f32` is an IEEE-754 number with the (unbiased) exponent set to $+23$ and a zero mantissa.

In `u23_to_f32`, $x$ is a $23$-bit integer. $x \oplus \mathrm{magic}$ is thus an IEEE-754 number with exponent $+23$ and mantissa $x$. It's mathematically equal to $2^{23} \cdot \left( 1 + \frac{x}{2^{23}} \right) = 2^{23} + x$. Subtracting a floating-point value $2^{23}$ thus produces an IEEE-754 number equal to $x$.

In `f32_to_u23`, $x$ is usually a non-negative number with exponent $< 23$. Adding $2^{23}$ thus produces a number with exponent exactly $23$. Its mantissa is therefore $(2^{23} + x) - 2^{23} = x$. XORing the number with $\mathrm{magic}$, which has the same exponent but a zero mantissa, leaves around just the mantissa $x$. Rounding matches the rounding of $+$. It works for $-0.25 \le x < 0$ because $+$ rounds such sums to $2^{23}$ exactly. It works for $x = 2^{23}$ because the effect of the exponent changing from $23$ to $24$ happens to flip the bit just before the mantissa from $0$ to $1$.

The situation for doubles and $64$-bit integers is similar.

`f64_to_u32_rounding` is equivalent to `f64_to_u52_rounding(x) as u32`; it's mentioned explicitly because the bottom $32$ bits of $\mathrm{magic}$ are zero, and thus the XOR can be optimized out. This trick was probably authored by [deft_code](https://stackoverflow.com/a/429812/5417677).

In cases where two different constants would make more intuitive sense, $\mathrm{magic}$ is used twice to improve code size and reduce register pressure.
