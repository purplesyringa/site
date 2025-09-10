---
title: Fenwick layout for interval trees
time: September 10, 2025
intro: |
    Fenwick trees and interval trees are well-known data structures in computer science. Interval trees in particular are commonly used in bioinformatics and computational geometry, and Fenwick trees are useful for keeping statistics.

    This post describes how the two can be merged to obtain a worst-case faster implementation of an interval tree. This approach is probably not very useful in these areas due to their specific requirements, but it's still a well-rounded general-purpose implementation, and it's so neat and short I can't help but share it.

    **TL;DR** if you're familiar with the topics: You can index the nodes of an interval tree by their unique center and use fast Fenwick-like navigation over the tree. A sample implementation is available [on GitHub](https://github.com/purplesyringa/brickingot/blob/baa3cab1e9686e7209a291a58f90f1810cbbdce0/src/interval_tree.rs).

    The rest of the post describes what this means in more detail. It assumes familiarity with DSA, but not necessarily with the data structures mentioned above.
---

Fenwick trees and interval trees are well-known data structures in computer science. Interval trees in particular are commonly used in bioinformatics and computational geometry, and Fenwick trees are useful for keeping statistics.

This post describes how the two can be merged to obtain a worst-case faster implementation of an interval tree. This approach is probably not very useful in these areas due to their specific requirements, but it's still a well-rounded general-purpose implementation, and it's so neat and short I can't help but share it.

**TL;DR** if you're familiar with the topics: You can index the nodes of an interval tree by their unique center and use fast Fenwick-like navigation over the tree. A sample implementation is available [on GitHub](https://github.com/purplesyringa/brickingot/blob/baa3cab1e9686e7209a291a58f90f1810cbbdce0/src/interval_tree.rs).

The rest of the post describes what this means in more detail. It assumes familiarity with DSA, but not necessarily with the data structures mentioned above.


## Interval trees

### Primer

