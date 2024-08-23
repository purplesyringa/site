---
title: I sped up serde_json strings by 20%
time: August 20, 2024
discussion: https://www.reddit.com/r/rust/comments/1eyxspu/i_sped_up_serde_json_strings_by_20/
intro: |
    I have recently done some performance work and realized that reading about my experience could be entertaining. Teaching to *think* is just as important as teaching to *code*, but this is seldom done; I think something I've done last month is a great opportunity to draw the curtain a bit.

    `serde` is *the* Rust framework for serialization and deserialization. Everyone uses it, and it's the default among the ecosystem. `serde_json` is the official `serde` "mixin" for JSON, so when people need to parse stuff, that's what they use instinctively. There are other libraries for JSON parsing, like [simd-json](https://lib.rs/crates/simd-json), but `serde_json` is overwhelmingly used: it has [26916](https://crates.io/crates/serde_json/reverse_dependencies) dependents at the time of this post, compared to only [66](https://crates.io/crates/simd-json/reverse_dependencies) for `simd-json`.

    This makes `serde_json` a good target <s>(not in a Jia Tan way)</s> for optimization. Chances are, many of those 26916 users would profit from switching to `simd-json`, but as long as they aren't doing that, smaller optimizations are better than nothing, and such improvements are reapt across the ecosystem.
---

I have recently done some performance work and realized that reading about my experience could be entertaining. Teaching to *think* is just as important as teaching to *code*, but this is seldom done; I think something I've done last month is a great opportunity to draw the curtain a bit.

`serde` is *the* Rust framework for serialization and deserialization. Everyone uses it, and it's the default among the ecosystem. `serde_json` is the official `serde` "mixin" for JSON, so when people need to parse stuff, that's what they use instinctively. There are other libraries for JSON parsing, like [simd-json](https://lib.rs/crates/simd-json), but `serde_json` is overwhelmingly used: it has [26916](https://crates.io/crates/serde_json/reverse_dependencies) dependents at the time of this post, compared to only [66](https://crates.io/crates/simd-json/reverse_dependencies) for `simd-json`.

This makes `serde_json` a good target <s>(not in a Jia Tan way)</s> for optimization. Chances are, many of those 26916 users would profit from switching to `simd-json`, but as long as they aren't doing that, smaller optimizations are better than nothing, and such improvements are reapt across the ecosystem.


### Where do I start?

I have recently been working on the [#[iex]](../you-might-want-to-use-panics-for-error-handling/) library. I used `serde` and `serde_json` as benchmarks and noticed some questionable decisions in their performance-critical code while rewriting it to better suit `#[iex]`.

`#[iex]` focuses on error handling, so the error path was the first thing I benchmarked. To my surprise, `serde_json`'s error path was more than 2x slower than the success path on the same data:

<table>
    <thead>
        <tr>
            <td rowspan="2">Speed (MB/s, higher is better)</td>
            <th colspan="2"><code>canada</code></th>
            <th colspan="2"><code>citm_catalog</code></th>
            <th colspan="2"><code>twitter</code></th>
        </tr>
        <tr>
            <th>DOM</th>
            <th>struct</th>
            <th>DOM</th>
            <th>struct</th>
            <th>DOM</th>
            <th>struct</th>
        </tr>
    </thead>
    <tbody>
        <tr>
            <td>Success path</td>
            <td align="center">283</td>
            <td align="center">416</td>
            <td align="center">429</td>
            <td align="center">864</td>
            <td align="center">275</td>
            <td align="center">541</td>
        </tr>
        <tr>
            <td>Error path</td>
            <td align="center">122</td>
            <td align="center">168</td>
            <td align="center">135</td>
            <td align="center">195</td>
            <td align="center">142</td>
            <td align="center">226</td>
        </tr>
        <tr>
            <td>Slowdown</td>
            <td align="center">-57%</td>
            <td align="center">-60%</td>
            <td align="center">-69%</td>
            <td align="center">-77%</td>
            <td align="center">-48%</td>
            <td align="center">-58%</td>
        </tr>
    </tbody>
</table>

Why? Error propagation cannot be that slow. Profiling via `perf` reveals that the bottleneck is this innocent function:

```rust
fn position_of_index(&self, i: usize) -> Position {
    let mut position = Position { line: 1, column: 0 };
    for ch in &self.slice[..i] {
        match *ch {
            b'\n' => {
                position.line += 1;
                position.column = 0;
            }
            _ => {
                position.column += 1;
            }
        }
    }
    position
}
```

...which is called from `position()` to format the error, which is documented as:

```rust
/// Position of the most recent call to next().
///
/// ...
///
/// Only called in case of an error, so performance is not important.
```

...Well. I agree that between a faster success path and a faster error path, the former wins, but taking more time than just parsing just to format an error is taking it way too far.

Can we do anything about this? `position_of_index()` wants to convert an index in a string to a line/column pair. To do that, we can reduce the problem to two simpler ones:

- Count `\n`s in `self.slice[..i]`; that's going to be the 0-based line number, and
- Find the last `\n` in `self.slice[..i]` and subtract its position from `i`; that's going to be the 1-based column number.

Searching a string for a single-character needle is a long-solved problem. In C, we use `strchr` for that; in Rust, we use the [memchr](https://crates.io/crates/memchr) crate. In fact, this crate also provides [an optimized way](https://docs.rs/memchr/2.7.4/src/memchr/memchr.rs.html#327-333) to *count* occurences, which we need for the first subproblem.

`memchr` uses SIMD in both cases, so it's a lot faster than a naive loop. Indeed, replacing the implementation above with:

```rust
fn position_of_index(&self, i: usize) -> Position {
    let start_of_line = match memchr::memrchr(b'\n', &self.slice[..i]) {
        Some(position) => position + 1,
        None => 0,
    };
    Position {
        line: 1 + memchr::memchr_iter(b'\n', &self.slice[..start_of_line]).count(),
        column: i - start_of_line,
    }
}
```

...results in a great improvement:

<table>
    <thead>
        <tr>
            <td rowspan="2">Speed (MB/s, higher is better)</td>
            <th colspan="2"><code>canada</code></th>
            <th colspan="2"><code>citm_catalog</code></th>
            <th colspan="2"><code>twitter</code></th>
        </tr>
        <tr>
            <th>DOM</th>
            <th>struct</th>
            <th>DOM</th>
            <th>struct</th>
            <th>DOM</th>
            <th>struct</th>
        </tr>
    </thead>
    <tbody>
        <tr>
            <td>Success path</td>
            <td align="center">283</td>
            <td align="center">416</td>
            <td align="center">429</td>
            <td align="center">864</td>
            <td align="center">275</td>
            <td align="center">541</td>
        </tr>
        <tr>
            <td>Error path (<code>memchr</code>)</td>
            <td align="center">216</td>
            <td align="center">376</td>
            <td align="center">238</td>
            <td align="center">736</td>
            <td align="center">210</td>
            <td align="center">492</td>
        </tr>
        <tr>
            <td>Slowdown</td>
            <td align="center">-24%</td>
            <td align="center">-10%</td>
            <td align="center">-45%</td>
            <td align="center">-15%</td>
            <td align="center">-24%</td>
            <td align="center">-9%</td>
        </tr>
    </tbody>
</table>

The error path is still slower than the success path, but the difference is a lot less prominent now.

I submitted [a PR introducing this optimization](https://github.com/serde-rs/json/pull/1160) and wondered if it's going to be merged. After all, `serde_json` has very few dependencies, and dtolnay seems to focus on build times, so would a PR adding a new dependency make it?

To my shock, the PR was quickly merged! Not a bad first contribution.


### What next?

dtolnay advised me to look for other places where a similar optimization could be applied, so that's what I did. (I can't overestimate how helpful he was to me during this endeavor.)

The first place I found is this loop in string parsing:

```rust
while self.index < self.slice.len() && !ESCAPE[self.slice[self.index] as usize] {
    self.index += 1;
}
```

What we want here is to find the first non-escape character. "Escape" characters are `\` (for obvious reasons) and `"` (because it marks the end of the string), but also all ASCII codes up to and including `0x1F`, because the JSON specification [forbids](https://www.crockford.com/mckeeman.html) control codes in strings (so e.g. `"line 1\nline 2"` is valid JSON, but replacing the `\n` with a literal newline invalidates it).

*If* all I needed was to find the first `\` or `"`, the [memchr2](https://docs.rs/memchr/latest/memchr/fn.memchr2.html) function provided by `memchr` would suffice. But I need something more complicated, so how am I supposed to go about it?


## Looking for escape

### First try

The first idea dtolnay and I had wasn't a good one, and it touches a tangential topic, but I still think it's important to discuss to learn how to not make the same mistake.

The idea was:

- Use `memchr2` to find the first `\` or `"`, and after that
- Go through the string character-by-character to ensure there are no control characters.

The idea was that offloading the search for `\` and `"` to a faster algorithm would improve the performance overall.

In reality, this turned out to be *slower* than the original code, because looping over the string *twice*, quickly and then slowly, is always bound to be worse than looping over the string *once*, just as slowly. Sure, a byte comparison (`ch < 0x20`) is a little bit faster than a memory access (`ESCAPE[...]`), but that effect is quickly offset by using two passes (and thus increasing memory bandwidth) instead of one.

It turns out that dtolnay based his intuition on [a post](https://nrk.neocities.org/articles/cpu-vs-common-sense) that studied various implementations of the standard C `strlcpy` function and found that a two-pass algorithm is faster than a single-pass algorithm. So what went wrong there?

`strlcpy(char *dst, const char *src, size_t size)` copies a string from `src` to `dst`, truncating it to at most `size - 1` characters. The 1 byte is reserved for the always-added null terminator. The competing implementations were (adding the terminating NUL byte is irrelevant, so not listed here):

- Single-pass: perform `*dst++ = *src++` at most `size - 1` times, until `*src` is a NUL byte, and
- Two-pass: compute `len = strlen(src)`, then call `memcpy(dst, src, min(len, size - 1))`.

The two-pass algorithm was faster because `strlen` and `memcpy` were calls to SIMD-optimized glibc routines, but the loop of the single-pass algorithm was scalar. The author realized this and provided their own implementations of `strlen` and `memcpy`, pessimizing the two-pass `strlcpy`, so that the two algorithms were more competitive:

```c
size_t bespoke_strlcpy(char *dst, const char *src, size_t size) {
    size_t len = 0;
    for (; src[len] != '\0'; ++len) {} // strlen()

    if (size > 0) {
        size_t to_copy = len < size ? len : size - 1;
        for (size_t i = 0; i < to_copy; ++i) // memcpy()
            dst[i] = src[i];
        dst[to_copy] = '\0';
    }
    return len;
}
```

GCC can easily detect such loops and replace them with glibc calls, so the author also explicitly disabled this with `-fno-builtin`. Even like this, the two-pass algorithm was still faster than a single-pass one.

However, one detail wasn't explicit. `-fno-builtin` does not disable *all* `memcpy`-related optimizations: the `memcpy`-like loop can still be vectorized, and that's what GCC did to `bespoke_strlcpy`. So the author was actually comparing scalar `strlen` (check for NUL, loop) + vectorized `memcpy` (check for size, loop) to scalar `strlcpy` (check for NUL, check for size, loop).

Disabling vectorization with `-fno-tree-vectorize` makes the two-pass algorithm *slower*, as it should be, because now we're comparing two loops (check for NUL, loop; check for size, loop) to one loop (check for NUL, check for size, loop), and the latter is faster because it puts less pressure on the branch predictor and has fewer memory accesses.

---

The lesson here is that vectorization is king, but vectorizing the simpler half of the code while the leaving the complex one scalar is not going to provide any improvements. So if we want to optimize this, we have to use SIMD for both parts.


### Second try

The original approach thus morphed into:

- Use `memchr2` to find the first `\` or `"`, and after that
- Use hand-written SIMD to ensure there are no control characters.

We would need to reinvent the bicycle, but this is quite neat if you think about it. In the success path, we *find positions* of `\` and `"`, but we only *check the absence* of control codes. So we can avoid a conditional branch in the hot loop, by replacing this:

```python
for simd_word in to_simd_words(data):
    if any(simd_word < 0x20):
        ...
```

...with this:

```python
mask = False
for simd_word in to_simd_words(data):
    mask |= simd_word < 0x20
if any(mask):
    ...
```

However, we quickly realized that this was a losing battle. We would slow down short strings by invoking a (runtime-selected!) function to search for `\` and `"`, but we would pessimize longer strings by reading memory twice *too*. We needed something better.


### Accepting fate

We really needed to search for `\`, `"`, *and* control codes in one pass.

But I tried hard to keep `serde_json` as simple as it was. I don't usually care about code complexity in my projects, but something another person has to maintain should preferably be as uninvasive as possible. This made separately implementing SIMD for different platforms a no-go.


### Mycroft's trick

However, there is one technique people used before common processors supported SIMD natively. Instead of processing 128-bit words, we could process 64-bit words, using bitwise operations to simulate per-element behavior. This idea is called SIMD Within A Register, or SWAR for short. To give a simple example, converting 8 Latin letters stored in a 64-bit word to lowercase is as easy as computing `x | 0x2020202020202020`.

What I wanted to do is search for a control character in an 64-bit word, implicitly split into 8 bytes.

The way this works is that for `c: i8`, the condition we're looking for is `c >= 0 && c < 0x20`, which can be rewritten as `c >= 0 && c - 0x20 < 0`. This just checks that the sign bit of `c` is `0` and the sign bit of `c - 0x20` is `1`, which is equivalent to `!c & (c - 0x20) & 0x80 != 0`.

So for 8 packed bytes, we compute `!c & (c - 0x2020202020202020) & 0x8080808080808080`. If it's `0`, great, no control character. If it's non-zero, we find the least significant non-zero byte in the mask, and that's our first occurence of a control character.

There's just one nuance. The `c - 0x20` in `c >= 0 && c - 0x20 < 0` is a wrapping subtraction, but performing a 64-bit subtraction can propagate carry/borrow between bytes. This is, however, not a problem: the borrow can only be propagated from a byte if it's less than `0x20`, and only to more significant bytes. We only wish to find the least significant control byte, so we don't care if it corrupts more significant bytes.

This, of course, only works on little-endian machines. On big-endian machines, `c` has to be bytereversed.

What about matching `\` (and `"`) though? The condition for `\` is as simple as `c ^ b'\\' >= 0 && c ^ b'\\' < 1`; this is just the formula above with `0x20` replaced with `0x01`. [The cherry on top](https://github.com/serde-rs/json/pull/1161#discussion_r1713040513) is that `b'\\'` doesn't have the sign bit set, so `c ^ b'\\' >= 0` is equivalent to `c >= 0`.

All in all, the formula simplifies to:

```rust
!c
& (
    (c - 0x2020202020202020)
    | ((c ^ b'\\') - 0x0101010101010101)
    | ((c ^ b'"') - 0x0101010101010101)
)
& 0x8080808080808080
```

This is `9` bitwise operations (counting `a & !b` as one instead of two). For comparison, this would require `7` SIMD operations on x86, so that's quite close to what we'd get from SIMD, just with 2x or 4x smaller throughput, depending on whether AVX is available.

But for short strings, throughput doesn't matter. Latency does. Maybe I made a mistake while trying out different variations, but this SWAR code was more efficient than "real" SIMD code on [json-benchmark](https://github.com/serde-rs/json-benchmark), probably because of this effect. Whatever the reason, this is the code we settled on eventually.


### The other end

Whenever you optimize something by unrolling a loop or using SIMD, a good question to ponder is whether the array is long enough to profit from this. For example, using SIMD to find the length of a 0-16 byte string in a branchless way is neat, but can easily lose to the simplest `strlen` implementation if the strings are usually just 3 bytes long.

Something similar happened here. For strings of around 5 characters, the SWAR approach became slower than scalar code. We decided that regressing such very short strings is a worthwhile investment if we get faster code in other cases.

However, there is one very common short string -- the empty string `""`. Also, due to a technicality, a similar regression applied to strings with consecutive escapes, e.g. `\r\n` or `\uD801\uDC37`, which is a really common sight in Unicode-ridden data. We certainly don't want to regress that. The fix is simple: just check if the very first character is an escape before entering the SWAR loop.

All in all, the improvements we got are this:

<table>
    <thead>
        <tr>
            <td rowspan="2">Speed (MB/s, higher is better)</td>
            <th colspan="2"><code>canada</code></th>
            <th colspan="2"><code>citm_catalog</code></th>
            <th colspan="2"><code>twitter</code></th>
        </tr>
        <tr>
            <th>DOM</th>
            <th>struct</th>
            <th>DOM</th>
            <th>struct</th>
            <th>DOM</th>
            <th>struct</th>
        </tr>
    </thead>
    <tbody>
        <tr>
            <td>Scalar</td>
            <td align="center">291</td>
            <td align="center">442</td>
            <td align="center">377</td>
            <td align="center">865</td>
            <td align="center">305</td>
            <td align="center">638</td>
        </tr>
        <tr>
            <td>Vectorized</td>
            <td align="center">292</td>
            <td align="center">442</td>
            <td align="center">367</td>
            <td align="center">905</td>
            <td align="center">335</td>
            <td align="center">785</td>
        </tr>
        <tr>
            <td>Speedup</td>
            <td align="center">0%</td>
            <td align="center">0%</td>
            <td align="center">-3%</td>
            <td align="center">+5%</td>
            <td align="center">+10%</td>
            <td align="center">+23%</td>
        </tr>
    </tbody>
</table>

`citm_catalog DOM` is quite flickery, so in the end there aren't even regressions on `json-benchmark`. There is one other regression though: empty strings still take a bit longer to parse, but the slowdown is luckily within 2% on a very specific microbenchmark.


## When lexing becomes complicated

### Unicode

What else about Unicode, by the way? `serde_json` can parse Unicode in both decoded and encoded formats, e.g. `"🥺"` and `"\ud83e\udd7a"`. While raw Unicode is trivial to parse, decoding `\u` escapes is a more complicated topic.

Can't you just parse four hex digits and that's it? Well, sort of. Number parsing is really hard, and it might surprise you how generic and complex some algorithms are.

Parsing a hex digit requires mapping disjoint intervals `'0'..='9'`, `'A'..='F'`, `'a'..='f'` to `0..16`. You could use conditionals for that:

```rust
match c {
    b'0'..=b'9' => c - b'0',
    b'A'..=b'F' => c - b'A' + 10,
    b'a'..=b'f' => c - b'a' + 10,
    _ => return Err(..),
}
```

...or branchless algorithms, which the Rust standard library does.

But nothing beats a LUT.

```rust
static HEX: [u8; 256] = {
    const __: u8 = 255; // not a hex digit
    [
        //   1   2   3   4   5   6   7   8   9   A   B   C   D   E   F
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 0
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 1
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 2
        00, 01, 02, 03, 04, 05, 06, 07, 08, 09, __, __, __, __, __, __, // 3
        __, 10, 11, 12, 13, 14, 15, __, __, __, __, __, __, __, __, __, // 4
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 5
        __, 10, 11, 12, 13, 14, 15, __, __, __, __, __, __, __, __, __, // 6
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 7
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 8
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 9
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // A
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // B
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // C
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // D
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // E
        __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // F
    ]
};

fn decode_hex_val(val: u8) -> Option<u16> {
    let n = HEX[val as usize] as u16;
    if n == 255 {
        None
    } else {
        Some(n)
    }
}
```

This is what `serde_json` used to utilize, and it actually worked pretty well (better than `std`, anyway: `std` has to be generic over radix, `serde_json` doesn't have to). This function would then be used like this:

```rust
let mut n = 0;
for _ in 0..4 {
    n = (n << 4) + decode_hex_val(self.slice[self.index])?;
    self.index += 1;
}
```

That's at least 3 `shl`s and 3 `add`s, with quite a few `mov`s, `cmp`s with 255, and conditional jumps inbetween. We can do better.

Let's start by removing the `?` on each iteration. That's quite simple. Instead of storing `HEX` as a `[u8; 256]` array, we can store it as a `[u32; 256]` array, mapping `__` to `u32::MAX`. No valid digit has the high 16 bits set, so we can easily figure out if some digit was invalid *after* the loop:

```rust
let mut n = 0;
for _ in 0..4 {
    n = (n << 4) + HEX[self.slice[self.index] as usize];
    self.index += 1;
}
ensure!(n >= 65536, "Invalid Unicode escape");
let n = n as u16;
```


### Punching out

Saving memory (and thus cache!) by using `u16` instead of `u32` looks impossible, because a `u16::MAX = 0xFFFF` in the leading digit would quickly get shifted to `0xFxxx` in `n`, and at that point you can't disambiguate between a valid codepoint and an invalid digit.

Or is it? Here's a trick [Yuki](https://github.com/yuki0iq/) invented. We can map `__` to `u16::MAX`, but also replace `n << 4` with `n.rotate_left(4)` and addition with bitwise OR:

```rust
let mut n = 0;
for _ in 0..4 {
    n = n.rotate_left(4) | HEX[self.slice[self.index] as usize];
    self.index += 1;
}
```

If all hex digits are valid, nothing's changed, `n` is still our codepoint. Rotation is exactly as efficient as shifts on x86, so no issues performance-wise either. But if some hex digit is invalid, it's going to "infect" `n`, setting it to `0xFFFF`, and the next iterations *will keep yielding `0xFFFF`*. Unicode defines `U+FFFF` as a codepoint that does not signify a character, meaning that it's extremely unlikely to be used in realistic data, so we can just branch on `n == 0xFFFF` afterwards and re-check if we should emit an error or the JSON genuinely contained a `\uFFFF`. Isn't that neat?


### clueless.jpg

Just as I was writing this post I [realized](https://github.com/serde-rs/json/pull/1178) that this is a classical case of overengineering. The `0xFFFF` only gets shifted out if we compute the codepoint in 16-bit arithmetic. But we aren't in the stone age; we have 32-bit integers! Let's store `HEX` as `[i8; 256]`, with `-1` stands for an invalid digit. Then

```rust
let mut n = 0;
for _ in 0..4 {
    n = (n << 4) | HEX[self.slice[self.index] as usize] as i32;
    self.index += 1;
}
```

...will produce a non-negative number on success and a negative number on failure. The seemingly operational `as i32` turns out to be a no-op because x86 fuses a memory load and a sign extension into one `movsx` instruction.

What I like about signed numbers is that most processors have a single instruction to branch on sign. Instead of `cmp r, imm; je label`, we can just do `js label` in most cases. This does not usually affect performance on modern CPUs, but hey, at least it looks prettier.


### Shifts

Shifts increase latency. Latency bad. Alisa want no latency.

Luckily, this is easy to fix by introducing two tables instead of one: `HEX0`, which is `HEX` cast to `[i16; 256]`, and `HEX1`, which is `HEX` cast to `[i16; 256]` but also left-shifted by `4`.  This allows the loop to be unrolled very clearly and is the final hex-decoding implementation.

```rust
fn decode_four_hex_digits(a: u8, b: u8, c: u8, d: u8) -> Option<u16> {
    let a = HEX1[a as usize] as i32;
    let b = HEX0[b as usize] as i32;
    let c = HEX1[c as usize] as i32;
    let d = HEX0[d as usize] as i32;

    let codepoint = ((a | b) << 8) | c | d;

    // A single sign bit check.
    if codepoint >= 0 {
        Some(codepoint as u16)
    } else {
        None
    }
}
```

Overall, this increased the performance of parsing JSON-encoded *War and Peace* in Russian from 284 MB/s to 344 MB/s, resulting in a 21% improvement.


## Transcoding

### Hazards

After the last optimization, the bottleneck of Unicode string parsing shifted to UTF-8 encoding.

This is funny, because UTF-8 is supposed to be really simple. To give a quick reminder, UTF-8 encodes codepoints in one of the following ways:

- 1 byte: `0xxxxxxx`
- 2 bytes: `110xxxxx 10xxxxxx`
- 3 bytes: `1110xxxx 10xxxxxx 10xxxxxx`
- 4 bytes: `11110xxx 10xxxxxx 10xxxxxx 10xxxxxx`

The `x`s signify the bits of a codepoint; the shortest representation long enough to fit all the bits is used. All codepoints fit in 21 bits.

Rust's standard library implements UTF-8 encoding by providing, among other functions, `char::encode_utf8` to store the char encoding to a buffer. There is, however, one minor inconvenience. The signature of the method is

```rust
fn encode_utf8(self, dst: &mut [u8]) -> &mut str;
```

...which means that it writes to a buffer that already stores *valid* `u8`s. You can't just create an *uninitialized* buffer and put UTF-8 there; you need to initialize (e.g. zero-initialize) it in advance.

The assumption here is that the optimizer is smart enough to optimize out zeroization. This *would* be true in other cases, but UTF-8 is a *variable-length* encoding. If you zeroed more bytes than `encode_utf8` would place, zeroization would be wrong to optimize out. So you need to zeroize a variable amount of bytes. But the semantics become too complicated to capture accurately for LLVM here, so it just drops the ball.

---

So `serde_json` used another approach:

```rust
scratch.extend_from_slice(c.encode_utf8(&mut [0u8; 4]).as_bytes());
```

`[0u8; 4]` is a local variable, so zeroing more bytes than necessary shouldn't be a problem because aliasing analysis should help with this. Which is kind of true in theory.

In practice, something horrendous happens instead. Remember how LLVM drops the ball on variable-length zeroization? Well, it drops the ball on a variable-length copy too. `Vec::extend_from_slice` needs to copy 1 to 4 bytes from the local buffer to heap, so LLVM invokes glibc's `memcpy` to do that. Wonderful.

---

The best way to avoid calls to `memset` and `memcpy` turned out to be generating UTF-8 manually. This is trivial algorithm-wise, but requires unsafety, so I was initially somewhat scared of that, but I had to submit.

Together with a few other minor modifications, this further increased performance on *War and Peace* to 374 MB/s (+9%).


## Final results

### 🥺

All in all, my work improved `serde_json` performance on various string-heavy JSON benchmarks by 10%, 23%, and 32%. A lot of JSON data contains many strings, so I believe that this will benefit the ecosystem in the long run.
