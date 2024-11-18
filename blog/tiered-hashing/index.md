---
title: "Tiered hashing: fast generative hash functions"
time: November 25, 2024
draft: true
intro:
---

There is a constant call for faster hash tables. Many non-cryptographic hash functions attempt to resolve this, but their performance is sometimes insufficient.

In these cases, faster hash functions are typically designed by hand, tuned to specific data -- quite a time-consuming process. I have developed the [tiered-hash](https://lib.rs/tiered-hash) crate to automatically generate fast hash functions trained on a dataset of keys, usable for hash tables.

The hash function can either be generated in runtime for dynamic data (with JIT compilation) or in compile time for static data. Non-default instruction sets (such as SSE3 on x86-64) can be enabled either automatically (when using JIT) or manually (when using AOT).


### Limitations

As `tiered-hash` trains on a specific dataset, it might behave significantly worse on a different dataset. In other words, *dynamic* hash tables based on `tiered-hash` are *not* DoS-resistant, by design. This is not a problem for static hash tables that are only built once, but dynamic hash tables need guard rails to switch to another hash function if collision rate increases unexpectedly.

`tiered-hash` is a tradeoff between construction time and access time. Hash functions are slower to construct than typically, though still fast enough for certain applications.

`tiered-hash` does not provide *any* security guarantees beyond a good collision rate on the dataset it was trained on.

The hash function generation is parametric, meaning that it has to be redone when the hash table parameters change.


## Architecture

### Output

Most general-purpose hash functions output a pseudo-random $N$-bit number $h$, which are then split into smaller individual hashes $(h_1, \dots, h_k)$ by the hash table. The hash table then typically assumes the $h_i$ are uniform over $[0; n_i)$ (and sometimes independent), and uses them directly for indices, steps, and tags.

For example, in separate chaining $h_1$ is used as the bucket index, while a fixed-length tag $h_2$ may be used for SIMD optimization, much like in SwissTable. For bucket-based PHFs, $h_1$ and $h_2$ are used for an individual position contribution and the bucket index, respectively. For open addressing with double hashing, $h_1$ and $h_2$ can be used as the initial probe index and the step (minus one). For linear probing, stronger guarantees are necessary to reliably prevent primary clustering; we'll handle that later.

Ensuring that the hash is high-quality for any choice of the splitting algorithm is complicated. Instead, our hashes output $(h_1, \dots, h_k)$ directly instead of leaving the post-processing step to the hash table implementation.


### Tiers

As realistic inputs often contain easily extractable entropy, we can often cut corners by not using a high-quality method for producing $h$, as long as it doesn't lead to problems on the training set.

We thus support various hashing methods, ordered into tiers. The word "tier" comes from tiered JIT compilation, where compilers of different tiers provide faster code at expense of compilation time. The first tier quickly produces a quality hash function that's slow in runtime. The next tiers try to decrease the runtime while preserving quality at the cost of increased compilation time.


### Quality

To check whether a certain hash is of sufficient quality, we analyze the number of collisions of *and across* $h_i$. (Again, ignore linear probing for now.)

In particular, we'll analyze the collision rate in certain subtuples $\{h_i\}_{i \in A_j}$ of hashes. Analyzing subsets is necessary because even if both $h_{i_1}$ and $h_{i_2}$ have few collisions, they might be correlated, so using them together does not necessarily decrease the collision rate multiplicatively.

Denoting by $H_j$ the hashes in $A_j$ mapped bijectively into a range $[0; N_j = \prod_{i \in A_j} n_i)$, we're interested in the collision rate of $H_j(x)$ among the elements $x \in X$ of the training set.

We could simply use a generic hash map to track the number of occurences of different $H_j$s, but it turns out that there's a more efficient way. The core of our approach is that at some point during testing, we stumble upon a hash $H_{j'}$ of known good quality, which lets us remove the hashing step from the generic hash map.

We will perform the quality test in order of increasing $N_j$. If a certain $j$ fails the test, we stop.

If $N_j \lessapprox |X|$, we can test $H_j$ for collisions by using an array of size $N_j$ as a histogram and filling it from $X$, tracking the number of encountered collisions. This takes $\Theta(N_j + |X|) = \Theta(|X|)$ time.

Otherwise, suppose $N_j \gg |X|$. *For all practical hash tables*, there exists a subset $A_{j'} \subset A_j$ such that $N_{j'} \approx |X|$. In simpler terms, if a hash table uses a large combination of hashes, it must also use one of its hash-table-sized subsets. This is quite intuitive: SwissTable, for instance, is interested in the collision rate of $(\mathrm{Bucket}, \mathrm{Tag})$, which is larger than the table itself, but it's also interested in the collision rate of $\mathrm{Bucket}$, which exactly spans the table.

