---
title: Optimal parse with phminposuw
time: July 11, 2026
intro: |
    This problem arose when I was writing a specialized data compressor.

    Say you want to encode a byte stream, and bytes can be encoded in different formats, e.g. optimized for ASCII, numbers, raw binary, etc. These formats prioritize better compression of a specific type of data. Realistic byte streams may contain all of them at different points, so we want to switch between formats on the fly optimally.

    The easiest way to do so is to split data into, say, 50-byte chunks, find the best format for each chunk, and use those formats. But that doesn't take into account that we need to store which format is used for which chunk, and choosing formats greedily may increase the size of this metadata because it itself compresses worse. Not to mention that it's not very precise. So what should we do?
---

<aside-start-here />

This problem arose when I was writing a specialized data compressor.

:::aside
Okay, full disclosure: "optimal parse" usually refers to [something else](https://cbloomrants.blogspot.com/2008/10/10-10-08-7_10.html) than described in this article, but I have no clue what else to call it, so optimal parse it is.
:::

Say you want to encode a byte stream, and bytes can be encoded in different formats, e.g. optimized for ASCII, numbers, raw binary, etc. These formats prioritize better compression of a specific type of data. Realistic byte streams may contain all of them at different points, so we want to switch between formats on the fly optimally.

The easiest way to do so is to split data into, say, 50-byte chunks, find the best format for each chunk, and use those formats. But that doesn't take into account that we need to store which format is used for which chunk, and choosing formats greedily may increase the size of this metadata because it itself compresses worse. Not to mention that it's not very precise. So what should we do?

Let's look at this problem from another angle. Let's make a table, where for each format we track how many bits it would take to encode each symbol with that format:

||a|b|r|a|c|a|d|a|b|r|a|
|-|-|-|-|-|-|-|-|-|-|-|-|
|Format&nbsp;1|6|3|4|6|5|6|9|6|3|4|6|
|Format&nbsp;2|7|2|6|7|4|7|3|7|2|6|7|
|Format&nbsp;3|3|7|8|3|6|3|8|3|7|8|3|

(Don't look too much into the numbers, they're chosen randomly. For example, I'm assuming the letter `a` always takes 7 bits to encode with format 2, but you can imagine fractional costs here, or use a measure other than bits, or chunk symbols.)

If switching formats was cheap, we could just choose the optimal format independently in each column, for instance:

<style>
td:has(strong) { background-color: var(--color-header-bg); color: #ffffff; }
</style>

||a|b|r|a|c|a|d|a|b|r|a|
|-|-|-|-|-|-|-|-|-|-|-|-|
|Format&nbsp;1|  6  |  3  |**4**|  6  |  5  |  6  |  9  |  6  |  3  |**4**|  6  |
|Format&nbsp;2|  7  |**2**|  6  |  7  |**4**|  7  |**3**|  7  |**2**|  6  |  7  |
|Format&nbsp;3|**3**|  7  |  8  |**3**|  6  |**3**|  8  |**3**|  7  |  8  |**3**|

But switching formats is not cheap. The effect is already visible even if it takes, let's say, just 2 bits -- the optimal selection changes to this one:

||a|b|r|a|c|a|d|a|b|r|a|
|-|-|-|-|-|-|-|-|-|-|-|-|
|Format&nbsp;1|  6  |**3**|**4**|  6  |  5  |  6  |  9  |  6  |**3**|**4**|  6  |
|Format&nbsp;2|  7  |  2  |  6  |  7  |  4  |  7  |**3**|  7  |  2  |  6  |  7  |
|Format&nbsp;3|**3**|  7  |  8  |**3**|**6**|**3**|  8  |**3**|  7  |  8  |**3**|

In this example, we switch for a short while to code `d` more optimally, but don't bother switching for `b`, since it's not as expensive in formats chosen for nearby symbols.

The question, then, is how to compute this "path" most efficiently.


### DP

The textbook approach to this problem is dynamic programming. For each cell in this table, we can iteratively compute the optimal path from that cell to the right side of the table, and then take the best path from the first column as optimal.

Here's how to do that. Each path starting at cell $(i, j)$ can be split into three parts:

1. Optionally switching the format to some other format $j'$ (costing $2$ bits), or keeping $j' = j$ (costing $0$ bits). This corresponds to moving within the column.
2. Coding the symbol at index $i$ with format $j'$, using the cost written in cell $(i, j')$. This corresponds to moving one cell to the right.
3. Coding the rest of the data, starting from cell $(i + 1, j')$.

In pseudocode (we'll switch to C once we get to low-level details),

```python
best_path_length[i, j] = min(
    (j != j_new) * 2 # (1)
    + cost[i, j_new] # (2)
    + best_path_length[i + 1, j_new] # (3)
    for j_new in range(n_formats)
)
```

Since each value `best_path_length[i, j]` only depends on values to the right of it, they can be computed starting from the right column and iterating backwards:

```python
# Initialize paths starting out-of-bounds as having cost 0.
for j in range(n_formats):
    best_path_length[n, j] = 0

# Compute the optimal paths starting at each cell.
for i in range(n_symbols - 1, -1, -1):
    for j in range(n_formats):
        best_path_length[i, j] = min(
            (j != j_new) * 2 # (1)
            + cost[i, j_new] # (2)
            + best_path_length[i + 1, j_new] # (3)
            for j_new in range(n_formats)
        )

# Infer the optimal path starting in the first column.
global_best_path_length = min(best_path_length[0, j] for j in range(n_formats))
print("Optimal path length:", global_best_path_length)
```


### Path

This computes the minimal possible cost, but not the specific path that produces it. To find the path, we can save the choice made at each cell and retrace our steps in the opposite direction:

```diff
 for i in range(n_symbols - 1, -1, -1):
     for j in range(n_formats):
-        best_path_length[i, j] = min(
+        (best_path_length[i, j], next_j[i, j]) = min(
+            (
                 (j != j_new) * 2 # (1)
                 + cost[i, j_new] # (2)
                 + best_path_length[i + 1, j_new] # (3)
+                , j_new
+            )
         )

+# Find the first optimal j (can also be fixed to some initial value, depending on specifics)
+j = min(range(n_formats), key = lambda j: best_path_length[0, j])
+
+# Record and recover next `j` column by column.
+for i in range(n_symbols):
+    j = next_j[i, j]
+    format[i] = j
```

If this doesn't make much sense, try to look at `next_j` as an intrusive linked list: `next_j[i, j]` represents the head of the linked list denoting the best path from $(i, j)$, and nodes are efficiently reused between multiple linked lists, CoW-style.

In this article, we'll pretend this part of the algorithm doesn't exist. Doing it justice requires writing another post, because there's plenty of subtleties here as well, but it'll have to wait until next time. (*Edit*: [here it is](../doubling-code-performance-with-a-useless-if/).)


### Optimization

For now, let's see how we can compute `best_path_length` most efficiently: this is a hot loop, and my compressor runs it multiple times per file, so this is crucial.

Let's start by refactoring the loop a little. Addends (2) and (3) don't depend on $j$, so they can be computed outside the nested loop:

```diff
 for i in range(n_symbols - 1, -1, -1):
+    tmp = [cost[i, j] + best_path_length[i + 1, j] for j in range(n_formats)]
     for j in range(n_formats):
         best_path_length[i, j] = min(
             (j != j_new) * 2
-            + cost[i, j_new]
-            + best_path_length[i + 1, j_new]
+            + tmp[j_new]
             for j_new in range(n_formats)
         )
```

Semantically, `tmp[j]` denotes the length of the shortest path starting at $(i, j)$, assuming that we can't switch formats on the first symbol.

<aside-start-here />

This reveals something interesting: if we decide to switch from $j$ to some $j'$, we should only ever switch to the format having the smallest `tmp[j']`, since switching formats takes the same cost regardless of where you're switching to:

```python
for i in range(n_symbols - 1, -1, -1):
    tmp = [cost[i, j] + best_path_length[i + 1, j] for j in range(n_formats)]
    best_j_new = min(range(n_formats), key = lambda j: tmp[j])
    for j in range(n_formats):
        best_path_length[i, j] = min(
            tmp[j], # don't switch
            2 + tmp[best_j_new] # switch to best_j_new
        )
```

:::aside
The constant switch cost is an assumption that may not hold in reality, but it produces good enough results that it's a reasonable approximation.
:::

With this optimization, we only have to consider two choices in the nested loop, not `n_formats` choices.


### Vectorization

And now we can get to the topic of this post.

Vectorization is, by far, the best way to speed up an algorithm so simple that it can't be optimized in any other way, like the one in question.

Most lines in the loop do the same thing for each $j$, so they can be vectorized quite easily. Switching to C, the loop can be rewritten with vectors like this:

```c
__m128i *cost; // assume costs are represented as an array of vectors: `cost[i]` is the column `i`

__m128i best_path_length[n_symbols + 1];
best_path_length[n_symbols] = _mm_setzero_si128();

for (int i = n_symbols - 1; i >= 0; i--) {
    __m128i tmp = _mm_add_epi16(cost[i], best_path_length[i + 1]);
    int min_tmp = /* the value of the minimal element in `tmp` */;
    best_path_length[i] = _mm_min_epu16(tmp, _mm_set1_epi16(2 + min_tmp));
}
```

You can handle fewer than $8$ formats by forcibly setting out-of-bounds costs to a large value, and more formats by repeating the logic for every $8$-format chunk.

The elephant in the room is the search for the minimal element. Global minimum is a *horizontal* operation, meaning that it merges values within a single vector, rather than corresponding elements of two vectors (which would be a *vertical* operation).

SIMD is not designed for fast horizontal operations, but in this scenario it's unavoidable. We can simulate horizontal minimum with a couple vertical operations:

<!-- Inline math seems to not support line wrapping. Embarrassing. -->

- Take the initial vector `(a0, a1, a2, a3, a4, a5, a6, a7)` and shuffle it to get `(a4, a5, a6, a7, x, x, x, x)` (where $x$ is "don't care").
- Take the vertical minimum of the original vector and the shuffled vector, producing `(min(a0, a4), min(a1, a5), min(a2, a6), min(a3, a7), x, x, x, x)`.
- Shuffle it to get `(min(a2, a6), min(a3, a7), x, x, x, x, x, x)` and apply vertical minimum again, producing `(min(a0, a4, a2, a6), min(a1, a5, a3, a7), x, x, x, x, x, x)`.
- Repeat the process one more time, producing `(min(a0, a4, a2, a6, a1, a5, a3, a7), x, x, x, x, x, x, x)`. The bottom word now contain the global minimum.

Computing the horizontal minimum of a $k$-word vector takes $\log_2 k$ steps, consisting of shuffling and vertical minimum each. Suffice to say, it's not exactly fast. (Though still better than scalar code, most of the time.)


### phminposuw

Which is where the star of this story comes in. *For whatever reason*, Intel has added exactly one fully horizontal instruction to their processors. It's called `phminposuw`, which stands for "**p**acked **h**orizontal **min**imum with **pos**ition for **u**nsigned **w**ords", but friends call her `_mm_minpos_epu16`.

Just to get across how bizarre this is:

- There is no instruction for horizontal addition, which would be a more obvious candidate. There is an instruction for horizontal *pairwise* addition, but nothing for summing up the entire vector at once -- that still requires $\log_2 k$ steps.
- There is no similar instruction for maximum -- that has to be simulated by inverting the input.
- There is no similar instruction for signed arithmetic -- that has to be simulated by flipping the sign bit.
- There is no similar instruction for 8-bit or 32-bit words.

The last two points are even more perplexing than it seems: `phminposuw` was added as part of SSE 4.1, the same ISA extension that fixed orthogonality for pre-existing *vertical* min/max instructions by adding the entire `_mm_{min,max}_ep{u,i}{8,16,32}` set, in addition to pre-existing `_mm_{min,max}_ep{u8,i16}` instructions.

As if that's not enough, `phminposuw` not only hasn't been extended to 256-bit SIMD registers when AVX came, even AVX-512 completely ignored it, despite improving orthogonality for every other operation. Oh, and NEON has nothing like it.

If this was any other instruction, it would surely forgotten and implemented in microcode, much like `loop`. But `phminposuw` has had a 4-cycle latency since Skylake and, on AMD, 3-cycle since Zen. For comparison, computing horizontal minimum in any other way would take 6 cycles, and that's without computing its position, which would take at least 5 more cycles.


### Code

Applying `_mm_minpos_epu16` gives us this:

```c
for (int i = n_symbols - 1; i >= 0; i--) {
    __m128i tmp = _mm_add_epi16(cost[i], best_path_length[i + 1]);
    int min_tmp = _mm_extract_epi16(_mm_minpos_epu16(tmp), 0);
    best_path_length[i] = _mm_min_epu16(tmp, _mm_set1_epi16(2 + min_tmp));
}
```

We can apply a couple other optimizations, like directly broadcasting the first word of `minpos` instead of extracting it:

```diff
 for (int i = n_symbols - 1; i >= 0; i--) {
     __m128i tmp = _mm_add_epi16(cost[i], best_path_length[i + 1]);
-    int min_tmp = _mm_extract_epi16(_mm_minpos_epu16(tmp), 0);
-    best_path_length[i] = _mm_min_epu16(tmp, _mm_set1_epi16(2 + min_tmp));
+    __m128i min_tmp = _mm_broadcastw_epi16(_mm_minpos_epu16(tmp));
+    best_path_length[i] = _mm_min_epu16(tmp, _mm_add_epi16(_mm_set1_epi16(2), min_tmp));
 }
```

You might also notice that we only ever access the previous value of `best_path_length` on each iteration, so it can just be a local instead of an array:

```diff
-__m128i best_path_length[n_symbols + 1];
-best_path_length[n_symbols] = _mm_setzero_si128();
+__m128i best_path_length = _mm_setzero_si128();

 for (int i = n_symbols - 1; i >= 0; i--) {
-    __m128i tmp = _mm_add_epi16(cost[i], best_path_length[i + 1]);
+    __m128i tmp = _mm_add_epi16(cost[i], best_path_length);
     __m128i min_tmp = _mm_broadcastw_epi16(_mm_minpos_epu16(tmp));
-    best_path_length[i] = _mm_min_epu16(tmp, _mm_add_epi16(_mm_set1_epi16(2), min_tmp));
+    best_path_length = _mm_min_epu16(tmp, _mm_add_epi16(_mm_set1_epi16(2), min_tmp));
 }
```

The body of this loop only has 8 ticks of latency, which I think is pretty cool.


### 16-bit costs

There is one glaring issue in this implementation: it assumes that all intermediate numbers fit in 16 bits, which they seldom do, because the numbers in columns grow when moving from right to left:

||a|b|r|a|c|a|d|a|b|r|a|
|-|-|-|-|-|-|-|-|-|-|-|-|
|Format&nbsp;1|51|45|42|38|34|29|24|18|12|9|5|0|
|Format&nbsp;2|52|46|44|38|33|29|22|19|13|11|5|0|
|Format&nbsp;3|50|47|44|36|33|27|24|17|14|11|3|0|

This example has small weights and a short input, but for realistic data, the counters often overflow and the logic breaks down. Let's fix that.

Subtracting a constant from each cell in a column does not change the optimal format selection, only decreases the total cost by that constant. We *could* choose the constant heuristically and pray, but there's a better way.

Nothing breaks if we use different constants for different columns, or even choose them in runtime. One obvious choice is to subtract `min_tmp` on each step: that guarantees that all costs stay non-negative and thus `_mm_minpos_epu16` works correctly (it assumes unsigned integers), while also limiting the values from above:

$$
\begin{align*}
\mathrm{len}'_j &= \min(\mathrm{tmp}_j, \min_k \mathrm{tmp}_k + 2) - \min_k \mathrm{tmp}_k \\
&\le 2 = \text{switch cost}.
\end{align*}
$$

```diff
 for (int i = n_symbols - 1; i >= 0; i--) {
     __m128i tmp = _mm_add_epi16(cost[i], best_path_length);
     __m128i min_tmp = _mm_broadcastw_epi16(_mm_minpos_epu16(tmp));
-    best_path_length = _mm_min_epu16(tmp, _mm_add_epi16(_mm_set1_epi16(2), min_tmp));
+    best_path_length = _mm_min_epu16(_mm_sub_epi16(tmp, min_tmp), _mm_set1_epi16(2));
 }
```

The latency exactly matches the previous loop.

The loop invariants work like this: `best_path_length` is limited by the switch cost, `tmp` is limited by the sum of the switch cost and the symbol cost, and after that the values are normalized back. As long as $\text{switch cost } + \text{max symbol cost } < 2^{16}$, the code works perfectly regardless of input length.


### Timings

The use of `phminposuw` brought my average end-to-end compression time down from $28$ ms to $22$ ms, which is around $20\%$. It's by far not the largest part of the compression pipeline, but it still made a noticeable dent and, together with other optimizations, let me run some brute-force analyses faster. I think that's cool.

Hope you enjoyed reading this post. As always, my code is open-source, and you can find it with more context [on GitHub](https://github.com/purplesyringa/computercraft-programs/blob/52982d2403e7bedd53c9c0c03de62d3e45b23a5e/initrd-ng/initrd-ng/src/entropy.rs#L77-L128).

---

Bonus chatter: I've recently been learning more stuff about compression from [ryg's blog](https://fgiesen.wordpress.com/). It's a goldmine of information. As chance would have it, I've also been listening to soundtracks by Farbrausch. Apparently ryg used to be part of it! It's a small world.
