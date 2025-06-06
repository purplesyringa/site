---
title: Recovering control flow structures without CFGs
time: Jun 6, 2025
intro: |
  I'm working on a Java decompiler because I'm not satisfied with the performance of other solutions. I've always heard that decompiling JVM bytecode is a solved problem, but I've concluded that the decompilation methods used by CFR and Vineflower are hacky, inefficient, and sometimes don't even work. The existing solutions are haphazard and inadequate compared to alternative approaches.

  Specifically, I have beef with the control flow extraction strategies employed by most decompilers. I haven't tackled decompilation *as a whole* yet, but I've found an approach to control flow recovery that works in isolation, is quite modular, and addresses common frustrations. I don't claim to be the first person to think of this method, but I haven't seen it mentioned anywhere, so this post describes it hoping that it's useful to someone else.
discussion:
  - https://www.reddit.com/r/programming/comments/1l4y5zm/recovering_control_flow_structures_without_cfgs/
  - https://lobste.rs/s/guet32/recovering_control_flow_structures
tikzThemes:
  light: |
    \colorlet{Arrow}{red!80}
    \colorlet{Text}{black}
  dark: |
    \definecolor{Arrow}{rgb}{.9,.3,.5}
    \colorlet{Text}{white}
---

I'm working on a Java decompiler because I'm not satisfied with the performance of other solutions. I've always heard that decompiling JVM bytecode is a solved problem, but I've concluded that the decompilation methods used by CFR and Vineflower are hacky, inefficient, and sometimes don't even work. The existing solutions are haphazard and inadequate compared to alternative approaches.

Specifically, I have beef with the control flow extraction strategies employed by most decompilers. I haven't tackled decompilation *as a whole* yet, but I've found an approach to control flow recovery that works in isolation, is quite modular, and addresses common frustrations. I don't claim to be the first person to think of this method, but I haven't seen it mentioned anywhere, so this post describes it hoping that it's useful to someone else.

> **Disclaimer**: The approach I describe is tuned for bytecode-based languages, such as Java and Python. There's a chance that it might apply to native code with modifications, but I have not researched that.


### Setting the scene

Most decompilers seem to follow one of these two control flow extraction strategies:

- They try to match bytecode to atomic structures like `if` or `while` directly, or
- They build a control flow graph and then locate atomic structures inside it.

Here are the problems with these approaches.


### Just match bytecode

Despite what people might tell you, javac *is* an optimizing compiler -- kind of, anyway. While it doesn't perform abstract execution, it can still thread jumps together and track reachability. Java bytecode rarely contains instructions `goto a; ... a: goto b`, by the magic of javac rewriting `goto a` to `goto b`. This means that

```rust
if (cond1) {
    if (cond2) {
        f();
    } else {
        g();
    }
} else {
    h();
}
```

is compiled to

```c
                if (!cond1) goto outer_else;
                if (!cond2) goto inner_else;
                f();
                goto outside;
inner_else:     g();
                goto outside;
outer_else:     h();
outside:
```

...and not the composition of two `if`s, which would look like

```c
                if (!cond1) goto outer_else;
                if (!cond2) goto inner_else;
                f();
                goto inner_outside;
inner_else:     g();
inner_outside:  goto outer_outside;
outer_else:     h();
outer_outside:
```

This gets even more complicated when you recall that constructs like `do { .. } while (false);` exist, which don't produce *any* bytecode, but enable unlimited forward `goto`s with multi-level `break`s.

Trust me, there's a *ton* of special cases in real-world Java bytecode. I've tried to write a decompiler based on this approach and it simply falls apart on complicated code.


### Just build a CFG

The CFG is effectively a high-level overview of the control flow situation, and the general approach here is to find simple patterns inside the graph. For example, an `if` conditional amoutns to a node splitting into two nodes, which are then reunited. No more javac-specific hard-coding! ...or that's how the story was supposed to go.

