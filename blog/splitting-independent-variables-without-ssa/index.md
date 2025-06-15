---
title: Splitting independent variables without SSA
time: Jun 15, 2025
intro: |
    I'm making progress on the Java decompiler I've mentioned [in a previous post](../recovering-control-flow-structures-without-cfgs/), and I want to share the next couple of tricks I'm using to speed it up.

    Java bytecode is a stack-based language, and so data flow is a bit cursed, especially when the control flow is complicated. I need to analyze data flow globally for expression inlining and some other stuff. [Single-static assignment](https://en.wikipedia.org/wiki/Static_single-assignment_form) produces basically everything I need as a byproduct... but it's not very fast.

    For one thing, it typically mutates the IR instead of returning data separately, and the resulting IR has imperative code mixed with functional code, which is a little unpleasant to work with. SSA has [multiple implementations](https://bernsteinbear.com/blog/ssa/) with very different performance characteristics and conditions, and each of them forces me to make a tradeoff I'm not positive about.
---

I'm making progress on the Java decompiler I've mentioned [in a previous post](../recovering-control-flow-structures-without-cfgs/), and I want to share the next couple of tricks I'm using to speed it up.

Java bytecode is a stack-based language, and so data flow is a bit cursed, especially when the control flow is complicated. I need to analyze data flow globally for expression inlining and some other stuff. [Single-static assignment](https://en.wikipedia.org/wiki/Static_single-assignment_form) produces basically everything I need as a byproduct... but it's not very fast.

For one thing, it typically mutates the IR instead of returning data separately, and the resulting IR has imperative code mixed with functional code, which is a little unpleasant to work with. SSA has [multiple implementations](https://bernsteinbear.com/blog/ssa/) with very different performance characteristics and conditions, and each of them forces me to make a tradeoff I'm not positive about.

- [Cytron's SSA](https://bernsteinbear.com/assets/img/cytron-ssa.pdf) needs to precompute dominance frontiers which I don't need for any other pass,
- [Brandis and Mössenböck's SSA](https://bernsteinbear.com/assets/img/brandis-single-pass.pdf) only works on structured control flow graphs,
- [Sreedhar and Gao's SSA](https://bernsteinbear.com/assets/img/sreedhar-ssa.pdf) is just... *complex* and uses a custom data structure,
- [Aycock and Horspool's SSA](https://bernsteinbear.com/assets/img/aycock-horspool-ssa.pdf) has questionable performance characteristics, not least because it inserts $\phi$ functions *everywhere*, wasting a ton of memory, and then removes them iteratively,
- [Bebenita's SSA](https://bernsteinbear.com/assets/img/bebenita-ssa.pdf) is intuitive and only relies on the [union-find](https://en.wikipedia.org/wiki/Disjoint-set_data_structure) data structure, but has a fixed time complexity and doesn't provide for any special-case optimizations,
- etc.

SSA is not a bad choice by any means, but I was thinking that maybe I could integrate ideas from SSA implementations into algorithms my decompiler actually needs, without computing SSA itself.


### Motivation

Java decompilation involves translating JVM's stack operations to Java's operations on variables. This is quite simple: by mapping the stack element at height $n$ to variable $a_n$, we ensure that each instruction maps to one assignment. For example:

```java
push(1);
push(2);
add();
// translates to:
a0 = 1;
a1 = 2;
a0 = a0 + a1;
```

To collapse such long chains into simpler ones, like `a0 = 1 + 2`, I need to track, for each stack read, which write produced the corresponding value. This seems very easy until you realize that branching exists:

```java
if (cond) {
    push(1);
} else {
    push(2);
}
push(3);
add();
// translates to:
if (cond) {
    a0 = 1;
} else {
    a0 = 2;
}
a1 = 3;
a0 = a0 + a1; // where does a0 in RHS come from here?
```

If you wanted to keep things simple, you'd recurse into the preceding basic blocks and merge the sources of `a0` they return, but that has horrible time complexity and quickly gets out of hand.


### Independence

There isn't a way to efficiently track, for each use, *precisely* which definitions that use can see, without SSA. But I don't need that kind of precision: I'm not writing an optimizing compiler that would benefit from such analysis. I can make do with two pieces of information:

- I want to track whether a given use comes from a *single* definition, and if true, precisely which definition it is, and
- I want to split variables that have multiple non-interfering live ranges.

The former is necessary for inlining expressions. The latter would benefit from an example. Suppose that I have code like this:

```java
a0 = f();
g(a0, a0);
a0 = h();
i(a0, a0);
```

I obviously can't inline `f()` into `g(...)`, so I have to retain `a0` as a variable. But as `f()` and `h()` can have different return types, I want the first two and the last two lines to access different variables, e.g.:

```java
a0v1 = f();
g(a0v1, a0v1);
a0v2 = h();
i(a0v2, a0v2);
```

The key idea is: all definitions visible to a given use need to access the same instance of the variable. A straightforward implementation would iterate over uses, query which definitions each use sees, and then merge them via [union-find](https://en.wikipedia.org/wiki/Disjoint-set_data_structure). The individual components would map to independent variables.


### Approach

Let's discuss how to do this enumeration and querying efficiently.

All def-use chains within a single basic block can be resolved statically, perhaps even while populating the basic block. We're left with tracking def-use chains across basic blocks.

Consider a graph whose vertices are (basic block, variable name) pairs. Entering vertex `(bb, var)` indicates that we're interested in finding definitions of `var` that are visible on entry to `bb`. For each predecessor `pred` of `bb`, we can either find the definition of `var` within `bb`, or recurse to `(pred, var)`. The algorithm starts with a DFS over such a graph.

Here's how I accumulate the located definitions. We create a node in union-find for each mention of a variable, i.e. for both uses and definitions. For each use `use` of `var` that doesn't have a matching definition in its basic block `bb`, we enter the vertex `(bb, var)` with the request for the answer to be merged into the component `use` of union-find. This "output" argument is propagated recursively and cached as the answer for each visited vertex. Whenever any given vertex `(bb, var)` is entered for the second time, the current and the cached output arguments are merged, and recursive traversal is skipped.

The implementation via iterative DFS fits on a napkin:

```rust
// We could just store `bb` on the stack, but `var` and `use` will come in handy a bit later.
stack.push((bb, var, use_));

while let Some((bb, var, use_)) = stack.pop() {
    match cached.entry((bb, var)) {
        Entry::Occupied(entry) => {
            union_find.merge(use_, *entry.get());
            continue;
        }
        Entry::Vacant(entry) => entry.insert(use_),
    }

    for pred in &predecessors[bb] {
        if let Some(def) = active_defs_at_end[pred].get(&var) {
            union_find.merge(use_, def);
        } else {
            stack.push((pred, var, use_));
        }
    }
}
```

Note that we save `use` into cache before recursing. This allows the algorithm to work correctly on cyclic CFGs: even though vertices of a strongly connected component may refer to different node IDs, the nodes will correspond to the same component in union-find.

The worst-case time complexity is $\mathcal{O}(n_{\text{basic blocks}} \cdot n_{\text{vars}} \cdot \alpha(n_\text{uses}) + n_\text{uses})$. This isn't even quasilinear, but it does match the usual time complexity of SSA algorithms, so it's not the end of the world. (Yes, some SSA algorithms don't have the $\alpha$ factor, but union-find is fast enough that this algorithm is faster than linear SSA in practice.)

However, the important part is that the big-O constant is quite low because it's a single DFS. Moreover, this bound can be tightened in some common cases, because only basic blocks lying along the paths between the uses and the definitions are visited. This means that, for instance, that in code structured like

```java
var0 = ...;
// <bb boundary>
f(var0);
// <bb boundary>
var1 = ...;
// <bb boundary>
f(var1);
// <bb boundary>
var2 = ...;
// <bb boundary>
f(var2);
// ...
```

...where the number of basic blocks within use-def is bounded, the time complexity is quasi-linear. As an additional data point, if analyzing two programs $A$ and $B$ takes time $T_A$ and $T_B$ respectively and uses in $B$ never see definitions from $A$, analyzing the concatenation of $A$ and $B$ takes time $T_A + T_B + \mathcal{O}(1)$. This basically means that you're only paying for cross-program data flow, unlike with some simple SSA algorithms.

Note that the graph is implicit and never manifests in memory. I've considered replacing union-find with an offline DFS scan, but I think it'd actually be slower. First, the time loss from allocating data structures to store the graph will probably outweigh the speed up from not having to touch union-find. Second, union-find allocates one word per node, while building the graph dynamically requires nested vectors and stores two words per *edge*, so DFS would have extremely questionable memory locality.


### Dead stores

You can extend this algorithm to eliminate dead definitions within the same pass. Dead definitions are definitions that no side effect uses transitively. The required modifications are:

1. Only initially trigger DFS on `(bb, var)` if the corresponding `use` is from a side effect, and
2. When a matching definition is found in `active_defs_at_end`, push every use inside the definition to stack. (`var` and `use` will come from the uses inside the definition, not the defined variable itself, which is why `stack` doesn't just contain a `bb` field.)

Any definition that was left untouched is a dead store.


### Inlining

With this approach in mind, let's return to inlining. We want to, for each use, determine if it comes from just a single definition. But because the algorithm described above only gives us information about *components*, we can only learn the *component* of the definition, which can contain definitions that this particular use does not see (but some others do).

This analysis is similar to the one described above on a basic level. We can navigate the same graph recursively, but instead of accumulating definitions in union-find, DFS should return either "undefined", or "defined at $\mathrm{def}$", or "many definitions". We can't use iterative DFS because we need to update the state of a node based on the state of its children, rather than the other way round.

```rust
enum Source {
    Undefined,
    DefinedAt(Definition),
    ManyDefinitions,
}

impl Source {
    fn merge(&mut self, other: Source) { /* ... */ }
}

fn visit(bb: usize, var: usize) -> Source {
    let mut source = match cached.entry((bb, var)) {
        Entry::Occupied(entry) => return *entry.get(),
        Entry::Vacant(entry) => *entry.insert(Source::Undefined),
    };

    for pred in &predecessors[bb] {
        if let Some(def) = active_defs_at_end[*pred].get(&var) {
            source.merge(Source::DefinedAt(def));
        } else {
            source.merge(visit(*pred, var));
        }
    }

    *cached.get_mut(&(bb, var)).unwrap() = source;
    source
}
```

There's only a problem: if the CFG is cyclic, only the first node in an [SCC](https://en.wikipedia.org/wiki/Strongly_connected_component) is guaranteed to see all the uses. For example, consider the following graph:

$$
\mathrm{def}_1 \rightarrow \mathrm{use}_1 \leftrightarrow \mathrm{use}_2 \leftarrow \mathrm{def}_2
$$

If we enter $\mathrm{use}_1$ and then visit $\mathrm{use}_2$, $\mathrm{use}_2$ will only see the definition in $\mathrm{def}_2$, even though $\mathrm{def}_1$ is reachable -- because $\mathrm{use}_2$ refuses to visit the already visited node $\mathrm{use}_1$.

We could [condense](https://en.wikipedia.org/wiki/Strongly_connected_component#Definitions) the graph, but that would be a separate pass. But how else would we spread `source` among the whole strongly connected component? [Tarjan's algorithm](https://en.wikipedia.org/wiki/Tarjan%27s_strongly_connected_components_algorithm) comes to the rescue: it's based on DFS as well, *and* it tells you when you're exiting the strongly connected component, *and* it provides you with the list of nodes in the component. The implementation grows a bit larger, but it's still a single, very fast pass:

```rust expansible
fn visit(bb: usize, var: usize) -> DfsNodeState {
    let index = tarjan_stack.len();

    let mut state = match cached.entry((bb, var)) {
        Entry::Occupied(entry) => return *entry.get(),
        Entry::Vacant(entry) => *entry.insert(DfsNodeState {
            low_link: index,
            source: Source::Undefined,
        }),
    };

    tarjan_stack.push((bb, var));

    for pred in &predecessors[bb] {
        if let Some(def) = active_defs_at_end[*pred].get(&var) {
            state.source.merge(Source::DefinedAt(def));
        } else {
            let pred_state = visit(*pred, var);
            state.source.merge(pred_state.source);
            state.low_link = state.low_link.min(pred_state.low_link);
        }
    }

    let is_scc_root = state.low_link == index;
    if is_scc_root {
        for scc_node in tarjan_stack.drain(index..) {
            *cached.get_mut(&scc_node).unwrap() = DfsNodeState {
                low_link: usize::MAX,
                source: state.source,
            };
        }
    } else {
        *cached.get_mut(&(bb, var)).unwrap() = state;
    }

    state
}
```

This is a solid implementation that can also be extended to track values through copies like `a = b`, or verify that a use can never read undefined memory, etc.


### Outro

So that's what I'm working with at the moment. I have an optimized (but not necessarily inlined) IR, I know how to structure control flow, I know how to inline expressions and detect common control flow structures; now I just need to glue all of this together. And also figure out exceptions.

When decompiling a Minecraft server, the passes I've already implemented take $600$ ms in total and use $11$ MiB of RAM. These numbers will grow as I add passes, but there's still plenty of room for optimization (e.g. [noak](https://docs.rs/noak/) is quite slow, and I'm not sure if that's due to lack of cross-crate inlining or for a different reason), so it might even out.

Hopefully I'll get something else working by the time I publish the next post on this topic.
j