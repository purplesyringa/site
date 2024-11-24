---
title: "h: Practical, efficient perfect hash functions"
time: November 25, 2024
draft: true
intro: |
    I need a hash-table with integer keys. `HashMap<u32, T>`, right? Wrong. *For plot reasons*, I need it to perform at ridiculous speeds. So that's how the journey towards the fastest perfect hash function started.
---

Hash tables are frequently used in programming. Often, they are mostly static and only require read-only access after building. At this point, we encounter a tradeoff between build time and access time.

[For plot reasons](http://127.0.0.1:8000/blog/you-might-want-to-use-panics-for-error-handling/), I need a static hash table with integer keys with blazingly fast access times. `HashMap<u32, T>` is quite slow, and, surprisingly, so is [phf](https://docs.rs/phf/latest/phf/) -- the default solution for this problem in the Rust ecosystem.

I tried to replace the hash functions in the official benchmarks with no-ops, and still got $5$ ns per access -- more than I'm willing to pay. In addition, the build time for `phf` sucks. Sure, I can spend some time preprocessing the table, but I'd like it to not take much more than a second to process a few million keys.


### h

:::aside
I'm notorious for developing projects so that I can get a claim to their name rather than to make something good; that's just a common side effect.

`h` stands for hydrogen, because `h` is supposed to be as lightweight as reasonably possible on modern architectures. If you need a longer name, use `hPHF`, where `h` can either stand for hydrogen or "hybrid", as the math behind `h` is based on taking the best from several known approaches to building PHFs.
:::

So I made [a better perfect hash: h](https://lib.rs/h).

<aside-inline-here />

...


### Comparison

In my opinion, `h` is quite competitive. It's not very mathematically rigorous at the moment, although I believe this can be fixed with enough effort.

The `phf` crate takes $4.3$ seconds to build a PHF with $10^6$ integer $32$-bit keys. (That's with SipHash, which I couldn't avoid because any faster lazy hashes I tried to throw at `phf` lead to hangs and infinite loops.) In comparison, `h` does that in $140$ ms -- $30 \times$ faster. (It scales worse than linearly for larger sizes due to cache effects, but still reasonably well.)

`phf` takes $14.5$ ns to resolve an element in a $26$-element hash map keyed by random $32$-bit integers generated with a proc macro. If you hack the code and replace the hash with the fastest one that works, the time falls down to $3.9$ ns. In contrast, `h` takes $2.3$ ns to resolve it with the map *generated in runtime*, i.e. without constant propagation, and $2.0$ ns with compile time generation.

Unlike `phf` and `boomphf`, `h` is not a *minimal* PHF, but *a* PHF. This means that `h` wastes some memory in the hash table by design. In return, `h` takes less space for control information, so the overall memory use stays about the same for small objects and large datasets.

For the same reason, `h` is significantly less picky about the quality of the underlying imperfect hash than `phf`, which requires SipHash. This enables faster hashes like `wyhash` to be used.

Much like `phf`, the memory utilization is spread between two arrays, storing data and control information separately. The sizes of the arrays are, however, much more skewed than in `phf`, leading to better performance when only the smaller control array fits in cache.

`boomphf` builds hash functions $2$ -- $3 \times$ faster than `h`, but its access time stays at around $40$ ns per key regardless of the hash function. This can be easily explained: `boomphf` has variable-time access performance and requires quite a bit of pointer jumping. In contrast, `h` supports constant-time accesses and performs just one memory read apart from the accessed entry.

On large data, `h` takes about $5 \times$ more time to build than `HashMap`.


### Scope

`h` does not support all key types, and manually implementing a custom `PortableHash` trait (similar to `Hash` from std) might be necessary.

Although `h` was originally tuned to small integer keys, it provides good performance with any keys. Even if the access time turns out to be similar to alternatives, `h` takes relatively little to build. This matters for hashing large data, programmatically generated data, and building in runtime.

Finally, `h` provides constant access time, which is important for real-time systems and predictable performance.


## Theory

### Intro

If you're interested, .

> I'm experimenting with writing styles. This is not as formal as I'd like it to be (I'm hoping to write a paper on this topic eventually), but I wanted this to be an introduction to PHF construction too. We'll see how it goes.


### PHFs

*Perfect hash functions* are hash functions trained on a particular key set, with a lucky property that the hashes of these keys don't collide.

For example, for the key set $\{12534, 12421, 123567\}$, $H(x) = x \& 3$ is a valid PHF, because the hashes of the set are $\{3, 2, 1\}$ with no collisions. Note that collisions may arise if *other* unlisted keys are hashed: this is not considered a problem.

After the PHF is built, we can just put the metadata into an array like this:

```rust
let hashtable = [
    None,
    Some((123567, value for 123567)),
    Some((12421, value for 12421)),
    Some((12534, value for 12534)),
];
```

...and access it like this:

```rust
fn get(key: u32) -> Option<Value> {
    let (stored_key, value) = hashtable[hash(key)]?;
    // If the keys match, this key was present in the original keyset and we can return the value.
    // If the keys don't match, this is an unlisted key and we can return None.
    (key == stored_key).then_some(value)
}
```


### Spooky math

The hash function clearly can't be perfect for *all* keys, so it should depend on the key space. There's many different key spaces, so there must be many different PHFs, and that means they have to store *data* that somehow corresponds to the key space. So a PHF is not just *code*, but also a set of *lookup-up tables*.

Most constant-time PHFs (and we do need constant time, because any unpredictable conditional jumps are fatal at this scale) use the following approach. Firstly, two *imperfect* random hash functions are chosen: $\mathit{Approx}$ and $\mathit{Bucket}$. $\mathit{Approx}$ maps the key to its approximate hash (i.e. position in the hash table), while $\mathit{Bucket}$ maps the key to its bucket ID. For each bucket, a *displacement* value is stored in the look-up table, which is mixed with the approximate hash to obtain the final hash.

The idea is that while $\mathit{Approx}$ hashes might collide, as long as the colliding keys are from different buckets, we can choose different displacements to avoid the collision. If a particular $(\mathit{Approx}, \mathit{Bucket})$ combination still leads to collisions, we just choose another pair of imperfect hash functions and try again.

We control (and need to choose):

- The size of the hash space $M$
- The number of buckets $B$
- How the displacement is mixed into the hash
- The displacement range
- $\mathit{Approx}$ and $\mathit{Bucket}$

Off we go.


### Hash space

We could round the number of keys to the next power of two and use that as the hash space size. This simplifies calculations, because taking a number modulo $2^n$ is trivial. However, this wastes a lot of space in the hashtable. In my use case, the metadata is quite large, and storing a `Box<Metadata>` is not an option, because indirections are slow.

Alternatively, we could equate the hash space size to the key space size. This is called a *minimal* PHF. Obviously, the probability of collision increases significantly as the number of free locations decreases, so building an mPHF is very slow compared to a non-minimal PHF.

:::aside
PTHash then transforms this almost-mPHF to an mPHF with a separate look-up table. That introduces a conditional branch and an indirection, which would leads to a major performance loss, so we have to bite the bullet and pay the $1\%$ space increase.
:::

Finally, we could choose an in-between option. The authors of [PTHash](https://arxiv.org/pdf/2104.10402) have found that using a $0.99$ load factor (i.e. increasing the hash space size by $1\%$) significantly improves build time performance without increasing the memory use much, so that's what we're going to use.

<aside-inline-here />


### Bucket count

The number of buckets is a balance between memory consumption and build time. With $B = N$, the behavior is similar to cuckoo hashing, with all the space utilization problems. With $B = 1$, the PHF degenerates to "use a random imperfect hash and hope for the best".

This question is also tightly connected to the choice of $\mathit{Bucket}$ (which maps the key to its bucket). There are two common choices:

- [CHD](https://cmph.sourceforge.net/papers/esa09.pdf): Choose a *uniform* $\mathit{Bucket}$, with $5$ keys per bucket on average ($B = \frac15 N$)
- [FCH](https://cmph.sourceforge.net/papers/fch92.pdf): Choose a *significantly non-uniform* $\mathit{Bucket}$, with $B = \frac{6N}{\log_2 N}$.

Using a non-uniform $\mathit{Bucket}$ sounds weird, but it's actually intuitive when you think about it. To generate the bucket displacements, we're going to process them in order. At the beginning of this process, the load factor is low, so almost every displacement works, but then it gets harder and harder. It is thus better to start with large buckets and work our way down to smaller buckets. Non-uniformity helps use this relation to reduce the number of buckets without sacrificing build time.

However, non-uniformity is hard to come by in practice. Usually, it's achieved by computing a uniform hash and then skewing it artificially. This scaling takes time, which we have little of, so we'll have to use the CHD approach.


### Mix function

There are many approaches to displacing the hash. As the approximate hash is uniformly random, the displacement itself does not necessarily have to be random. This means that we can just try adding $0, 1, 2, \dots$ to the approximate hash, or maybe XOR it in.

While $+$ and $\oplus$ (i.e. XOR) seem similar, they actually differ in several characteristics:

- Space utilization (better for $\oplus$)
- Build success probability (better for $+$ for small load factors or $\oplus$ for large load factors)
- Build time (better for $+$)

For approximate hash $h$ and displacement $d$, we can either have $h \oplus d$ or $h + d$ as the final hash, and the former is clearly less in the general case. For large $h$ and small $d$, this is not a big problem, but for smaller hash tables, using $\oplus$ reduces the hash table size.

The question of success rate is tangentially related to collision rate in open addressing vs closed addressing in hash tables. With $+$, buckets effectively partially overlap, just like in open addressing, while with $\oplus$, they either match completely or don't intersect at all, just like in closed addressing. In typical hash tables, we're only interested in the *expected* collisions, while in PHFs, any collisions that can't be resolved by displacements are fatal. It's a well-known fact (I hope so, anyway; I don't want to burden you with a proof here) that open addressing is more lax in this regard.

As for the build time, we'll discover why the mix function matters later.


### Displacements

How many different displacements do we need to try, i.e. what is the largest $d$ we can expect, and what datatype should we use? There's quite a bit of math here; feel free to skip it if that's not your kind of thing.

For bucket of size $b$ and current load factor is $\alpha$, we can expect a success rate of $(1 - \alpha)^b$ per displacement. Bucket sizes are distributed according to the Poisson distribution, so $\alpha \approx 1.01 - Q(b, 5)$, thus the success rate is $(Q(b, 5) + 0.01)^b$. This peaks at around $1 / 400$, indicating an expected number of $400$ displacements until success. This is the result for *one* bucket; the probability for *all* bucket displacements to be small is lower. In practice, I have found that using $16$-bit displacements works for sizes up to several million with high probability, which suffices for my usecase.


## Implementation

### Hash functions

Now we need to design $\mathit{Approx}$ and $\mathit{Bucket}$, the functions for mapping keys to approximate hashes and for mapping keys to buckets.

Assume that we already have a fast $64$-bit hash function $\mathit{Uniform}$ with much entropy in the top bits. This is the only place that needs to be done separately for different kinds of data. I'll omit this section, as it's irrelevant to the PHF itself.

Now here's a problem. Say we've got a "uniform" $64$-bit number, but $\mathit{Approx}$ needs to output a number between $0$ and $M = 1.01 N$, which isn't necessarily a power of two, so we'll need to perform another calculation to scale the uniform hash. Scale, you say? That's just fixed-point multiplication: $(h \times M) >> 64$ maps $h \in [0; 2^{64})$ to $[0; M)$, which is precisely what we need.

What about $\mathit{Bucket}$? I'd rather avoid yet another multiplication, so we'll have to bite the bullet and round the bucket count to a power of two (buckets take little space per key, so that's okay). We can then use the *lower* half of the $64 \times 64$ multiplication, shifted to the right, as a bucket index.

On x86-64, multiplication takes 3 ticks to produce the low half, and 4 ticks to produce the high half. In parallel, a 1-tick shift can be performed to get the bucket index. So that's, uh, a good hash with a 4 tick latency:

```rust
struct Precomputed {
    hash_space: u64,
    bucket_shift: u64,
}

struct Hash {
    approx: u64,
    bucket: u64,
}

fn hash(h: u64, precomputed: Precomputed) -> Hash {
    let product = h as u128 * precomputed.hash_space as u128; // 3 ticks
    Hash {
        approx: (product >> 64) as u64, // 1 tick
        bucket: (product as u64) >> precomputed.bucket_shift, // 1 tick (computed concurrently)
    }
}
```

Nevermind the `mov`s; these will go away after inlining:

```x86asm
hash:
    mov     rcx, rdx
    mov     rax, rsi
    mul     rdi
    mov     rsi, rax
    shr     rsi, cl
    mov     rax, rdx
    mov     rdx, rsi
    ret
```

Now all we have to do is compute the final hash with $\mathrm{mix}(\mathit{Approx}, \mathit{Disp}[\mathit{Bucket}])$.


## Generation

### The what?

Okay, now how do we *generate* the table in the first place? We need to decide on:

- `hash_space`
- `bucket_shift`
- The displacement LUT

None of these are as simple as they look like.


### Hash space

This looks trivial: just round $1.01 n$ up. But there's three pitfalls here.

The easier one is that multiplying by an even number shifts entropy to the left, so there might not be enough entropy left for $\mathit{Bucket}$. Thus `hash_space` needs to be odd.

The medium one is that due to a limited displacement range, $1.01 n$ might not be enough. This effect is exacerbated by large table sizes. Therefore, we try different `hash_space` values out: $1.01 n, 1.02 n, 1.03 n, \dots$, and increase the starting value based on the the number of elements. If we reach a power-of-two `hash_space` value, we're in luck: although we're now wasting memory, multiplication by `hash_space` has just become a shift, reducing latency by 2 ticks; this in turn allows us to try better $\mathit{Uniform}$ functions.

The harder one is that while $\mathit{Approx}$ can be in-bounds, $\mathrm{mix}(\mathit{Approx}, \mathit{Disp}[\mathit{Bucket}])$ can be out-of-bounds. For example, for hash table size $5$, a key with approximate hash $4$ and displacement $2$ has final index $6$ (for both $+$ and $\oplus$ mixing functions). Handling this case as wrap-around is a performance hazard, and forbidding such out-of-bounds accesses decreases the probability of a successful build. The solution is to keep the output space of $\mathit{Approx}$ (i.e. `hash_space`) limited to $1.01 n$ (or another multiple of $n$), but pad the resulting hash table to the right such that all possible accesses are in-bounds.

There's a conundrum here. Do we make *accesses to existing elements* in-bounds, or do we legalize *all accesses*? If accessed keys are guaranteed to be present in the hashmap, the former suffices; if we need to add an existence check (like in the first example in this post), we need to do the latter. My case calls for access verification, so we'll need to make all accesses sound, increasing the hash table size to *[Insert incomprehensible math warning]* $1 + \max\limits_{i=0}^{B-1} \max\limits_{x=0}^{M-1} \mathrm{mix}(x, \mathit{Disp}_i)$.

For the $+$ mixing function, this is just $M + \max\limits_{i=0}^{B-1} \mathit{Disp}_i$. For $\oplus$, a tighter boundary is possible; I'm a nerd, so we're going to solve this Leetcode problem now. Skip to the next section if you despise that sort of thing.

We're looking for a way to calculate $\max\limits_{x=0}^X x \oplus Y$ in $\mathcal{O}(1)$ for different $Y$ values. I won't bore you with details, but it works like this:

```rust
fn max_xor(x: usize, y: usize) -> usize {
    let bound = (x & y).checked_ilog2().unwrap_or(0);
    x | y | ((1 << bound) - 1)
}
```

:::aside
Hint: try to compute the largest $x \oplus Y$ bit by bit, from high to low. If $X$ and $Y$ have different bits at some position, the resulting bit can easily be $1$, which is the largest possible value. However, when both $X$ and $Y$ have a $1$ bit, we can decrease $X$ so that this bit is $0$ and all the following bits are inverses of $Y$.
:::

Proving this is left as an exercise to the reader. I have no intuition for why this function is symmetric; if you figure it out, please let me know. Anyway; this function is monotonous in $X$ (by definition) and symmetric (by implementation), so it's monotonous in $Y$, so instead of substituting several displacements, we can just substitute the largest one, i.e. the hash table size is $1 + \mathrm{max\_xor} \left( M - 1, \max\limits_{i=0}^{B-1} \mathit{Disp}_i \right)$.

<aside-inline-here />


### Bucket shift

I lied, this one's actually simple. A good change of pace. Do you feel relieved? Please feel relieved.

If you still remember, we compute $\mathit{Bucket}$ of a key as `(product as u64) >> bucket_shift`, so `bucket_shift` is just $64 - \log_2 B$, and the bucket count is $\frac15 N$ (shamelessly stolen from CHD), rounded up to a power of two.


### Displacements

Displacements are a core part of PHFs, so this part's more typical. We split keys into buckets according to $\mathit{Bucket}$, sort the buckets in order of decreasing size, and assign displacements to buckets in order. We try displacements $0, 1, 2, \dots, 2^{16} - 1$; the first applicable displacement (s.t. the elements don't collide with already assigned ones) is used. If no displacement works, we restart the algorithm with different hash functions.

The basic idea of what we're trying to do is this:

```rust expansible
struct Phf {
    hash_space: u64,
    bucket_shift: u64,
    displacements: Vec<u16>,
}

fn try_generate_phf(keys: &[u64]) -> Option<Phf> {
    // Hash space. TODO: Increase on failure.
    let hash_space = (keys.len() + keys.len().div_ceil(100)) | 1;

    // Number of buckets
    let b = (keys.len().div_ceil(5) + 1).next_power_of_two();

    // Split keys into buckets
    let bucket_shift = 64 - b.ilog2();
    let mut keys_per_bucket = vec![Vec::new(); b];
    for key in keys {
        // TODO: Apply Uniform to key.
        let bucket = key.wrapping_mul(hash_space as u64) >> bucket_shift;
        keys_per_bucket[bucket as usize].push(*key);
    }

    // Reserve space for elements, plus 2^16 - 1 for out-of-bounds displacements
    let mut free = vec![true; hash_space + u16::MAX as usize];

    // We'll fill this per-bucket array during the course of the algorithm
    let mut displacements = vec![0; b];

    // Handle buckets in order of decreasing size
    let mut bucket_order: Vec<usize> = (0..b).collect();
    bucket_order.sort_unstable_by_key(|bucket| core::cmp::Reverse(keys_per_bucket[*bucket].len()));
    for bucket in bucket_order {
        // Compute all Approx values preemptively
        let mut approx_for_bucket: Vec<u64> = keys_per_bucket[bucket]
            .drain(..)
            .map(|key| ((key as u128 * hash_space as u128) >> 64) as u64)
            .collect();

        // Ensure that Approx values don't collide inside the bucket
        approx_for_bucket.sort_unstable();
        if approx_for_bucket
            .windows(2)
            .any(|window| window[0] == window[1])
        {
            return None;
        }

        // Find non-colliding displacement. On failure, return None.
        let displacement = (0..=u16::MAX).find(|displacement| {
            approx_for_bucket.iter().all(|approx| *unsafe {
                free.get_unchecked(mix(*approx, *displacement) as usize)
            })
        })?;

        // Place the bucket
        displacements[bucket] = displacement;
        for approx in approx_for_bucket {
            *unsafe { free.get_unchecked_mut(mix(approx, displacement) as usize) } = false;
        }
    }

    Some(Phf {
        hash_space: hash_space as u64,
        bucket_shift,
        displacements,
    })
}
```

:::aside
The reason I'm not saying any specific numbers is that the per-key performance depends on how well the data fits in cache. Different sizes lead to different performance characteristics; make sure to benchmark your usecases. During this optimization stage, all I can provide without lying is approximate relative numbers.
:::

This code works, but quite slow. Luckily, there's opportunities for optimization.

<aside-inline-here />


## Faster bucket logic

### Allocations & cache

There's two obvious problems with this code.

One problem is that `keys_per_bucket` is a two-dimensional array containing $B$ vectors. Most of these vectors are non-empty, and calling `malloc` approximately $B$ times is really slow. Even if we reduce this to one allocation, we're still going to jump all over the memory, polluting the cache.

To fix both of these, we need to find another approach. Most cache-oblivious algorithms use the same old trick: sorting data with a cache-friendly algorithm and reading it in order is faster than reading it out of order. So we're going to sort the keys by bucket, `group_by` the result into buckets, and then split this sequence into per-size bucket lists.

```rust expansible
// Sort keys by bucket using a cache-friendly algorithm
let mut sorted_by_bucket = keys.to_vec();
radsort::sort_by_key(&mut sorted_by_bucket, |key| key_to_bucket(*key));

// We'll store per-size bucket lists here
let mut buckets_by_size: Vec<Vec<(u64, usize)>> = Vec::new();

// A manual group_by implementation
let mut left = 0;
while left < keys.len() {
    let bucket = key_to_bucket(sorted_by_bucket[left]);

    let mut right = left;
    let mut product;
    while right < keys.len() && {
        // Keep going while the key has the same bucket as the previous one
        product = sorted_by_bucket[right] as u128 * hash_space as u128;
        bucket == product as u64 >> bucket_shift
    } {
        // Replace the key with its Approx value in-place for future use. We have already computed
        // the product, so this is cheap.
        sorted_by_bucket[right] = (product >> 64) as u64;
        right += 1;
    }
    left = right;

    // Add bucket to its per-size list
    let size = right - left;
    if buckets_by_size.len() <= size {
        buckets_by_size.resize(size + 1, Vec::new());
    }
    buckets_by_size[size].push((bucket, left));
}
```

Iterating over the buckets is then as simple as:

```rust
for (size, buckets) in buckets_by_size.iter().enumerate().rev() {
    for &(bucket, start) in buckets {
        let approx_for_bucket = &mut sorted_by_bucket[start..start + size];
        ...
    }
}
```

This alone increases performance almost twofold. `radsort` might not be the best radix sort implementation, but it's the only one I found that supports stateful `sort_by_key`, so we'll have to live with it.


### Displacements (XOR)

The second problem is this quadratic loop:

```rust
// Find non-colliding displacement. On failure, return None.
let displacement = (0..=u16::MAX).find(|displacement| {
    approx_for_bucket.iter().all(|approx| *unsafe {
        free.get_unchecked(mix(*approx, *displacement) as usize)
    })
})?;
```

The average displacement is quite large, so the outer loop can perform quite a few iterations. Meanwhile, the inner loop iterates through a small subset of indices all across the memory, which doesn't even always fit in L2.

Luckily, this is easy to fix by simultaneously computing the predicate for *multiple* values of `displacement`, as if by unrolling the loop. But hey, $\mathit{Approx} \oplus 0, \dots, \mathit{Approx} \oplus 7$ exactly covers an 8-aligned slice of `free`, which means `free` can cheaply be turned into a bitset!

Well... sort of. It *looks* like it should be as simple as

```rust expansible
// SAFETY: `free` must be a bitset large enough to fit `approx ^ displacement`.
unsafe fn find_valid_displacement(approx_for_bucket: &[u64], free: &[u8]) -> Option<u16> {
    // Outer unrolled loop
    for displacement_base in (0..=u16::MAX).step_by(8) {
        let mut global_bit_mask = u8::MAX;

        // Iterate over keys
        for approx in approx_for_bucket {
            let approx = *approx as usize;
            // Inner unrolled loop, aka bitmask logic
            let bit_mask =
                *unsafe { free.get_unchecked((approx ^ displacement_base as usize) / 8) };
            global_bit_mask &= bit_mask;
        }

        // Find the first applicable displacement (i.e. such that all `free` values are 1)
        if global_bit_mask != 0 {
            let displacement_offset = global_bit_mask.trailing_zeros() as u16;
            return Some(displacement_base + displacement_offset);
        }
    }

    None
}
```

But that's actually wrong. We *want* to SIMDify the computation of

- `free[approx[0] ^ 0] && free[approx[1] ^ 0] && free[approx[2] ^ 0] && ...`
- `free[approx[0] ^ 1] && free[approx[1] ^ 1] && free[approx[2] ^ 1] && ...`
- ...
- `free[approx[0] ^ 7] && free[approx[1] ^ 7] && free[approx[2] ^ 7] && ...`

Which we rewrite as

- `free[approx[0] / 8][(approx[0] & 7) ^ 0] && ...`
- `free[approx[0] / 8][(approx[0] & 7) ^ 1] && ...`
- ...
- `free[approx[0] / 8][(approx[0] & 7) ^ 7] && ...`

But note that the bit index $0$ to $7$ needs to be XORed with `approx`, while in our code, we just directly access bits $0$ to $7$ without consideration for the low three bits of `approx`. If we want this scheme to work, we need to *shuffle bits inside a byte* by XORing *the bit index* (rather than the bits themselves) with `approx & 7`:

- If `approx & 7 == 0`, the shuffle order is $01234567 \to 01234567$.
- If `approx & 7 == 1`, the shuffle order is $01234567 \to 10325476$.
- If `approx & 7 == 2`, the shuffle order is $01234567 \to 23016745$.
- If `approx & 7 == 3`, the shuffle order is $01234567 \to 32107654$.
- ...
- If `approx & 7 == 7`, the shuffle order is $01234567 \to 76543210$.

You get the idea. The most performant way to compute this is with a LUT:

```rust expansible
const BIT_INDEX_XOR_LUT: [[u8; 256]; 8] = {
    let mut lut = [[0; 256]; 8];
    let mut xor = 0;
    // For loops are unsupported in const, smh my head
    while xor < 8 {
        let mut bit_mask = 0;
        while bit_mask < 256 {
            let mut bit_index = 0;
            while bit_index < 8 {
                lut[xor][bit_mask] |= ((bit_mask as u8 >> bit_index) & 1) << (bit_index ^ xor);
                bit_index += 1;
            }
            bit_mask += 1;
        }
        xor += 1;
    }
    lut
};

global_bit_mask &= BIT_INDEX_XOR_LUT[approx % 8][bit_mask as usize];
```

This again brings build time down twofold. Unfortunately, increasing the performance further is a lot harder. Increasing the step to 16 bits would either require a 2 MiB LUT, which is not really a good idea for many reasons, or introduce conditionals into the `global_bit_mask &= ...` calculation -- hardly a positive change.


### Displacements (+)

So that's it for $\oplus$. What about $+$?

We're in luck: bits $\mathit{Approx} + 0$ to $\mathit{Approx} + 7$ can be extracted simply by performing an unaligned 16-bit read, followed by a bit shift. But we don't have to limit ourselves to validating $8$ displacements at once: we can easily validate $57$ displacements at once!

```rust expansible
// SAFETY: `free` must be large enough to fit `approx + displacement + 8`.
unsafe fn find_valid_displacement(approx_for_bucket: &[u64], free: &[u8]) -> Option<u16> {
    // Outer unrolled loop
    for displacement_base_index in 0..u16::MAX / 57 {
        // We don't iterate through a few of the top 65536 displacements, but that's noise
        let displacement_base = displacement_base_index * 57;

        // Can't trust bits farther than the first 57, because we shift out up to 7 bits, shifting
        // in meaningless zeros
        let mut global_bit_mask = (1 << 57) - 1;

        // Iterate over keys
        for approx in approx_for_bucket {
            // Inner unrolled loop, aka bitmask logic
            let start = *approx as usize + displacement_base as usize;
            let bit_mask = unsafe { free.as_ptr().add(start / 8).cast::<u64>().read_unaligned() }
                >> (start % 8);
            global_bit_mask &= bit_mask;
        }

        if global_bit_mask != 0 {
            let displacement_offset = global_bit_mask.trailing_zeros() as u16;
            return Some(displacement_base + displacement_offset);
        }
    }

    None
}
```

This is approximately twice as fast as the XOR logic.