:::aside
To be clear: this is typically not a problem for *compilers*, as compilers have information about loops and usually only have to handle reducible control flow. This is why LLVM's [WASM stackification implementation](https://github.com/llvm/llvm-project/blob/8f7e57485ee73205e108d74abb5565d5c63beaca/llvm/lib/Target/WebAssembly/WebAssemblyCFGStackify.cpp#L434) is so short and fast. *Decompilers*, on the other hand, do not have such a luxury. If you're aware of an alternative algorithm that soundly handles *any* CFG in quasilinear time while at the same time producing high-quality code for reducible CFGs, please do share.
:::

CFG is messy. You can't recover constructs from inside out because an `if` might contain a `continue` and you'd have no idea how to handle such a split, as you don't even know if it's a `continue` or a different control flow construct yet. But if you attempt to parse the CFG in the opposite order, you'll quickly realize that you need to track connectivity and similar properties. This means that you have to re-run the analysis each time you find a new construct, and suddenly your decompiler is frustratingly slow.

<aside-inline-here />


### My idea

My approach is as follows. Using the CFG directly is a) slow because it's a graph without a simple structure, b) bad because it's unrelated to the original program order. Let's bring the order back and hope that linearizing the graph allows us to use faster algorithms.

Suppose that we have something that *looks* like an `if`. We can only translate it to a true `if` if there are no cross-jumps between the `then` and `else` branches. In a CFG, this is a connectivity query; in a linear order, this is a "is there an arrow starting in $[l_1; r_1]$ and ending in $[l_2; r_2]$" query, which is much easier to implement and optimize.

Of course, reading sequential bytecode, building a control flow graph, and then linearizing it back would be bollocks. If we can trust control flow not to be obfuscated, javac output is *already* a good instruction order, and our attempts to find a better one will likely produce worse code in the end. Not only does trusting javac provide a better result, but it's also closer to the source.

:::aside
Shut up, I know that AST is already "abstract", but *abstract* doesn't have many synonyms.
:::

This might sound like the "just match bytecode" approach, which I've just criticized, so let me explain the difference. The "just match bytecode" gang attempts to immediately match `if`, `while`, and other CF constructs. *My* goal is to build a very abstract AST, where nodes aren't labeled with any specific high-level constructs, and then refine this abstract AST to a concrete one. This closely mirrors the CFG approach but uses a tree rather than a graph.

<aside-inline-here />


### Abstract AST

We're now deep into the unexplored territory. A few papers come close, but there's nothing specifically like this, so let me introduce some terminology. You might be familiar with some of it from this work, which we'll build upon in a moment:

> Lyle Ramshaw. 1988. Eliminating go to's while preserving program structure. J. ACM 35, 4 (Oct. 1988), 893–920. https://doi.org/10.1145/48014.48021

An input program is a list of statements, where each statement is either an atom (i.e. an unsplittable instruction without any control flow) or a (possibly conditional) `goto`:

```rust
enum InputStatement {
    Atom(Atom),
    Goto(Condition, StatementId),
}
```

An output program is a list of statements: either an atom, or a labeled *block* containing other statements, or a (possibly conditional, possibly multi-level) `continue` or `break` from a block:

```rust
enum OutputStatement {
    Atom(Atom),
    Block(BlockId, Vec<OutputStatement>),
    Continue(Condition, BlockId),
    Break(Condition, BlockId),
}
```

When executed, a block executes all its children and exits. It does not form a loop, and it's not a functional control flow construct by itself, but it forms the basis of other constructs. Specifically, `break`ing a block jumps to the statement after the block, and `continue`ing a block jumps to the *beginning* of the block. For example,

```rust
while (x != 10) {
    x++;
}
```

can be implemented as

```rust
block #1 {
    if (x == 10) break #1;
    x++;
    continue #1;
}
```

Blocks kind of mirror CFG nodes, in that they contain statements and can be branched to, while `break`s and `continue`s are ways to simulate structured `goto`.

