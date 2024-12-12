---
title: Thoughts on Rust hashing
time: December 12, 2024
discussion:
    - https://www.reddit.com/r/rust/comments/1hclif3/thoughts_on_rust_hashing/
    - https://internals.rust-lang.org/t/low-latency-hashing/22010
intro: |
    In languages like Python, Java, or C++, values are hashed by calling a "hash me" method on them, implemented by the type author. This fixed-hash size is then immediately used by the hash table or what have you. This design suffers from some obvious problems, like:

    How do you hash an integer? If you use a no-op hasher (booo), DoS attacks on hash tables are inevitable. If you hash it thoroughly, consumers that only cache hashes to optimize equality checks lose out of performance.
---

In languages like Python, Java, or C++, values are hashed by calling a "hash me" method on them, implemented by the type author. This fixed-hash size is then immediately used by the hash table or what have you. This design suffers from some obvious problems, like:

How do you hash an integer? If you use a no-op hasher (booo), DoS attacks on hash tables are inevitable. If you hash it thoroughly, consumers that only cache hashes to optimize equality checks lose out of performance.

How do you mix hashes? You can:

- Leave that to the users. Everyone will then invent their own terrible mixers, like `x ^ y`. Indeed, both arguments are pseudo-random, what could possibly go wrong?
- Provide a good-enough mixer for most use cases, like `a * x + y`. Cue CVEs because people used `mix(x, mix(y, z))` instead of `mix(mix(x, y), z)`.
- Provide a quality mixer, missing out on performance in common simple cases.

What if the input data is already random? Then you're just wasting cycles.

What guarantees do you provide regarding the hash values?

- Do you require the avalanche effect? Your hash is suboptimal even for simple power-of-two-sized hash tables.
- Do you require a half-avalanche effect instead? Congrats, you broke either those or prime-sized hash tables.
- Do you require the hash table to perform finalization manually? Using strings as keys is now suboptimal, because computing a non-finalized hash of a string is of good enough quality already.

Is your hash function seeded?

