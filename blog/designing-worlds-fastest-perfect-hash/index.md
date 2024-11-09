---
title: Designing world's fastest perfect hash
time: November 15, 2024
intro: |
  I need a hash-table with integer keys. `HashMap<u32, T>`, right? Wrong. *For plot reasons*, I need it to perform at ridiculous speeds. So that's how the journey towards the fastest perfect hash function started.
---

I need a hash-table with integer keys. `HashMap<u32, T>`, right? Wrong. *For plot reasons*, I need it to perform at ridiculous speeds. So that's how the journey towards the fastest perfect hash function started.


### Setting the stage

Okay, so a little bit of context is necessary.

I'm working with a file containing structured data, and I need to attach metadata to some of the entries. Entries don't have any natural keys, so I have to use the file offset as the key. I'm allowed to spend a long time preprocessing the file, but after that, mapping the offset to the metadata needs to be near-instantaneous.


### PHFs

Problems like this are typically solved is by using *perfect hash functions*. These are hash functions trained on a particular key set, with a lucky property that the hashes of these keys don't collide.

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

:::aside
Okay, I'm overreacting. These crates aren't really tuned towards my usecase, and they shine when used for string keys. But that doesn't help *me*.
:::

There's just one problem: all the classical PHFs are damn slow for my usecase! The [phf](https://docs.rs/phf/latest/phf/) and [boomphf](https://docs.rs/boomphf/latest/boomphf/) crates both take `14 ns` per access, perhaps because they use wyhash for internal hashing. We have 4 GHz CPUs these days, and these crates can't even hash 100M integers per second!

<aside-inline-here />

Are you pondering what I'm pondering? We're going to design a $10 \times$ faster PHF.


## Theory

### Spooky math

The hash function clearly can't be perfect for *all* keys, so it should depend on the key space. There's many different key spaces, so there must be many different PHFs, and that means they have to store *data* that somehow corresponds to the key space. So a PHF is not just *code*, but also a set of *lookup-up tables*.

Most constant-time PHFs (and we do need constant time, because any unpredictable conditional jumps are going to hurt performance) use the following approach. Firstly, two *imperfect* random hash functions are chosen: $\mathit{Approx}$ and $\mathit{Bucket}$. $\mathit{Approx}$ maps the key to its approximate hash (i.e. position in the hash table), while $\mathit{Bucket}$ maps the key to its bucket ID. For each bucket, a *displacement* value is stored in the look-up table, which is mixed with the approximate hash to obtain the final hash.

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
- Build success probability (better for $+$)
- Build time (better for $+$)

For approximate hash $h$ and displacement $d$, we can either have $h \oplus d$ or $h + d$ as the final hash, and the former is clearly less in the general case. For large $h$ and small $d$, this is not a big problem, but for smaller hash tables, using $\oplus$ reduces the hash table size.