A *gap* is the space between consecutive statements in the input program. An *arrow* is an arrow connecting two gaps in a particular direction.


### Blocks

I'm using blocks instead of any other construct because they do not introduce any control flow that hasn't been present in the bytecode. I'm also not making a difference between blocks that only support `break` and blocks that only support `continue`, as the paper does: I acknowledge that real-world programs often have what amounts to both `continue` and `break` in the same loop, and that making this unnatural split is counterproductive.

Let's talk about the end goal. If we manage to produce a minimal (in some sense) abstract AST, we can *then* pattern-match it to well-known constructs. For example,

```rust
block #n {
    if (cond) break #n;
    <something not containing any mention of #n>
}
```

...is certainly just an `if`, and

```rust
block #n {
    if (cond) break #n;
    <statements...>
    <a divergent statement>
}
```

...is equivalent to a `while`. Note how tricky this second example is: we can parse a `while` *without hard-coding `continue #n` at the end*. `continue #n` is necessarily divergent, but so is an `if` statement with `continue #n` in one branch and `return` in the other one.

We'll get into the specifics more soon, but you can see that although it *seems* like we're just hard-coding control flow structures, the patterns we match are much more general than bytecode matching could ever achieve. By designing later passes carefully, you can gracefully recover *all* control flow structures from the original code, regardless of their complexity or code optimization.

Atypical blocks can be lowered to `while (true) { ... break; }`. *In reality*, this is a special case of the `while` lowering (or should I say upping?), but it's useful to understand that there's always a simple fallback if push comes to shove.


### Ramshaw's approach

To construct blocks, we'll first need to build an *arrow set*. An arrow set succinctly represents the structure of all jumps a program can take. The presence of an arrow $x \to y$ in the set indicates that *some* statement between the gaps $x$ and $y$ wants to jump to gap $y$. To build an arrow set, we use the targets of `goto`s as the heads and insert the shortest arrows containing the `goto` statements themselves.

Ramshaw's paper makes use of these two core facts:

1. If the arrows form a tree, i.e. any two intersecting arrows are nested, the arrows can be directly translated to blocks. If the arrow points forward, all jumps to its head can be implemented with a multi-level `break`, and if it points backward, `continue` does the trick.
2. The *tail* of any arrow can be arbitrarily extended without making any jumps unimplementable.

The paper explains precisely how to expand the arrows' tails to make them form a tree, but this is where we need to depart from the classics. This approach ignores that *not all arrows represent logical blocks*. `continue` and `break` statements will be parsed as arrows, and if those are treated equivalently to natural control flow construct arrows, the generated code will get chaotic *even if* you try to optimize them out after arrow extension. Specifically, consider the following two cases:

```tikz
% alt Both cases have two arrows with a tail-to-tail conflict: the left arrow points backward, while the right arrow points forward. In the first example, the left arrow is a continue and the right arrow is from an if statement. In the second example, the left arrow is from a while statement and the right arrow is a multi-level break.
\draw[very thick,<-,Arrow] (0,0) -- node[above,Text] {\large continue} (6,0);
\draw[very thick,->,Arrow] (3,-.8) -- node[above,Text] {\large if} (9,-.8);
\draw[very thick,<-,Arrow] (0,-2.5) -- node[above,Text] {\large while} (6,-2.5);
\draw[very thick,->,Arrow] (3,-3.3) -- node[above,Text] {\large multi-level break} (9,-3.3);
```

There's no *local* way to decide which arrow represents `continue`/`break` and which one represents a control flow construct, so at least in one of those cases, the arrow corresponding to the CF statement will have its tail extended, and thus the block won't be parsed as a high-level construct. This might make *that* arrow intersect another arrow not represented here, triggering more and more extensions like ripples in the water, which will be a nightmare to roll back if and when we decide the `break` and `continue` arrows can be optimized out.