What this means is that we can replace the array of size $N_j$ with a hash table that uses $H_{j'}$ as the key. As $H_{j'}$ passed the test, it has a low collision rate, and as $N_{j'} \approx |X|$, this makes it is a good hash for a table containing $|X|$ elements. Therefore, filling the table from $X$ with this hash takes $\Theta(|X|)$ time, and after that's done, we can count the collisions of $H_j$ in each bucket individually. We can do this in quadratic time of the bucket size, which will be performant, as the expected bucket size is just $\Theta(1)$. In total, the algorithm takes $\Theta(|X|)$ time.


## Hashing data

### Groups

Often, we need to hash short data whose properties can be analyzed in reasonable time.

Firstly, note that we don't have to hash all input into one number and then produce $h_1, \dots, h_n$ from it. Instead, we can split the input words into several groups, hash each group individually, and extract each $h_i$ from one of the groups. Some words may be dropped altogether if the other words suffice entropy-wise.

Note that the mapping from $h_i$ to groups is not injective: for example, if $h_1$ and $h_2$ need to be independent, but the input contains just one word, we'll have to extract them both from the hash of that word. Nevertheless, when such a split is possible, it allows several hashes to be computed independently in parallel, relying on the superscalar architecture of modern CPUs.


### Generic hash

So how do we hash the individual groups? Our design is based on [the universal NH family](https://www.cs.ucdavis.edu/~rogaway/papers/umac-full.pdf), which computes

$$
u(x_0, \dots, x_{2k-1}) = \sum_{i=0}^{k-1} ((x_{2i} + a_{2i}) \bmod 2^N) ((x_{2i+1} + a_{2i+1}) \bmod 2^N) \pmod{2^{2N}}
$$

for random odd $a_0, \dots, a_{2k-1}$ and adds $x_{2k}$ for an odd number of words. NH is provably almost universal, meaning that it has good collision rate. We reuse this approach with $N \times N \to 2N$ multiplication replaced with $N \times N \to N$ folded multiplication to simplify the calculations:

$$
\mathrm{mum}(x, y) = (xy \bmod 2^N) \oplus (xy \mathop{div} 2^N),
$$

$$
u(x_0, \dots, x_{2k-1}) = \sum_{i=0}^{k-1} \mathrm{mum}(x_{2i} \oplus a_{2i}, x_{2i+1} \oplus a_{2i+1}) \pmod{2^N}.
$$

Here, both the inputs $x_i$ and the output $u$ are $N$ bits long, and $a_i$ are random (not necessarily odd) $N$-bit values. To hash an odd number of words, we add $x_{2k}$ to the sum.

Purely for intuition, consider that if $+$/$\oplus$ and $\mathrm{mum}$ were replaced with addition and multiplication in $GF(2^N)$, respectively, the scheme would clearly be $2^{-N}$-almost universal. To be clear, we can only speculate on the quality of this replacement, therefore we don't claim any kind of DoS resistance. It seems to work well in practice, but real cryptanalysis is necessary to make any formal claims.

The arbitrary choices of $\oplus$ vs $+$ were made with a goal of minimizing linearity in both $\mathbb{Z} / 2 \mathbb{Z}$ and $\mathbb{Z} / 2^N \mathbb{Z}$.

As another point of comparison, [wyhash](https://github.com/wangyi-fudan/wyhash) successfully uses $\mathrm{mum}$ as a mixer. We use a parallelizable scheme instead, as $\mathrm{mum}$ has a high latency ($5$ ticks on Alder Lake), so absorbing input iteratively with $\mathrm{mum}$ (like `wyhash` does) is slow for short data. On Alder Lake, $u(x_0, \dots, x_{2k-1})$ has latency $6 + \lceil \log_2 k \rceil$ as long as there are enough registers; increment $k$ by one for odd-length data. When the input is just one word, $u(x) = x$.


### Extraction

While we might trust this scheme to be almost universal, that doesn't mean its bits are independent. Even though $\mathrm{mum}$ has full avalanche, it has some very predictable $\Delta$s.

If we only care about the collisions in $h_1, (h_1, h_2), \dots, (h_1, \dots, h_k)$, i.e. just prefixes, we can compute $v_1 = bu \pmod 2^N$ for a random odd $b$ and extract $h_1, \dots, h_k$ iteratively as follows:

$$
\begin{align*}
h_i &= v_i n_i \mathop{div} 2^N, \\
v_{i + 1} &= v_i n_i \bmod 2^N.
\end{align*}
$$

For power-of-two $n_i$, we can optimize multiplication to shifts. This is a generalization of [the multiply-shift scheme](https://en.wikipedia.org/wiki/Universal_hashing#Avoiding_modular_arithmetic), and it provides guaranteed bounds on the collision rate of $(h_1, \dots, h_i)$.

When the extracted $h_i$ need to be independent, a separate finalization step is necessary. [Various bit mixers are available for this](http://mostlymangling.blogspot.com/2019/01/better-stronger-mixer-and-test-procedure.html). With such a high-quality output, we don't need to worry about extracting entropy from the top specifically, and can simultaneously extract bit groups from the bottom for power-of-two $n_i$s.


### Optimizations

So that was our Tier 1. How do we optimize it?

We're looking for ways to remove some operations or replace them with faster ones, utilizing patterns observed in the training set.

Our main tool is to mix individual words faster than with multiplication if the words are independent in some fashion. Several smaller-than-$N$ inputs can be merged together with shifting and addition injectively. (On a relevant note, if we need to produce a $32$-bit hash, merging inputs into $64$-bit words and then truncating the hash is going to be more efficient than working with $N = 32$.) Multiple uncorrelated $N$-bit inputs can often be mixed simply with $+$ or $\oplus$. Correlated inputs can be "decorrelated" by bit-rotating or byte-swapping.

Once a smaller vector $(y_0, \dots, y_{k-1})$ is obtained such that no two $y_i, y_j$ can be mixed together cheaply without increasing the collision rate significantly, we hash the vector with the function $u$.

We then try to find a word that we can directly add/xor into the output of $u$ instead of passing it as an argument $u$. Bit-rotating or byte-swapping $y_i$ might also be useful here. How successful this is depends on the data.

We then consider optimizing individual $\mathrm{mum}$ calls. For uniform inputs, $\oplus a_i$ can be omitted, saving one or more xors. Alternatively, they can sometimes be reused across invocations, which is a positive, as $64$-bit constants have to be loaded into registers manually. Finally, if the input is somewhat uniform, $\mathrm{mum}$ can sometimes be replaced with just the low half of the multiplication, which has latency $3$ rather than $5$.

We then check if the hash function works well with the finalizer omitted, or perhaps only applied to the $+ y_{2k}$ part to reduce latency.

We then optimize the finalizer. We iterate through faster and worse bit mixers, attempting to rotate bits of the hash (or just of the $+ y_{2k}$ part) for better result. We consider extracting entropy from the word directly with a `pext` instruction when all $n_i$ are powers of two, saving one AND.

Finally, we try to use CRC32 as the finalizer, which performs as fast as multiplication while achieving full avalanche on x86-64. The instruction `crc32 a, b` computes CRC32 of $a \oplus b$, allowing us to save $1$ more tick when $x$ is a xor-mix. This produces a $32$-bit hash, so we need to adapt the extraction logic.


### Long data

When hashing longer objects, we can try to find a few individual subwords that have enough entropy and hash them with the above methods.

If that fails, we have to hash the byte array directly instead, for which any of your favorite hash algorithms suffice. Note that we don't necessarily have to choose a "good" hash, just one that leads to few collisions on the training dataset. In order of decreasing performance and increasing collision resistance, we thus try:

- CRC32
- The AES-NI implementation of `t1ha0`
- XXH3
- MeowHash


### Sources

Several papers contibuted to the approaches described in this article:

- M. Dietzfelbinger, T. Hagerup, J. Katajainen, M. Penttonen. [A Reliable Randomized Algorithm for the Closest-Pair Problem](https://doi.org/10.1006/jagm.1997.0873)
- J. Black, S. Halevi, H. Krawczyk, T. Krovetz, P. Rogaway. [UMAC: Fast and Secure Message Authentication](https://www.cs.ucdavis.edu/~rogaway/papers/umac-full.pdf)
<!-- - P. Crowley, E. Biggers. [Adiantum: length-preserving encryption forentry-level processors](https://tosc.iacr.org/index.php/ToSC/article/view/7360/6530) -->
<!-- - D. Lemire, O. Kaser. [Faster 64-bit universal hashing using carry-less multiplications](https://arxiv.org/pdf/1503.03465) -->
<!-- - D. J. Bernstein. [Polynomial evaluation and message authentication](https://cr.yp.to/antiforgery/pema-20071022.pdf) -->