[Interval trees](https://en.wikipedia.org/wiki/Interval_tree) are a data structure that stores intervals (for the purposes of this article, we'll use half-open ranges $[a; b)$, where $0 \le a < b \le M$) and can answer queries of kind "which intervals contain the given point $x$?" efficiently. Interval trees can be built in $\mathcal{O}(n \log n)$ time, answer queries in $\mathcal{O}(\log \min(n, M) + \mathrm{ans})$ time, and use $\mathcal{O}(n)$ space.

Interval trees are based on binary trees. The root node covers the full range $[0; M)$ and is split at some central point $m$ (not necessarily exactly in the middle) into two children, covering $[0; m)$ and $[m; M)$, and this process is repeated recursively. Each node stores a list of intervals. The lists are populated during structure initialization, and each interval is placed in the topmost node whose center $m$ is within the range $[a; b)$. Nodes are created lazily, and empty subtrees are omitted.

This can be implemented in a top-down approach, finding the correct node iteratively: if $b \le m$, we enter the left child; if $a > m$, we enter the right child; otherwise, $a \le m < b$ and we add the interval into the current node.

Since all intervals of a node are located within the node range, all intervals containing a given point $x$ must be in one of the $\mathcal{O}(\log n)$ nodes covering $x$, which can be found iteratively. But how does this help us -- aren't we back to square one?

Not quite. Since the intervals within each node contain $m$, we know that $a \le m < b$ holds for these nodes. If, say, $x < m$, this means that $x < b$ holds implicitly, and we only need to filter the intervals by $a \le x$. Similarly, if $x > m$, we only need to filter by $x < b$, and if $x = m$, every single interval works. Filtering by a single condition is much easier than by two, since we can just build two lists: one sorted by $a$ and another sorted by $b$.

Since we want to emit each matching interval, we don't need to run binary search over the lists: we can greedily emit all matching intervals by iterating over lists in linear order and stopping at the first non-matching one.


### Applications

Interval trees have many modifications, e.g. allowing for querying all intervals that intersect a query interval, supporting higher-dimensional spaces, or allowing insertion or deletion on the fly by replacing sorted arrays with binary trees.

I've mentioned areas that typically use this data structure above, but I'd like to add some unorthodox uses.

The first one is something I've posted about earlier: I'm using interval trees as a helper data structure to [decompile control flow](../recovering-control-flow-structures-without-cfgs/) without a CFG with a subquadratic worst-case complexity.

:::aside
This problem can also be solved without auxiliary data structures. Can you figure out the solution?
:::

For the second one, here's a party trick. Suppose you have a graph $(V = \{0, 1, \dots, n - 1\}, E)$ where edges map not from a single vertex to a single vertex, but from a vertex range $[a; b)$ to a single vertex $v$. If you want to run DFS over such a graph without making it quadratic, store the edges in the interval tree, and, when looking for neighbours of $u$, look up $v$s corresponding to intervals containing $u$. Then remove such intervals from the interval tree, so that $\sum \mathrm{ans} = \mathcal{O}(E)$, and the total time complexity becomes $\mathcal{O}(V \log V + E)$. You don't need a complicated implementation to handle removal: just drop the matching prefix/suffix from the index after emitting it.

<aside-inline-here />


### Implementation

Basically any balanced tree can underlie the interval tree. But that's on paper. Real-world implementations need to deal with the constant factor, cache friendliness, space use, branch prediction, etc. Typically, the simpler balanced tree you use, the better performance you're going to get. And AVL is not simple, which put me on this path.


## Segment trees

### Primer

Enter [segment trees](https://cp-algorithms.com/data_structures/segment_tree.html). The name is similar to "interval trees", but don't let that fool you. The segment tree is a distinct, classical data structure, though it's mostly unknown outside of competitive programming. Segment trees are used to optimize range queries on arrays, e.g. range sum query or [range minimum query](https://en.wikipedia.org/wiki/Range_minimum_query), which is useful for cumulative histograms, various tree algorithms, and some ad-hoc problems.

The core idea is that for dense data, it's faster to use a static balanced tree layout than a self-balancing tree. Static trees are worse at adapting to circumstances, but if the circumstances are pre-determined, balancing mechanisms are a dead weight, even if they are asymptotically insignificant.

The data structure is a [perfect binary tree](https://en.wikipedia.org/wiki/Binary_tree#perfect) (meaning the array size is effectively rounded up to the nearest power of two), where each node is split exactly at its center $m = \frac{l + r}{2}$. Nodes are never added or removed, only traversed. Each leaf node stores a single element of the array, and indices are not explicitly stored. Queries are handled as with normal self-balancing trees.

The implication is that the nodes of a perfect binary tree can be stored in the [Eytzinger layout](https://en.algorithmica.org/hpc/data-structures/binary-search/#eytzinger-layout), which allows child indices to be *computed* from the parent index without having to store child pointers. (The two children of node $v$ are defined as $2v$ and $2v + 1$, with node $v = 1$ being the root.) This saves both on space *and* access performance *and* cache friendliness, as we don't have to read memory to obtain the child index and the top levels of the tree are all close to the beginning of the node array.

When combined, the benefits are so high that this works better than any kind of self-balancing tree even for not very dense data. (This can also be adapted to allow more than 2 children per node to benefit from SIMD; wide segment trees are to segment trees as B-trees are to binary trees.)


### Bottom-up

Before we move on, I want to highlight one last feature of segment trees. Since the layout is pre-determined, we can immediately compute the index of node $[x; x + 1)$ as $v = n + x$, where $k$ is the depth of the tree. We can also compute the parent of a given node as $\lfloor \frac{v}{2} \rfloor$. This allows us to iterate through nodes containing $x$ in the bottom-up order rather than the typical top-down order, thus saving on comparisons/branches typically required to decide which child to recurse into.

But wait, I thought we were going to use Fenwick trees rather than segment trees as the basis?


## Fenwick trees

### Primer

While seldom defined this way, [Fenwick trees](https://en.wikipedia.org/wiki/Fenwick_tree) are actually an optimized variation of segment trees. (The only source I'm aware of that highlights this fact is [this paper](https://byorgey.github.io/blog/posts/2025/01/23/Fenwick.html) by Brent Yorgey -- shout out to them).

The idea of Fenwick trees is that, as long as queries are only performed on prefixes, nodes that are right children of their parents are never accessed (but may still be traversed). This means two things: a) we can remove half of the nodes, saving space, b) each node has a distinct right coordinate. This means that the end of the node can be used as the index of the node.

Typically, when implementing segment trees, you need to store both the current node index and its range, but if the two are one and the same, you can significantly simplify formulas. It turns out that the bit representation of a node index stores a lot of information: the end coordinates of present leaves always end with $1$, their parents end with $10$, the 2nd level parents end with $100$, and so on. This allows you to easily compute the depth of the node just from its index.

In a similar manner, you can compute the index of the closest present ancestor or the left child of a node just by placing or removing some bits with simple bit tricks, which results in a very fast implementation of segment trees. The specifics don't matter for the purposes of this post, so I'll omit them, but you can look up "Fenwick tree" online if you're interested.


### Cache

It's important to note that Fenwick trees have a fatal flaw compared to segment trees: their topmost nodes are stored far from each other, so they are less cache-friendly for medium sizes than segment trees. It's still worth it in the end, since there's twice as little data to store and the algorithms are simpler, but it's worth mentioning. They are also slower than SIMD-optimized wide segment trees, but that should be obvious.


## Better interval trees

### Design

The specific idea we're borrowing from Fenwick trees is that by dropping unnecessary nodes, we can drop auxiliary indices in favor of their unique properties. For interval trees, such a unique property is the node center $m$: if two nodes share the same center, only the topmost node should be retained.

Assume for the moment that $M$ is rounded to the nearest power of two. The root node has index $M / 2$, i.e. $100 \dots 00$ in binary form. Its children have indices $010 \dots 00$ and $110 \dots 00$, and so on: the number of trailing zeros corresponds to the height of the node.

Let's solve two parts of the problem individually. A query for some point $x$ iterates over all nodes containing $x$, starting from the root and stopping at the node with $m = x$. We can do this the other way round: start at node $x$ and move upward.

A node $abcd100\dots0$ has a height as per the number of zeroes, and so its range is $[abcd000\dots0; abcd111\dots1]$. The parent of this node thus has range $[abc0000\dots0; abc1111\dots1]$, and its center is $abc1000\dots0$. So computing the parent of a node amounts to finding the lowest set bit, resetting it and setting the bit above it.

This means that querying for intervals containing a given point $x$ can be implemented as:

```rust
let m = x;
while m != M / 2 {
    // filter intervals stored in `nodes[m]`, assuming the center is `m`
    m = (m & (m - 1)) | (2 << m.trailing_zeros());
}
```

To build the tree, we need to find the topmost node whose center is within $[l; r)$, i.e. a value $m$ with as many trailing zeros as possible such that $l \le m < r$. This can be implemented by taking the maximal possible index $m = r - 1$ and resetting the bottommost bits while $m \ge l$ holds. But there's a constant-time way to achieve the same goal.

Consider the bits of $l - 1$ and $r - 1$. A few topmost bits (denote this prefix by $p$) might match, and then there will be a position $i$ where $l - 1$ contains $0$ and $r - 1$ contains $1$. We know that $m = p100\dots00$ would work, since $l - 1 < m$ (because $p$ matches and the bit after that is larger in $m$) and $m \le r - 1$ (since $p1$ is a common prefix and $0$ is less than or equal to anything), so $m$ is a valid choice. But this $m$ is also an optimal choice, since resetting the bit $i$ is guaranteed to produce a number $m' = p000\dots00 \le l - 1 < l$.

The resulting code for inserting an interval is:

```rust
let m = (r - 1) & (usize::MAX << ((l - 1) ^ (r - 1)).ilog2());
nodes[m].add(l, r);
```


### Special cases

There's a catch: the formulas above don't work for $m = 0$ and $l = 0$ or $r = 1$, respectively, since some assumptions break down. Luckily, there's a clean solution. By implicitly incrementing each coordinate by $1$ when computing node indices, we get the code to work for all inputs with few modifications, and it even becomes simpler as a side effect:

```rust
let m = x;
while m != M / 2 - 1 {
    // filter intervals stored in `nodes[m + 1]`, assuming the center is `m`
    m = (m | (m + 1)) & !(2 << m.trailing_ones());
}
```

```rust
let m = r & (usize::MAX << (l ^ r).ilog2());
nodes[m].add(l, r);
```

Now that we recognize that node $0$ is never accessed, we can shift all nodes by $1$ and remove some $+ 1$ shenanigans:

```rust
let m = x;
while m != M / 2 - 1 {
    // filter intervals stored in `nodes[m]`, assuming the center is `m`
    m = (m | (m + 1)) & !(2 << m.trailing_ones());
}
```

```rust
let m = (r & (usize::MAX << (l ^ r).ilog2())) - 1;
nodes[m].add(l, r);
```


### Power of two

One last trick: while we have to round up sizes to the next power of two for tree layouting to work correctly, this doesn't mean that all $M = 2^k$ nodes are non-empty. If every coordinate is $\le M'$, then nodes with indices $\ge M'$ are guaranteed to be empty and thus don't have to be stored; so there only need to be as many nodes as there are coordinates. Note that out-of-bounds nodes may be *accessed* still, just not populated.


### Implementation

My implementation is available [on GitHub](https://github.com/purplesyringa/brickingot/blob/baa3cab1e9686e7209a291a58f90f1810cbbdce0/src/interval_tree.rs).

It's pretty unoptimized: `Vec`s should be collapsed into a single allocation, sorting should only be run twice or perhaps not at all, the iteration logic can be improved, etc. -- but the layout is there. Benchmarking is left as an exercise to the reader; that would be a nice addition, but there are only so many hours in a day.

I researched this for fun, since this isn't a bottleneck on realistic data in my scenario, and I only need a fast-ish implementation to let code scale predictably in the worst case. I find this focus on medium to large data compared to micro-optimizations I typically deal with refreshing; it helps prevent my skills from getting too rusty.

Overall, this blend between data structures seems to be unknown, or at least I couldn't find it described anywhere. If you find something similar described in a paper, I'm all ears! Until then, bye, and good luck on your projects.