This is where the Relooper and [Stackifier](https://medium.com/leaningtech/solving-the-structured-control-flow-problem-once-and-for-all-5123117b1ee2) approaches fundamentally get stuck. You can see from the linked article that Stackifier cannot restore good-quality high-level control flow because they attempt to create blocks that do not and *cannot* exist in the source.


### Alternative

Here's an alternative approach.

We construct the blocks from the outside. This allows us to ignore arrows that can be implemented with `continue` or `break`, as all the blocks that could have satisfied them have already been built. Here's specifically how we do that.

We define a recursive procedure $\mathrm{build}(l, r)$ that parses the statements in range $[l; r]$. We find *split gaps* inside this range, i.e. gaps that no arrows cross. If such gaps are present, we split the $[l; r]$ interval into smaller intervals and recurse. If no split gaps are present, either the $[l; r]$ range covers just one statement, which is trivial to handle, or there's multiple statements covered by the union of arrows.

In the latter case, we *have* to create a block covering the entire $[l; r]$ range: it can be proven that any smaller block would either conflict with arrows or not resolve any "topmost" arrow. Creating such a block allows us to satisfy all jumps to $l$ and $r$ from within the range with `continue` and `break`, respectively, and remove all the corresponding arrows from the arrow set.

In *most* cases, new split gaps will appear at this point, allowing us to recurse, and that's the whole algorithm.


### Irreducibility

But *sometimes*, no new split gaps appear. To be specific, as we haven't removed forward arrows from $l$ and backward arrows from $r$, no new gaps can appear in cases like this:

```tikz
% alt A couple of "chained" arrows, each one both starting after the previous one but also ending after the previous one. The first few chained arrows all point forward, and the rest point backward. Some other arrows may also be present.
\draw[very thick,->,Arrow] (0,0) -- (3,0);
\draw[very thick,->,Arrow] (2,-.5) -- (5,-.5);
\draw[very thick,->,Arrow] (4,-1) -- (7,-1);
\draw[very thick,<-,Arrow] (6,-1.5) -- (9,-1.5);
\draw[very thick,<-,Arrow] (8,-2) -- (11,-2);
\node[Text] at (5.5,-2.5) {\large $\dots$};
```

The core of the problem is the *head-to-head* collision. It closely resembles irreducible control flow from the CFG world. Ramshaw's paper proves that such conflicts cannot be resolved with `break`, `continue`, and blocks alone: no tail extension can make such arrows nest, but head extensions are impossible because they change the target of the jump... or are they?

There's a non-zero-cost approach to this: we can introduce a *dispatcher* at the start of the block, extend the backward arrow's head to point to the dispatcher, and then add an arrow from the dispatcher to the original head:

```tikz
% alt We start with a simple head-to-head collision. The forward arrow stays as is, while the backward arrow's head is extended to the very left such that the arrows now correctly nest. The new backward arrow is named "jump to dispatcher". A new arrow, called "dispatch", appears, starting at the leftmost position and continuing to the right to where the head of the backward arrow was.
\draw[very thick,->,Arrow] (0,0) -- (5,0);
\draw[very thick,<-,Arrow] (2,-.8) -- (7,-.8);
\node[Text] at (8.5,-.8) {\Huge $\Rightarrow$};
\draw[very thick,->,Arrow] (10,0) -- (15,0);
\draw[very thick,<-,Arrow] (10,-.8) -- node[above,Text] {\large jump to dispatcher} (17,-.8);
\draw[very thick,->,Arrow] (10,-1.6) -- node[above,Text] {\large dispatch} (12,-1.6);
```

The "jump to dispatcher" arrow sets a synthetic variable and emits `continue`. The dispatcher, which is a synthetic statement inserted at $l$, reads the synthetic variable and, if it's set, uses `break` to jump to the target location. For example,

```c
    if (cond) goto b;
a:  f();
b:  g();
    goto a;
```

...can be decompiled as:

```java
Target target = Target.Fallthrough;
while (true) {
    switch (target) {
    case Target.A:
        target = Target.Fallthrough;
        break;
    case Target.Fallthrough:
        if (!cond) {
            f();
        }
    }
    g();
    target = Target.A;
}
```

...and while it's somewhat ugly, a trivial dispatcher is at least a viable starting point. When you think about it, it's similar to how people implement finite automata in high-level languages: with a variable for the current state, `switch` over the state inside `while (true)`, and changing states implemented by modifying the variable and restarting the `switch`. Certain decompilers handle methods with irreducible control flow like this *globally*, but this approach allows us to localize the impact to the exact block that causes trouble.

*Anyway*, the main point here is that there's a simple way to fix this in terms of arrows. Find the first split gap inside $[l; r]$ *while ignoring all backward arrows*. Such a gap always exists, and is only an invalid gap due to all the backward arrows crossing it:

```tikz
% alt A couple of "chained" forward arrows, with the last one's head pointing to a vertical "gap" marker. A couple of backward arrows cross this marker. There are also a few random arrows to the right of the gap.
\draw[very thick,->,Arrow] (0,0) -- (3,0);
\draw[very thick,->,Arrow] (2,-.5) -- (5,-.5);
\draw[very thick,->,Arrow] (4,-1) -- (7,-1);
\draw[very thick,<-,Arrow] (6,-1.5) -- (10,-1.5);
\draw[very thick,<-,Arrow] (4,-2) -- (8,-2);
\draw[very thick,<-,Arrow] (1,-2.5) -- (9,-2.5);
\draw[very thick,<-,Arrow] (10,0) -- (11,0);
\draw[very thick,->,Arrow] (9,-.5) -- (12,-.5);
\draw[very thick,Text] (7,.5) node[above] {\large gap} -- (7,-3);
```

Create new dispatch arrows from $l$ to the backward arrows' heads -- they will all be to the left of the gap -- and satisfy the backward arrows with an assignment to the selector and a `continue`. This makes the gap valid and recursion possible:

```tikz
% alt All backward arrows have been replaced with forward arrows, starting at the very left and ending at the same position as before.
\draw[very thick,->,Arrow] (0,0) -- (3,0);
\draw[very thick,->,Arrow] (2,-.5) -- (5,-.5);
\draw[very thick,->,Arrow] (4,-1) -- (7,-1);
\draw[very thick,->,Arrow] (0,-1.5) -- (6,-1.5);
\draw[very thick,->,Arrow] (0,-2) -- (4,-2);
\draw[very thick,->,Arrow] (0,-2.5) -- (1,-2.5);
\draw[very thick,<-,Arrow] (10,0) -- (11,0);
\draw[very thick,->,Arrow] (9,-.5) -- (12,-.5);
\draw[very thick,Text] (7,.5) node[above] {\large gap} -- (7,-3);
```

To reiterate, dispatchers are only necessary as a fallback for complicated control flow. javac does not (typically) generate such code, and handling this case is mostly only required for correctness and completeness, as JVM bytecode doesn't forbid it.


### In a nutshell

Phew. Here's a bird's eye view of what we've done.

1. Given a range $[l; r]$ and an arrow set, we searched for gaps in $(l; r)$ that weren't covered by the arrows. We split $[l; r]$ at such gaps and recursed.

2. We then created a block for $[l; r]$, marked all backward arrows to $l$ and forward arrows to $r$ as satisfied, and removed them from the arrow set. (We don't necessarily have to check if they start within the range, as all arrows covering the range have already been resolved; but we do have to only filter for *backward* arrows to $l$ and *forward* arrow to $r$, because there can be arrows pointing in the other direction from outside the range.)

3. We then found the leftmost split gap taking only forward arrows into account, found all backward arrows crossing this gap (again, such arrows are necessarily within the block; there can also be no such arrows, and that's fine too), marked them as satisfied with a jump to dispatch, and removed them from the arrow set. We added forward dispatch arrows to the arrow set. After that, we recursed, as we know there's now a valid split gap.

A key property of the resulting AST is that each block has at least one arrow that couldn't be satisfied by any other block, making it minimal in a certain sense. Each jump is satisfied by the outermost valid block, not the innermost, as your passes might expect, but this is a *good* thing. As javac threads `goto`s, javac can easily implement a `break` from an internal loop as a `continue` of an outer loop, and the nested `if` example from the beginning of the post still applies, so forcing the outermost block as the target reduces the number of cases that must be handled.

The blocks are also minimal in the sense that either the first statement of the block uses `break` or someone inside the block uses `continue`, and that either the last statement uses `continue` or someone inside the loop uses `break`. This means that, among other things, a block without `continue` will necessarily *start* with an `if (...) break;`, indicating that this block can be implemented with an `if`; and vice versa, an `if` can only be parsed as a block *starting* with `if (...) break;`, not merely *containing* such a statement somewhere in the middle. This makes patterns easy to enumerate.


### Implementation

At the beginning of the post, I mentioned that CFG-style approaches are not only sometimes bad at code generation but also slow, so it's time to explain how to improve time complexity of this approach.

First, we introduce a data structure over arrows that supports the following queries in logarithmic time:

- Add arrow $x \to y$.
- Remove arrow $x \to y$.
- Find the first split gap in range $[l; r]$.

It's quite simple -- a [segment tree](https://cp-algorithms.com/data_structures/segment_tree.html) (the one supporting range queries, not from computational geometry) over gaps suffices:

- Adding an arrow can be implemented by incrementing the counters corresponding to all gaps in a certain range (specifically, $(\min(x, y), \max(x, y))$), marking them as blocked from splitting by the arrow.
- Removing an arrow can be implemented by decrementing the counters over the same range.
- Finding a split gap amounts to finding the first $0$ in the range, which can be implemented by storing the minimum and its position and verifying that the minimum is $0$.

This can be used to split the $[l; r]$ range by gaps and to find the first split gap over forward arrows only.

The search for backward arrows crossing a given gap can be straightforwardly implemented with an [interval tree](https://en.wikipedia.org/wiki/Interval_tree).

Finally, simple arrays can be used to find forward and backward arrows ending at a certain point.


### Conclusion

So. That's how you implement a high-quality control flow restoration algorithm with quasilinear performance. Q&A:

**Why does decompilation performance matter this much to you?**

Because I have a use case that requires a fast and slim Java decompiler, and so I started doing some research, and suddenly three weeks have already passed, and this is where I found myself.

**Why no realistic samples?**

Because you need a ton of passes *after* control flow structuring to make the code somewhat readable. This is just one part of the process, and while its implementation is interesting from a technical point of view, the results themselves are boring. I'll draw the rest of the fucking owl someday, but it's harder than it seems.

**Why not wait until you get further results?**

Because I might stumble upon some other problem I don't know how to resolve and not finish the project. This is my way of sharing knowledge while I still can.

**If I understand the above, can I make a Java decompiler?**

Maybe, but don't hold your breath. This is just the beginning -- you still need to write passes to convert blocks to `if`s and `while`s, you need to handle complicated control flow with `&&` and `||` in `if`/`while` conditions, you need to handle exception handlers and `synchronized` monitors, and so on. Decompiling Java is *not* simple at all unless you don't care about correctness, efficiency, and code quality, which... many decompilers don't, and that's valid, but it's not something I'd be proud of.

**Can this be applied to other languages?**

If we're talking about bytecode-based languages, then yes, the core of the work should be applicable as is. For native code, it's probably not that easy, as the compiler often reorders code for efficiency. This is somewhat a problem for obfuscated JVM bytecode as well -- it's just out of scope for me at the moment -- but you'll likely get acceptable results if you reorder statements according to DFS order and a few heuristics. No guarantees, though.
