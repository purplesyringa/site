---
title: Thoughts on "SIMD in Pure Python"
time: July 26, 2026
intro: |
  I found the post [SIMD in Pure Python](https://www.da.vidbuchanan.co.uk/blog/python-swar.html) by *retr0id* about optimizing Game of Life and thought it's a good opportunity to yap about SWAR. Make sure to read that post before continuing here! Or at least skim it.
---

I found the post [SIMD in Pure Python](https://www.da.vidbuchanan.co.uk/blog/python-swar.html) by *retr0id* about optimizing Game of Life and thought it's a good opportunity to yap about SWAR. Make sure to read that post before continuing here! Or at least skim it.

---

In the post, the author computes Game of Life with SWAR by storing each cell as a 0 or 1 in a 4-bit field. This allows him to quickly sum up 9 neighbors to get a 0-9 in a 4-bit field. At that point, he wants to convert the number back to a 0 or 1 depending on whether the count is exactly 3 (or 4, or 7).

The approach in the post takes 5 operations: one XOR with a broadcast `~3`, and then two halving steps for horizontal AND. `3 ^ ~3 = 15`, where all bits are set, resulting in a 1; any other value XORed with `~3` will have a zero bit, resulting in a 0. Makes sense.

There's a better way to achieve the same result, though. We can rewrite `(x ^ 3) == 0` as `(x ^ 3) < 1`, and that as `(x ^ 3) + 15 < 16`. And for 5-bit values, `y < 16` can be computed simply by checking if the top bit of `y` is `0`. So with an appropriate bit shift at the end, the formula ends up being pretty much just:

```python
has_3_neighbors = (summed ^ MASK_3) + MASK_15
```

The idea behind this trick is that by adding a constant, we move the cut-off point for `<` to a nice power-of-two value. This requires allocating 5 bits per cell (rather than 4) to avoid cross-carrying, but it more than halves the opcount, so it's most likely still faster in the end.

---

Given that the author ends up using a 4-bit paletted SDL mode, a question remains: is a speed-up possible without increasing bit count? It turns out that the answer is yes!

Instead of computing `x == 3`, let's focus on `x >= 3` for now. That's simple: it's equivalent to `x + 5 >= 8`, and *this* time `9 + 5` fits in 4 bits, so there's no overflow. In a certain sense, the formula is simply

```python
has_at_least_3_neighbors = summed + MASK_5
```

...with the answer stored in the top bit. Similarly, we can compute

```python
has_at_least_4_neighbors = summed + MASK_4
has_at_least_5_neighbors = summed + MASK_4
has_at_least_7_neighbors = summed + MASK_1
has_at_least_8_neighbors = summed # free!
```

...and then merge them as necessary, e.g.

```python
has_3_neighbors = has_at_least_3_neighbors & ~has_at_least_4_neighbors
has_4_neighbors = has_at_least_4_neighbors & ~has_at_least_5_neighbors
has_7_neighbors = has_at_least_7_neighbors & ~has_at_least_8_neighbors
```

Oh, and you don't need all the negations, e.g.:

```python
~has_at_least_5_neighbors = -has_at_least_5_neighbors - 1 = ~MASK_4 - summed # one op, not two
```

Compared to the original 15 operations, this is either 8 or 6, depending on how you count `&~`. Not as crazy of an improvement as in the 5-bit case, but still cool.

---

I'd love to tell you where these tricks originated from, but I don't know! The earliest mention I could find is [this Usenet post back in 1987](https://groups.google.com/forum/#!original/comp.lang.c/2HtQXvg7iKc/xOJeipH6KLMJ) by Alan Mycroft. If any digital archaeologist can figure out if this was known earlier, I'd love to amend the post.

And if you're looking for a practical application of these tricks, check out [my earlier post on `serde_json`](../i-sped-up-serde-json-strings-by-20-percent/).