- If not, hi DoS.
- If yes, but you reuse the same seed between different hash tables, [your tables are now quadratic](https://accidentallyquadratic.tumblr.com/post/153545455987/rust-hash-iteration-reinsertion).
- If the seed is explicitly passed to each hasher, how do you ensure different hashers don't accidentally cancel out?


### In Rust

Rust learnt from these mistakes by splitting the responsibilities:

- Objects implement the `Hash` trait, allowing them to write underlying data into a `Hasher`.
- Hashers implement the `Hasher` trait, which hashes the data written by `Hash` objects.

Objects turn the structured data into a stream of integers; hashers turn the stream into a numeric hash.

On paper, this is a good solution:

- Hashing an integer is as simple as sending the integer to the hasher. Consumers can choose hashers that provide the necessary guarantees.
- Users don't have to mix hashes. Hashers can do that optimally.
- If the data is known to be random, a fast simple hasher can be used without changing the `Hash` implementation.
- Different hash tables can use different hashers, efficiently providing only as much avalanche as necessary.
- The hasher can be seeded per-table. Only the hasher has access to the seed, so safely using the seed during mixing is easy.

Surely this enables optimal and performant hashing in practice, right?


### No

Let's take a look at the `Hasher` API:

```rust
pub trait Hasher {
    // Required methods
    fn finish(&self) -> u64;
    fn write(&mut self, bytes: &[u8]);

    // Provided methods
    fn write_u8(&mut self, i: u8) { ... }
    fn write_u16(&mut self, i: u16) { ... }
    fn write_u32(&mut self, i: u32) { ... }
    fn write_u64(&mut self, i: u64) { ... }
    fn write_u128(&mut self, i: u128) { ... }
    fn write_usize(&mut self, i: usize) { ... }
    fn write_i8(&mut self, i: i8) { ... }
    fn write_i16(&mut self, i: i16) { ... }
    fn write_i32(&mut self, i: i32) { ... }
    fn write_i64(&mut self, i: i64) { ... }
    fn write_i128(&mut self, i: i128) { ... }
    fn write_isize(&mut self, i: isize) { ... }
    fn write_length_prefix(&mut self, len: usize) { ... }
    fn write_str(&mut self, s: &str) { ... }
}
```

This API is tuned to *streaming hashes*, like the polynomial hash and its various knock-offs. But just like in encryption, hashing is block-wise these days.

Block hashes have some internal state that iteratively "absorbs" input blocks of a fixed length. When the data runs out, the last block is padded with length and absorbed as a fixed-length block too. A finalization step then reduces the internal state to 64 bits (or more, depending on the use case).

That's how SHA-2 and many other cryptographic hashes work, but you might be surprised to know that the top hashes [in the SMHasher list](https://gitlab.com/fwojcik/smhasher3/-/tree/main/results) all use the same approach.

The block-wise design is objectively superior to streaming. Consuming as much data as possible at once reduces the amortized avalanche cost, enabling safer hash functions at greater speed than streaming hashes can achieve. Block-wise hashes have a lower latency, as the latency is accumulated per-block, not per-stream-input.


## Block hash support in Rust

### :ferrisClueless:

The `Hasher` API makes no effort to suit block hashes. The hasher is not informed of the length of the data or of its structure. It must *always* be able to absorb just one more `u8`, bro, I promise. There's only two ways to deal with this:

- Either you pad all individual inputs, even very short ones, to the full block width,
- Or you accumulate a block and occasionally flush it to the underlying block hasher.

Let's see what's wrong with these approaches.


### Padding

Let's consider a very simple block-wise hash:

```rust
fn absorb(state: &mut u64, block: &[u8; 8]) {
    let block = u64::from_ne_bytes(*block);
    *state = state.wrapping_mul(K).wrapping_add(block);
}
```

This is just a multiplicative hash, not unlike FNV-1, but consuming $8$ bytes at a time instead of $1$.

Now what happens if you try to hash two 32-bit integers with this hash? With padding, that will compile to two multiplications even though one would work. This halves throughput and increases latency.

Practical hashes use much larger blocks. `rapidhash` has a $24$-byte state and can absorb $48$ bytes at once. `ahash` has a $48$-byte state and absorbs $64$-byte blocks. `meowhash` has a $128$-byte state and absorbs $256$ bytes. (I only selected these particular hashes because I'm familiar with their kernels; others have similar designs.)

These are some of the fastest non-cryptographic hashes in the world. Do you really want to nuke their performance by padding $8$-byte inputs to $48$, $64$, or $256$ bytes? Probably not.


### Chains

Okay, but what if we cheated and modified the hash functions to absorb small data somewhat more efficiently than by absorbing a full block?

Say, the `rapidhash` kernel is effectively *this*:

```rust
fn absorb(state: &mut [u64; 3], seed: &[u64; 3], block: &[u64; 6]) {
    for i in 0..3 {
        state[i] = mix(block[i] ^ state[i], block[i + 3] ^ seed[i]);
    }
}
```

That's three independent iterations, so *surely* we can absorb a smaller 64-bit block like this instead:

```rust
fn absorb_64bit(state: &mut [u64; 3], seed: &[u64; 3], block: u64) {
    state[0] = mix(block ^ state[0], seed[0]);
}
```

Surely this is going to reduce the $6 \times$ slowdown to at least something like $2 \times$, right?

Why does `rapidhash` even use three independent chains in the first place? That's right, latency!

`mix` has a $5$ tick latency on modern x86 processors, but a throughput of $1$. Chain independence allows a $16$-byte block to be consumed without waiting for the previous $16$ bytes to be mixed in. We just threw this optimization out.


### Accumulation

Okay, so padding is a terrible idea. Can we accumulate a buffer instead? How much hashes I had to scroll through in SMHasher before I found *one* Rust implementation that took this approach is a warning bell.

[The implementation I found](https://docs.rs/farmhash/1.1.5/src/farmhash/lib.rs.html#92-110), of course, stores a `Vec<u8>` and passes it to the underlying hasher in `finish`. I believe I don't need to explain why allocating in a hash function is not the brightest idea.

Let's consider [another implementation](https://docs.rs/highway/1.2.0/src/highway/portable.rs.html#272-288) that stores a fixed-size buffer instead. Huh, that's a lot of `if`s and `for`s. I wonder what Godbolt will say about this. Let's try something very simple:

```rust expansible
struct StreamingHasher {
    block_hasher: BlockHasher,
    buffer: [u8; 8],
    length: usize,
}

impl StreamingHasher {
    fn write(&mut self, input: &[u8]) {
        // If the input fits in the free space in the buffer, just copy it.
        let rest = unsafe { self.buffer.get_unchecked_mut(self.length..) };
        if input.len() < rest.len() {
            rest[..input.len()].copy_from_slice(input);
            self.length += input.len();
            return;
        }

        // Otherwise, copy whatever fits and hash the chunk.
        let (head, tail) = input.split_at(rest.len());
        rest.copy_from_slice(head);
        self.block_hasher.feed(self.buffer);

        // Split the rest of the input into blocks and hash them individually, move the last one
        // to the buffer.
        let chunks = tail.array_chunks();
        let remainder = chunks.remainder();
        self.buffer[..remainder.len()].copy_from_slice(remainder);
        self.length = remainder.len();
        for chunk in chunks {
            self.block_hasher.feed(*chunk);
        }
    }
}
```

Surely this will compile to good code? :ferrisClueless:

Here's what writing 1 (one) byte into this hasher compiles to:

```x86asm expansible
write_u8:
        push    r15
        push    r14
        push    r13
        push    r12
        push    rbx
        sub     rsp, 16
        mov     rbx, rdi
        mov     byte ptr [rsp + 15], sil
        mov     r14, qword ptr [rdi + 16]
        add     rdi, r14
        add     rdi, 8
        cmp     r14, 6
        ja      .LBB0_2
        mov     byte ptr [rdi], sil
        mov     r14, qword ptr [rbx + 16]
        inc     r14
        jmp     .LBB0_3
.LBB0_2:
        lea     r15, [rbx + 8]
        mov     edx, 8
        sub     rdx, r14
        lea     r12, [rsp + rdx]
        add     r12, 15
        add     r14, -7
        lea     rsi, [rsp + 15]
        mov     r13, qword ptr [rip + memcpy@GOTPCREL]
        call    r13
        movabs  rax, 5512829513697402577
        imul    rax, qword ptr [rbx]
        add     rax, qword ptr [rbx + 8]
        mov     qword ptr [rbx], rax
        mov     rsi, r14
        and     rsi, -8
        add     rsi, r12
        mov     rdi, r15
        mov     rdx, r14
        call    r13
.LBB0_3:
        mov     qword ptr [rbx + 16], r14
        add     rsp, 16
        pop     rbx
        pop     r12
        pop     r13
        pop     r14
        pop     r15
        ret
```

Waow, what happened? That's right, `copy_from_slice` did! LLVM *cannot* compile a variable-length copy into anything other than `memcpy`. Did you write a loop with a guaranteed bound on the iteration count by hand? Too bad, that goes in the `memcpy` hole.


### SipHasher

So crates in the wild do this wrong. How does the built-in Rust hasher handle this? [It conveniently doesn't define `write_*`](https://github.com/rust-lang/rust/pull/69152) -- by design, because this important optimization leads to a small increase in compile time. Riiiiiight.

The `siphasher` *crate*, though, optimizes the short-length `memcpy` with [bitwise operations](https://docs.rs/siphasher/latest/src/siphasher/sip.rs.html#330-354). Let's try it out:

```rust expansible
fn write(&mut self, input: u64, input_len: usize) {
    assert!(input_len <= 8);
    if input_len != 8 {
        assert!(input >> (8 * input_len) == 0);
    }

    // Consume as many inputs as fit.
    let old_length = self.length;
    self.buffer |= input << (8 * self.length);
    self.length += input_len;

    // On overflow, feed the buffer block hasher and initialize the buffer with the tail.
    if self.length > 8 {
        self.block_hasher.feed(self.buffer);
        self.buffer = input >> (8 * (8 - old_length));
        self.length -= 8;
    }
}
```

```x86asm expansible
write_u8:
        mov     rax, qword ptr [rdi + 16]
        movzx   ecx, sil
        lea     edx, [8*rax]
        lea     rsi, [rax + 1]
        shlx    rdx, rcx, rdx
        or      rdx, qword ptr [rdi + 8]
        mov     qword ptr [rdi + 8], rdx
        mov     qword ptr [rdi + 16], rsi
        cmp     rsi, 9
        jb      .LBB0_2
        movabs  rsi, 5512829513697402577
        imul    rsi, qword ptr [rdi]
        add     rsi, rdx
        mov     edx, eax
        add     rax, -7
        neg     dl
        mov     qword ptr [rdi], rsi
        shl     dl, 3
        shrx    rcx, rcx, rdx
        mov     qword ptr [rdi + 8], rcx
        mov     qword ptr [rdi + 16], rax
.LBB0_2:
        ret
```

This is kind of better? Now let's try hashing `(u8, u8)` like Rust would do:

```rust
#[no_mangle]
fn write_u8_pair(&mut self, pair: (u8, u8)) {
    self.write(&[pair.0]);
    self.write(&[pair.1]);
}
```

```x86asm expansible
write_u8_pair:
        mov     r8, qword ptr [rdi + 16]
        movzx   r9d, sil
        movabs  rax, 5512829513697402577
        lea     ecx, [8*r8]
        shlx    rsi, r9, rcx
        or      rsi, qword ptr [rdi + 8]
        lea     rcx, [r8 + 1]
        cmp     rcx, 9
        jb      .LBB1_2
        mov     rcx, qword ptr [rdi]
        imul    rcx, rax
        add     rcx, rsi
        mov     qword ptr [rdi], rcx
        mov     ecx, r8d
        add     r8, -7
        neg     cl
        shl     cl, 3
        shrx    rsi, r9, rcx
        mov     rcx, r8
.LBB1_2:
        lea     r8d, [8*rcx]
        movzx   edx, dl
        shlx    r8, rdx, r8
        or      r8, rsi
        lea     rsi, [rcx + 1]
        mov     qword ptr [rdi + 8], r8
        mov     qword ptr [rdi + 16], rsi
        cmp     rcx, 8
        jb      .LBB1_4
        imul    rax, qword ptr [rdi]
        add     rax, r8
        mov     qword ptr [rdi], rax
        mov     eax, ecx
        add     rcx, -7
        neg     al
        shl     al, 3
        shrx    rax, rdx, rax
        mov     qword ptr [rdi + 8], rax
        mov     qword ptr [rdi + 16], rcx
.LBB1_4:
        ret
```

Waow. So elegant. What went wrong?

In retrospect, the reason is obvious. The two writes can "tear" if the first write fills the buffer to the end. The optimizer does not realize the writes can be combined, so we're left with this monstrosity.

More generally, the problem is that `write_*` methods cannot predict the current state of the buffer, so the branches and variable-index accesses cannot be optimized out. And if `write`s forced the state to a fixed one? Well, that's equivalent to padding the data to a full block. Eugh.


### Inlining

Okay, but hear me out, *surely* the state can be predicted if the `hash` and `write_*` calls are inlined? Here:

```rust
fn hash_u8_pair(pair: (u8, u8)) -> u64 {
    let mut hasher = Self::new();
    hasher.write_u8(pair.0);
    hasher.write_u8(pair.1);
    hasher.finish()
}
```

```x86asm
hash_u8_pair:
        movzx   eax, sil
        movzx   ecx, dil
        shl     eax, 8
        or      eax, ecx
        ret
```

That's a nice argument, but let me introduce to you: variable-length collections. `Vec<T>` is hashed by writing the length and then hashing the elements one by one. Even if the element hashing is somehow vectorized (it's not, LLVM is a dumdum), nothing *after* this variable-length collection can be hashed efficiently.


### std

*Surely* someone thought of this problem before? C'mere, take a look at how slices of integers [are hashed](https://doc.rust-lang.org/src/core/hash/mod.rs.html#818-827):

```rust
#[inline]
fn hash_slice<H: Hasher>(data: &[$ty], state: &mut H) {
    let newlen = mem::size_of_val(data);
    let ptr = data.as_ptr() as *const u8;
    // SAFETY: `ptr` is valid and aligned, as this macro is only used
    // for numeric primitives which have no padding. The new slice only
    // spans across `data` and is never mutated, and its total size is the
    // same as the original `data` so it can't be over `isize::MAX`.
    state.write(unsafe { slice::from_raw_parts(ptr, newlen) })
}
```

So that's good.

Meanwhile newtypes are crying in the corner, as `#[derive(Hash)]` understandably does not apply this optimization to them (nor to structs with multiple fields and tuples), the built-in hasher uses $2.5 \times$ worse code than it could even today, which *also* takes way more space in your instruction cache than necessary.


## It can't be that bad

### It can

Shall we benchmark some code?

```rust
use std::any::type_name;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::Instant;

fn time<T: Hash>(obj: T) {
    let start = Instant::now();
    let mut hasher = DefaultHasher::new();
    obj.hash(&mut hasher);
    let h = hasher.finish();
    println!("{}: {:?} (-> {h:0x})", type_name::<T>(), start.elapsed());
}

#[derive(Hash)]
struct NewType(i32);

fn main() {
    let n = 100000000;
    time((0..n).collect::<Vec<i32>>());
    time((0..n).map(NewType).collect::<Vec<NewType>>());
}
```

Hashing `[i32]` transmutes the slice into `[u8]` and performs a single `write` call, while hashing `[NewType]` hashes the elements one by one. This benchmark thus measures the cost of individual calls. Note also that we hash almost $400$ MiB of memory. This doesn't fit in cache, which might *hide* some inefficiencies. I'm feeling generous.

```
alloc::vec::Vec<i32>: 117.756736ms (-> 1984796e743a33f5)
alloc::vec::Vec<ruined_portal::NewType>: 469.774204ms (-> 1984796e743a33f5)
```

~~Huh, literally 1984.~~

We get $5 \times$ slower code, even though it computes the exact same hash. Let's try the `siphasher` crate:

```
alloc::vec::Vec<i32>: 196.330253ms (-> 95697c476562afec)
alloc::vec::Vec<ruined_portal::NewType>: 243.031408ms (-> 95697c476562afec)
```

That's better, though admittedly a $25\%$ difference is still eugh. But keep in mind that this is a *cryptographic* hash, which takes *a lot* of time to hash a block. This difference will be exacerbated on non-cryptographic hashes.

`rapidhash`:

```
alloc::vec::Vec<i32>: 54.224434ms (-> 1908e25736ad8479)
alloc::vec::Vec<ruined_portal::NewType>: 278.101368ms (-> 949efa02155c336a)
```

`ahash`:

```
alloc::vec::Vec<i32>: 56.262629ms (-> 217325aa736f75a8)
alloc::vec::Vec<ruined_portal::NewType>: 177.900032ms (-> 4ae6133ab0e0fe9f)
```

`highway`:

```
alloc::vec::Vec<i32>: 53.843217ms (-> f2e68b031ff10c02)
alloc::vec::Vec<ruined_portal::NewType>: 547.520541ms (-> f2e68b031ff10c02)
```

That's not good. Note that all hashers have about the same performance on `Vec<i32>`. That's about the speed of RAM. For small arrays that fits in cache, the difference is even more prominent. (I didn't verify this, but I am the smartest person in the room and thus am obviously right.)


## My goal

### (Kinda)

What I really want is a general-purpose hash that's good for most practical purposes and kinda DoS-resistant but not necessarily cryptographic. It needs to perform fast on short inputs, so it can't be a "real" block hash, but rather something close to `rapidhash`.

We want:

$$
\mathrm{consume}(a, x, y) = \mathrm{mix}(x \oplus a, y \oplus C).
$$

Right, Rust doesn't support this. Okay, let's try another relatively well-known scheme that might be easier to implement. It's parallel, surely that'll help?

To hash a $64$-bit word sequence $(x_1, \dots, x_{2n})$, we compute

$$
\mathrm{mix}(x_1 \oplus a_1, x_2 \oplus a_2) + \dots + \mathrm{mix}(x_{2n - 1} \oplus a_{2n - 1}, x_{2n} \oplus a_{2n}),
$$

where $(a_1, \dots, a_{2n})$ is random data (possibly generated from the seed once), and

$$
\mathrm{mix}(x, y) = (x \cdot y \bmod 2^{64}) \oplus (x \cdot y \mathop{div} 2^{64}).
$$

This is a UMAC-style combination of certain well-known primitives. The problem here is that $a_i$ needs to be precomputed beforehand. This is not a problem for fixed-length keys, like structs of integers -- something often used in, say, `rustc`.

Unfortunately, Rust forces each hasher to handle *all* possible inputs, including of different lengths, so this scheme can't work. The hasher isn't even parametrized by the type of the hashed object. Four well-layouted 64-bit integers that can easily be mixed together with just two full-width multiplications? Nah, `write_u64` goes brrrrrrrrrrrr-


### Stop bitching

I've been designing fast hash-based data structures for several months before realizing they are, in fact, not fast, purely because of the hashing performance. *Surely* something that isn't a problem in C++ and Python won't be a problem in Rust, I thought.

But yeah, sorry for whining.


### Actually how

The obvious way forward is to bring the structure of the data back into the picture. If the hasher knew it's hashing fixed-size data, it could use the $a_i$ approach. If the hasher knew it's hashing an array, it could vectorize the computation of individual hashes. If the hasher knew the types of the fields in the structure it's hashing, it could prevent tearing, or perhaps merge small fields into 64-bit blocks efficiently. Alas, the hasher is clueless...

In my opinion, `Hasher` and `Hash` are a wrong abstraction. Instead of the `Hash` driving the `Hasher` ~~insane~~, it should be the other way round: `Hash` providing introspection facilities and `Hasher` navigating the hashed objects recursively. As a bonus, this could enable (opt-in) portable hashers.

How this API should look like and whether it can be shoehorned into the existing interfaces remains to be seen. I have not started work on the design yet, and perhaps this article might be a bit premature, but I'd love to hear your thoughts on how I missed something really obvious (or, indeed, on how Rust is fast enough and no one cares).


## Non-solutions

### Like C++

I'd like to note that the way Java, C++, and Python take is not without its own share of problems. The good news is that fields in a product type are hashed the same way regardless of the values of other fields. For example, hashing `(Vec<T>, U)` always applies the same hash to `U` and the mixes it with the hash of `Vec<T>`, unlike Rust.

However, this approach is suboptimal in the general case. Let's get back to the UMAC example. Hashing $((a, b), c)$ as $\mathrm{mix}(\mathrm{mix}(a, b), c)$ has a higher latency than necessary: computing $\mathrm{mix}(a, b) \oplus \mathrm{mix}(c, 0)$ would suffice. But, again, applying this rule generally as $\mathrm{mix}(a, 0) \oplus \mathrm{mix}(b, 0) \oplus \mathrm{mix}(c, 0)$ is suboptimal too.

This odd $\mathrm{mix}(a, b)$/$\mathrm{mix}(a, 0)$ duality arises because the block size of the UMAC-style approach is, at its minimum, two $64$-bit words, while hashes take $64$ bits. This distinction gets much worse for larger block sizes.


### Specialization

After this article was published, several people advised me to look into specialization. I'd like to comment a bit on why this does not solve the problem either.

Specialization does not support efficient hashing of nested objects. Although `(u8, u8, u8, u8)` can be specialized to be hashed with `write_u32`, this gets complicated with types like:

```rust
struct S {
    a: (u8, u16),
    b: u8,
}
```

The best way to serialize this type is to fit `b` into the padding byte of `a`. We can't do that during layouting, but we can when hashing. This is very hard to do automatically just with specialization, and next to impossible if people implement `Hash` manually.


### Rule of thumb

The bottom line is: hashing a product type can only be efficient if it's linearized. Hashing a structure composed of structures *needs* to consider the nested fields. Each such field *needs* to be associated with a static index, so that it can be associated with a constant from a pool, an offset in the block, or what have you.

Fields that are stored in the structure after variable-length fields like `&[T]`/`Vec<T>` needs to have static indices regardless.

This applies to arrays: hashing `[T; 2]` by performing two calls into `T::hash` is suboptimal, because that leads to reuse of constants, which in turn requires more thorough mixing for acceptable hash quality.

It also applies to *slices*: hashing `[T]` needs to split the slice into fixed-size chunks, where each chunk is hashed as a single block. Extending the API to emit start/end annotations for `[T]` slices does not help *either*, because the indices of fields inside each `T` need to be predictable, too. If `Hash for T` emits $3$ words and the block size is $8$ words, this will vectorize *badly* due to the misalignment.

As much as these rules apply to product types, they apply to sum types. Hashing a `Result<T, E>` needs to either produce $h_1(\mathrm{ok})$ or $h_2(\mathrm{err})$, where $h_i$ are different elements of a hash family. This can be *simulated* by prepending the discriminant, but this is suboptimal. Perhaps more clearly, `Option<T>` should either hash its element or return a random (but static) constant for `None`.

These rules apply to objects that contain non-primitives too. Hashing

```rust
struct Key {
    top: u64,
    middle: u64,
    low: u64,
    meta: Option<String>,
}
```

shouldn't be slower than hashing `[u64; 3]` in the cases where `meta` is `None`, and should be barely slower than that if it's `Some`, as long as the string is short. This isn't magic -- Rust just can't represent the solution in the type system.