The question of success rate is tangentially related to collision rate in open addressing vs closed addressing in hash tables. With $+$, buckets effectively partially overlap, just like in open addressing, while with $\oplus$, they either match completely or don't intersect at all, just like in closed addressing. In typical hash tables, we're only interested in the *expected* collisions, while in PHFs, any collisions that can't be resolved by displacements are fatal. It's a well-known fact (I hope so, anyway; I don't want to burden you with a proof here) that open addressing is more lax in this regard.

As for the build time, we'll discover why the mix function matters later.


### Displacements

How many different displacements do we need to try, i.e. what is the largest $d$ we can expect, and what datatype should we use? There's quite a bit of math here; feel free to skip it if that's not your kind of thing.

For bucket of size $b$ and current load factor is $\alpha$, we can expect a success rate of $(1 - \alpha)^b$ per displacement. Bucket sizes are distributed according to the Poisson distribution, so $\alpha \approx 1.01 - Q(b, 5)$, thus the success rate is $(Q(b, 5) + 0.01)^b$. This peaks at around $1 / 400$, indicating an expected number of $400$ displacements until success. This is the result for *one* bucket; the probability for *all* bucket displacements to be small is lower. In practice, I have found that using $16$-bit displacements works for sizes up to several million with high probability, which suffices for my usecase.


## Implementation

### Uniformity

Popular non-cryptographic hash functions are a big no-no due to performance considerations, and preferably, we should avoid them altogether.

To do this, we need to inspect the input data. As a quick reminder, the keys are data structure offsets. The offsets aren't uniformly random (why would they be, with a variable-length format?), but some bits have more entropy than others. Lower bits have little entropy due to alignment, higher bits have little entropy due to the structure, but some bits inbetween work well as an entropy source.

Here's a distribution plot for 1.5 M keys, rotated by different amounts of bits:

![32 distribution plots, for shifts from 0 to 31 inclusive. For shift 0, the plot is clearly non-uniform, but looks close to that of a continuous function. For shifts 1 to 6, the plot shows increasingly many visible peaks with nothing inbetween. For shifts 7 to 12, these peaks grow in number until the plot becomes visibly uniform. For shifts 13 to 16, the plot is uniform. For shifts 17 to 31, uniformity slowly breaks down and the plot slowly starts to look closer to the original non-uniform almost-continuous plot.](distribution.png)

With [the KS test](https://en.wikipedia.org/wiki/Kolmogorov%E2%80%93Smirnov_test) we can ensure that the most uniform result (on a large scale; low bits, of course, stay non-uniform) is produced by rotation by $14$ bits (and, surprisingly, by $11$), differing from a real uniform distribution by only $0.8\%$ max. With just *one* instruction, we've turned the input into a suitable hash! On x86-64, that takes one tick.

But then I had a very good idea. I used [uops.info](https://uops.info/table.html). See, using uops.info gave me a whole new perspective and I was able to see an instruction I couldn't have seen before.

It turns out that rotation is not the best one-tick way to increase uniformity. This ought to be surprising, because there's very few one-tick instructions, and they typically perform linear arithmetic, which doesn't shuffle bits in any way. But there's *one* instruction that does: `bswap`. It swaps bits inside of a 32-bit register, and with certain inputs, it generates more entropy than rotation does. So in practice, we need to choose whether to use rotation or `bswap` depending on the data.


### Hash functions

*Finally* we're getting closer to writing some code. We need to design $\mathit{Approx}$ and $\mathit{Bucket}$, the functions for mapping keys to approximate hashes and for mapping keys to buckets.

Now here's a problem. We've got a "uniform" $32$-bit number, but $\mathit{Approx}$ needs to output a number between $0$ and $M = 1.01 N$, which isn't necessarily a power of two, so we'll need to perform another calculation to scale the uniform hash. Scale, you say? That's just fixed-point multiplication: $(h \times M) >> 32$ maps $h \in [0; 2^{32})$ to $[0; M)$, which is precisely what we need.

What about $\mathit{Bucket}$? I'd rather avoid yet another multiplication, so we'll have to bite the bullet and round the bucket count to a power of two (buckets take little space per key, so that's okay). We can then use the *lower* half of the $32 \times 32$ multiplication, shifted to the right, as a bucket index.

On x86-64, multiplication takes 3 ticks, and the two shifts (to obtain the high part and to get the bucket index) can be computed in parallel, taking 1 tick each.

So that's, uh, a good hash with a 5 tick latency:

```rust
struct Precomputed {
    entropy_shift: u32,
    hash_space: u32,
    bucket_shift: u32,
}

struct Hash {
    approx: u32,
    bucket: u32,
}

fn hash(key: u32, precomputed: Precomputed) -> Hash {
    let h = key.rotate_right(precomputed.entropy_shift); // 1 tick
    // Alternatively: let h = key.swap_bytes(); // 1 tick
    let product = h as u64 * precomputed.hash_space as u64; // 3 ticks
    Hash {
        approx: (product >> 32) as u32, // 1 tick
        bucket: (product as u32) >> precomputed.bucket_shift, // 1 tick (computed concurrently)
    }
}
```

Nevermind the memory loads; these will go away after inlining:

```x86asm
hash:
    movzx   ecx, byte ptr [rsi]
    ror     edi, cl ; 1 tick
    mov     edx, dword ptr [rsi + 4]
    imul    rdx, rdi ; 3 ticks
    mov     rax, rdx
    movzx   ecx, byte ptr [rsi + 8]
    shr     edx, cl ; 1 tick
    shr     rax, 32 ; 1 tick (computed concurrently)
    ret
```

Now all we have to do is compute the final hash with $\mathrm{mix}(\mathit{Approx}, \mathit{Disp}[\mathit{Bucket}])$.


## Generation

### The what?

Okay, now how do we *generate* the table in the first place? We need to decide on:

- `entropy_shift` (or to switch to `swap_bytes`)
- `hash_space`
- `bucket_shift`
- The displacement LUT

None of these are as simple as they look like.


### Entropy shift

For this, we need a way to measure the uniformity of a distribution. Typical solutions assume the existence of a PDF, but only have access to a sample of the distribution, so we'll use [the KS test](https://en.wikipedia.org/wiki/Kolmogorov%E2%80%93Smirnov_test) instead. In layman terms, we just sort the hashes in increasing order, draw a scatter plot, and compare it to a straight line.

Not that this needs to be particularly performant, but avoiding floating point operations sounds like a nice bonus, so let's do that. We need to compute approximately $\max\limits_{i=0}^{N-1} \left\lvert \frac{h_i}{2^{32}} - \frac{i}{N} \right\rvert$, which we can do as follows:

```rust
fn compute_ks_for_sorted(hashes: &[u32]) -> f32 {
    let scaled_abs = hashes
        .into_iter()
        .enumerate()
        .map(|(i, hash)| (*hash as u64 * hashes.len() as u64).abs_diff((i as u64) << 32))
        .max()
        .unwrap();
    (scaled_abs >> 32) as f32 / hashes.len() as f32
}
```

This takes $1.5$ ns per key to compute, so it's fine, I guess?

Now we just need to figure out how to sort the hashes for 32 different shift amounts without sorting the array 32 times. Luckily, this is easy to do: when a sorted array is rotated to the left by one bit, the two halves (split by the highest bit pre-rotate and by the lowest bit post-rotate) are still sorted, so we can just merge them together in $O(n)$:

```rust expansible
fn rotate_left_sorted(source: &[u32], target: &mut [u32]) {
    assert_eq!(source.len(), target.len());

    let middle = source.partition_point(|key| (key >> 31) == 0);
    let mut i = 0;
    let mut j = middle;

    // Sorry, itertools, this is faster
    while i < middle && j < source.len() {
        let i_value = source[i].rotate_left(1);
        let j_value = source[j].rotate_left(1);
        if i_value < j_value {
            target[i + j - middle] = i_value;
            i += 1;
        } else {
            target[i + j - middle] = j_value;
            j += 1;
        }
    }
    while i < middle {
        target[i + j - middle] = source[i].rotate_left(1);
        i += 1;
    }
    while j < source.len() {
        target[i + j - middle] = source[j].rotate_left(1);
        j += 1;
    }
}
```

This takes $2$ ns per key. Good enough.


### Hash space

This looks trivial: just round $1.01 n$ up. But there's three pitfalls here.

The easier one is that multiplying by an even number shifts entropy to the left, so there might not be enough entropy left for $\mathit{Bucket}$. Thus `hash_space` needs to be odd.

The medium one is that this makes hash calculation completely deterministic. Do you remember that PHF building is a probabilistic process, and if too many collisions arise from the imperfect hash functions $\mathit{Approx}, \mathit{Bucket}$, we need to regenerate them? Well, among all the variables we can easily control, our hash functions depend exclusively on `entropy_shift` and `hash_space`. We *really* want to play around with them, so we'll have to try different `hash_space` values out: perhaps $1.01 n, 1.02 n, 1.03 n, \dots$. This also has a bonus of decreasing the load factor, thus offsetting the effect of a limited displacement range.

The harder one is that while $\mathit{Approx}$ can be in-bounds, $\mathrm{mix}(\mathit{Approx}, \mathit{Disp}[\mathit{Bucket}])$ can be out-of-bounds. For example, for hash table size $5$, a key with approximate hash $4$ and displacement $2$ has final index $6$ (for both $+$ and $\oplus$ mixing functions). Handling this case as wrap-around is a performance hazard, and forbidding such out-of-bounds accesses decreases the probability of a successful build. The solution is to keep the output space of $\mathit{Approx}$ (i.e. `hash_space`) limited to $1.01 n$ (or another multiple of $n$), but pad the resulting hash table to the right such that all possible accesses are in-bounds.

There's a conundrum here. Do we make *accesses to existing elements* in-bounds, or do we legalize *all accesses*? If accessed keys are guaranteed to be present in the hashmap, the former suffices; if we need to add an existence check (like in the first example in this post), we need to do the latter. My case calls for access verification, so we'll need to make all accesses sound, increasing the hash table size to *[Insert incomprehensible math warning]* $1 + \max\limits_{i=0}^{B-1} \max\limits_{x=0}^{M-1} \mathrm{mix}(x, \mathit{Disp}_i)$.

For the $+$ mixing function, this is just $M + \max\limits_{i=0}^{B-1} \mathit{Disp}_i$. For $\oplus$, a tighter boundary is possible; I'm a nerd, so we're going to solve this Leetcode problem now. Skip to the next section if you despise that sort of thing.

We're looking for a way to calculate $\max\limits_{x=0}^X x \oplus Y$ in $O(1)$ for different $Y$ values. I won't bore you with details, but it works like this:

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


### Fallback

Before we continue talking about generation parameters, we need to talk about generating a *family* of hash functions again. As a reminder, we need to generate new $\mathit{Approx}, \mathit{Bucket}$ hashes if the current ones lead to collisions. We can affect `hash_space` and `entropy_shift`, which is a nice hack, but it doesn't always work, because it does not completely reshuffle the key hashes; rather, it tweaks them slightly, barely enough that the collision *might* go away.

So here's how the fallback implementation works. The reason we have a slow multiplication calculation is that `hash_space` is not a power of two; if we *fail* to generate a small enough non-power-of-two table, we round `hash_space` up to a power of two, replacing the multiplication with a bitwise operation. This frees up 2 ticks, enabling us to bring a multiplication *back*. You see, multiplication by an odd number shuffles the bits quite well, so we can just multiply `h` by an honest-to-God random number:

```rust
fn hash(key: u32, precomputed: Precomputed) -> Hash {
    let h = key.rotate_right(precomputed.entropy_shift); // 1 tick
    // Alternatively: let h = key.swap_bytes(); // 1 tick
    let h = h as u64 * precomputed.random_factor as u64; // 3 ticks
    Hash {
        approx: (h >> 32) as u32 & precomputed.hash_space_minus_one, // 2 ticks
        bucket: (h as u32) >> precomputed.bucket_shift, // 1 tick (computed concurrently)
    }
}
```

`random_factor` does not affect performance characteristics like `hash_space` did, so we can use an arbtirary factor. This significantly increases the chances of a successful generation, only decreasing fallback performance by 1 tick.


### Bucket shift

I lied, this one's actually simple. A good change of pace. Do you feel relieved? Please feel relieved.

If you still remember, we compute $\mathit{Bucket}$ of a key as `(product as u32) >> bucket_shift`, so `bucket_shift` is just $32 - \log_2 B$, and the bucket count is $\frac15 N$ (shamelessly stolen from CHD), rounded up to a power of two.


### Displacements

Displacements are a core part of PHFs, so this part's more typical. We split keys into buckets according to $\mathit{Bucket}$, sort the buckets in order of decreasing size, and assign displacements to buckets in order. We try displacements $0, 1, 2, \dots, 2^{16} - 1$; the first applicable displacement (s.t. the elements don't collide with already assigned ones) is used. If no displacement works, we restart the algorithm with different hash functions.

The basic idea of what we're trying to do is this:

```rust expansible
struct Phf {
    hash_space: u32,
    bucket_shift: u32,
    displacements: Vec<u16>,
}

fn try_generate_phf(keys: &[u32]) -> Option<Phf> {
    // Hash space. TODO: Increase on failure.
    let hash_space = (keys.len() + keys.len().div_ceil(100)) | 1;

    // Number of buckets
    let b = (keys.len().div_ceil(5) + 1).next_power_of_two();

    // Split keys into buckets
    let bucket_shift = 32 - b.ilog2();
    let mut keys_per_bucket = vec![Vec::new(); b];
    for key in keys {
        // TODO: Apply entropy shift.
        let bucket = key.wrapping_mul(hash_space as u32) >> bucket_shift;
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
        let mut approx_for_bucket: Vec<u32> = keys_per_bucket[bucket]
            .drain(..)
            .map(|key| ((key as u64 * hash_space as u64) >> 32) as u32)
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
        hash_space: hash_space as u32,
        bucket_shift,
        displacements,
    })
}
```

This code works, but takes around $280$ ns per key. We can do better.


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
let mut buckets_by_size: Vec<Vec<(u32, usize)>> = Vec::new();

// A manual group_by implementation
let mut left = 0;
while left < keys.len() {
    let bucket = key_to_bucket(sorted_by_bucket[left]);

    let mut right = left;
    let mut product;
    while right < keys.len() && {
        // Keep going while the key has the same bucket as the previous one
        product = sorted_by_bucket[right] as u64 * hash_space as u64;
        bucket == product as u32 >> bucket_shift
    } {
        // Replace the key with its Approx value in-place for future use. We have already computed
        // the product, so this is cheap.
        sorted_by_bucket[right] = (product >> 32) as u32;
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

This alone increases performance almost twofold, to $145$ ns per key. `radsort` is not the best radix sort implementation, but it's the only ready one I found that supports stateful `sort_by_key`, so we'll have to live with it unless someone wants to contribute to [voracious_sort](https://github.com/lakwet/voracious_sort).


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
unsafe fn find_valid_displacement(approx_for_bucket: &[u32], free: &[u8]) -> Option<u16> {
    // Outer unrolled loop
    for displacement_base in (0..=u16::MAX).step_by(8) {
        let mut global_bit_mask = u8::MAX;

        // Iterate over keys
        for approx in approx_for_bucket {
            let approx = *approx as usize;
            // Inner unrolled loop, aka bitmask logic
            let bit_mask =
                *unsafe { free.get_unchecked((approx ^ displacement_base as usize) / 8) };
            global_bit_mask &= bit_mask as usize;
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

global_bit_mask |= BIT_INDEX_XOR_LUT[approx % 8][bit_mask as usize];
```

This brings build time down to $65$ ns per key, more than $2 \times$ faster than before.

Unfortunately, increasing this performance further is a lot harder. Increasing the step to 16 bits would either require a 2 MiB LUT, which is not really a good idea for many reasons, or introduce conditionals into the `global_bit_mask |= ...` calculation -- hardly a positive change.


### Displacements (+)

Okay, that was it for $\oplus$. What about $+$?

We're in luck: bits $\mathit{Approx} + 0$ to $\mathit{Approx} + 7$ can be extracted simply by performing an unaligned 16-bit read, followed by a bit shift. But we don't have to limit ourselves to validating $8$ displacements at once: we can easily validate $57$ displacements at once!

```rust expansible
// SAFETY: `free` must be large enough to fit `approx + displacement + 8`.
unsafe fn find_valid_displacement(approx_for_bucket: &[u32], free: &[u8]) -> Option<u16> {
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

This increases performance to $40$ ns per key.
