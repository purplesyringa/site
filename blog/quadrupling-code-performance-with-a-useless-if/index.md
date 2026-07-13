---
title: Quadrupling code performance with a "useless" if
time: July 12, 2026
discussion:
    - https://lobste.rs/s/1an425/quadrupling_code_performance_with
    - https://news.ycombinator.com/item?id=48889148
intro: |
    So I was [optimizing a domain-specific compressor the other day](/blog/optimal-parse-with-phminposuw/), as one does.

    One important problem was chunking the input string and optimally choosing the most compact encoding for each chunk (different encodings compress different characters better, so where to split is not immediately obvious). [The previous post](/blog/optimal-parse-with-phminposuw/) describes the algorithm if you're interested, but it boils down to finding the shortest path on a grid. For each cell, the algorithm computes the best cell following it. Following references from the first cell to the last one gives the optimal coding order.
---

So I was [optimizing a domain-specific compressor the other day](../optimal-parse-with-phminposuw/), as one does.

One important problem was chunking the input string and optimally choosing the most compact encoding for each chunk (different encodings compress different characters better, so where to split is not immediately obvious). [The previous post](../optimal-parse-with-phminposuw/) describes the algorithm if you're interested, but it boils down to finding the shortest path on a grid. For each cell, the algorithm computes the best cell following it. Following references from the first cell to the last one gives the optimal coding order.

```c
uint8_t next_j[n_symbols][8]; // references to the next cell

// The core of the algorithm populating `next_j`.
// Don't worry too much about understanding it, it's just here for completeness.
__m128i best_path_length = _mm_setzero_epi16();
for (int i = n_symbols - 1; i >= 0; i--) {
    __m128i tmp = _mm_add_epi16(cost[i], best_path_length);
    __m128i minpos = _mm_minpos_epu16(tmp);
    __m128i cost_without_switching = _mm_sub_epi16(tmp, _mm_broadcastw_epi16(minpos));
    __m128i cost_with_switching = _mm_set1_epi16(switch_cost);
    best_path_length = _mm_min_epu16(cost_without_switching, cost_with_switching);
    __m128i choice = _mm_blendv_epi8(
        _mm_set1_epi16(_mm_extract_epi16(minpos, 1)),
        _mm_set_epi16(7, 6, 5, 4, 3, 2, 1, 0),
        _mm_cmpeq_epi16(best_path_length, cost_without_switching)
    );
    _mm_storeu_si64(&next_j[i], _mm_packs_epi16(choice, choice));
}

// Find the optimal encoding for each symbol.
// Chunk boundaries are located where encodings change.
uint8_t encoding[n_symbols];
uint8_t j = 0; // always start with encoding 0 for simplicity
for (int i = 0; i < n_symbols; i++) {
    j = next_j[i][j];
    encoding[i] = j;
}
```

That long loop is not the topic of this post, it's well-optimized. We're here to talk about the second loop, which at first glance looks much simpler.


### Latency

Excluding the write, the body of the loop is just `j = next_j[i][j]`, which compiles to a single `mov` instruction. How could this possibly not be optimal?

If we were programming in 1984, it would be, but modern processors have [instruction-level parallelism](https://en.wikipedia.org/wiki/Instruction-level_parallelism) -- that is, they can execute multiple instructions in parallel. This works even across iterations of a loop, and it's one reason why we usually don't pay attention to instructions for `i < n_symbols` and `i++` when evaluating loop performance -- they don't usually prevent the CPU from doing more work.

Crucially, though, you cannot run two *dependent* instructions at the same time. In our case, each iteration of the loop cannot begin before the previous iteration ends because `j` is threaded through the loop, so we're limited by the latency of memory access, which is pretty noticeable even with cache.

Can this be fixed? In this specific case, yes! We don't expect too many chunks, so `next_j[i][j]` is quite likely to just be equal to `j`. If we could tell the CPU to predict that `j` stays intact, the loop would become throughput-bound rather than latency-bound.

While we don't have direct control over address prediction, we can simulate this with branch prediction:

```c
for (int i = 0; i < n_symbols; i++) {
    if (j != next_j[i][j]) {
        j = next_j[i][j];
    }
    encoding[i] = j;
}
```

If the CPU predicts the `if` body as unlikely, it will ignore it and thus not see any dependency between different iterations. When the condition eventually evaluates to `true`, branch misprediction resolution will kick in, undo wrong speculative writes, and restart with the right `j`. That's exactly what we want!


### Lies to compilers

The only issue is that from the perspective of the compiler, this `if` is completely useless. If `j` was in memory, it would avoid possibly writing to read-only memory, but it's in a *register*. Unlike most other cases where we'd reach for compiler hints, we want to convert branchless code to branchy, not the other way round -- and no compiler supports that, least of all for code that any [CSE pass](https://en.wikipedia.org/wiki/Common_subexpression_elimination) will remove without a second thought! Stupid compiler doesn't realize integers have hardware provenance.

The only way to implement this that I'm aware of is with a cast to `volatile` to make it seem like the condition and the assignment are independent:

```c
for (int i = 0; i < n_symbols; i++) {
    if (j != next_j[i][j]) {
        j = *(uint8_t volatile *)&next_j[i][j];
    }
    encoding[i] = j;
}
```

> *Edited on July 13*: Or so I thought -- as [ibookstein discovered](https://lobste.rs/s/1an425/quadrupling_code_performance_with#c_4clhdw), an `[[unlikely]]` annotation (or `__builtin_expect(..., 0)`) also has this effect with LLVM. `volatile` is still useful, though, because it generates better code and also works with GCC.

In a synthetic benchmark, this change has sped up the loop from $320$ us to $80$ us on my data. (This doesn't look like much, but the loop runs many times during compression, so it adds up.)

In a more realistic experiment, I only witnessed a $2 \times$ increase, most likely due to suboptimal codegen by LLVM. Still worthwhile, though!


### Sidenote

Interestingly, in this algorithm specifically, each `next_j[i][j]` can only be one of two values -- either `j` (most often), or some value *dependent only on `i`, but not `j`*. So I could replace each 8-element array `next_j[i]` with that value paired with a bitmask, which would automatically make the `if` semantically important and remove the need for `volatile` shenanigans. But that would likely slow down the code, since testing a variable bit is slower than a comparison (at least on x86).
