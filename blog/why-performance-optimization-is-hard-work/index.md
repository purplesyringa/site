---
title: Why performance optimization is hard work
time: April 29, 2025
discussion:
    - https://news.ycombinator.com/item?id=43831705
    - https://www.reddit.com/r/programming/comments/1kam686/why_performance_optimization_is_hard_work/
intro: |
    I'm not talking about skill, knowledge, or convincing a world focused on radical acceleration that optimization is necessary. Performance optimization is hard because it's fundamentally a brute-force task, and there's nothing you can do about it.

    This post is a bit of a rant on my frustrations with code optimization. I'll also try to give actionable advice, which I hope enchants your experience.
---

I'm not talking about skill, knowledge, or convincing a world focused on radical acceleration that optimization is necessary. Performance optimization is hard because it's fundamentally a brute-force task, and there's nothing you can do about it.

This post is a bit of a rant on my frustrations with code optimization. I'll also try to give actionable advice, which I hope enchants your experience.


### Composability

Certain optimizations can only work together, while others lead to pessimizations when combined. To be an expert means to know what optimization avenues exist; to be a master means to know which ones to choose.

I have a post on integer formatting in the works, covering a very particular algorithm design -- and I *still* haven't finished it because there's like five different choices to make, I have no idea how they impact each other, and I need to analyze $2^5$ variants to claim which one's the best in conscience. Several of my projects are similarly stuck because I don't have the willpower to implement a dozen combinations.

Pruning "obviously" suboptimal approaches is all but a heuristic. I like to think I'm more in tune with an x86-64 CPU than most people, and it still manages to surprise me from time to time. Dumb algorithms can become more applicable due to vectorization, smart code can fail due to [branch misprediction](https://en.wikipedia.org/wiki/Branch_predictor) or [store-to-load forwarding](https://en.wikipedia.org/wiki/Memory_disambiguation#Store_to_load_forwarding) gone wrong.

Optimization takes a lot of trial and error. I dislike the "intuition doesn't work, profile your code" mantra because it seemingly says profiling is a viable replacement for theoretical calculations, which it isn't. But I can't argue that profiling is avoidable. I often joke that `perf report` is my go-to disassembler.

Worse yet, you can't trust "obviously" good code either. In [a previous post](../the-ram-myth/), I optimized a single linear pass by replacing it with a superlinear sort. This is by no means a unique experience: just yesterday, I saw someone optimize best-of-class [Barrett reduction](https://en.wikipedia.org/wiki/Barrett_reduction) by dividing the numbers as `double`s, rounding them, and computing the reminder from the quotient. It's so stupid that it can't possibly work, yet it does.

The good news is that the work here can be split among multiple people trying different approaches. Open-source projects in particular benefit from this, because contributors typically have different strengths and focus on different ideas. Try to reuse work by consulting your teammates or reading about others' experiences in solving similar tasks.


### Continuity

A variation on this is algorithms where a *cut-off boundary* is present. You no longer choose whether to apply an optimization: you also need to select parameters via more trial and error. For example:

- Hybrid sorting algorithms can switch between different implementations due to high big-O constants,
- [FFT](https://en.wikipedia.org/wiki/Fast_Fourier_transform) can switch between recursive and iterative approaches to better utilize processor cache.
- Depending on data density, the optimal set structure might be bitsets, hash sets, or complementary hash sets.

Modifying either of the alternative algorithms requires rebenchmarking to update the optimal boundary. Small modifications here can lead to *drastic* end performance changes due to interactions with CPU cache, branch and memory access prediction, the discrete nature of recursive cut-offs, and floating-point precision (for [big integer multiplication via FFT](https://en.wikipedia.org/wiki/Sch%C3%B6nhage%E2%80%93Strassen_algorithm#Details)). Forgetting to rebenchmark and abandoning a prospective approach can easily leave $2 \times$ performance on the table.

For another example, consider a program that executes $n$ times either action $A$ or $B$ depending on probability $p$. If $p$ is far from $\frac12$, branch prediction means it's better to implement the switch with an `if`; if $p$ is close to $\frac12$, branch prediction will fail and a branchless approach will work better. Not only does the relative performance of $A$ and $B$ matter here, but the cost of branch misprediction matters as well, and that might depend not only on the CPU but on the precise code executed.

Ideally, you'd have a test bench that plots graphs and finds optimal parameter values automatically, even though getting this working can be draining. This way, running checks all the time becomes cheap and emotionally easier. Even if it takes half an hour, you can still work on something else in parallel.


### Incompatibility

The worst example of incompatible optimizations is those that fail due to external constraints.

One example is when two [LUTs](https://en.wikipedia.org/wiki/Lookup_table) don't fit in cache together, but do individually. You can sometimes fix this by splitting the computation into multiple passes, where each pass only needs to access helper data that does fit into cache. This does not necessarily mean two passes over *all* data, consuming $2 \times$ memory bandwidth -- you can chunk the data and apply two passes on a chunk, which increases performance if the chunk fits into, say, L3. But sometimes that doesn't work, and then I bash my head against the wall.

Register pressure is even worse because that is only a problem because of the ISA, not the [microarchitecture](https://en.wikipedia.org/wiki/Microarchitecture). The hardware has enough registers, they just aren't exposed to user code. You can try to split data between general-purpose registers and vector registers, and that works as long as you seldom cross the GPR-SIMD boundary, but at that point, you might as well [change your profession](https://github.com/docker/cli/issues/267#issuecomment-695149477).

It doesn't have to be that way. [FPGAs](https://en.wikipedia.org/wiki/Field-programmable_gate_array) enable you to design your own hardware (kind of, anyway), and alternative approaches like [interaction nets](https://en.wikipedia.org/wiki/Interaction_nets) have a chance to make software-specified operations as optimal as operations that are usually implemented in hardware. But that's not the world we live in, no, we live in the world where Intel keeps introducing useful instructions to AVX-512 only to abandon them later, so I need to choose between a CPU with `vp2intersect` or with [FP16](https://en.wikipedia.org/wiki/Half-precision_floating-point_format). So not only do you have to benchmark different code, you also have to test it on different CPUs to decide which EC2 instance to deploy it on.

The only advice I have for this is to try to achieve the best possible result, even if it's worse than the theoretical optimum. Reduce the size of one of the LUTs by moving some calculations to runtime, rewrite a chunk of code in assembly to manage registers better, and when all else fails, accept that you have to make a choice.


### Compilers

"Compilers are smarter than humans" is a common mantra. It couldn't be further from the truth. Any developer can see that the following two snippets are (supposed to be) equivalent:

```rust
let condition1 = HashSet::from([a, b]).contains(&c);
let condition2 = a == c || b == c;
```

But compilers aren't going to optimize the former into the latter ([JVM's JIT, in some cases, excluded](https://4comprehension.com/the-curious-case-of-jdk9-immutable-collections/)). They don't reason in abstractions, and they certainly don't reason in *your* auxiliary abstractions. This doesn't just apply to high-level code: LLVM [does not even understand](https://godbolt.org/z/j3ehhr3KT) that bitwise AND is an intersection.

No, compilers excel at something different from optimization: they turn higher-level languages into zero-cost abstractions, but there's no ingenuity. Compilers are optimal transpilers -- barring a few exceptions, they codegen exactly what you wrote in the source. They allow you to write assembly with the syntax and capabilities of Rust or C++, but don't you dare forget that the `arr.map(|x| x / c)` you wrote will invoke `idiv` without performing obvious [libdivide](https://github.com/ridiculousfish/libdivide)-style precalculations.

Sometimes I wonder if `-O2` should be renamed to `-fzero-cost-abstractions`.

This might make it sound like I'm arguing that compilers are only good at plumbing, but they aren't even good at that. For example, they can be terrible at register allocation of all things. If a rarely executed chunk of code needs many registers, GCC satisfies that need by [spilling variables accessed by the hot path](https://godbolt.org/z/53o1vdsfj) *on every iteration*, not only on entry to cold path. Clang handles this simple example better but fails in more complicated cases.

The lesson is never to trust the compiler blindly. Always check the disassembly, consult an instruction-level profiler like `perf`, and don't be afraid to use this information to nudge the compiler to do the right thing if it leads to tangible improvements.

Despite obvious shortcomings, compilers don't allow you to correct them on things they get wrong. There is no way to provide both optimized assembly and equivalent C code and let the compiler use the former in the general case and the latter in special cases. Custom calling conventions are mostly unsupported, and so is choosing between branchless and branchy code and any other assembly tricks. There are intrinsics, but LLVM and rustc *still* try to be smart and rewrite them, which sometimes causes pessimizations, leaving no alternative but to add an optimization barrier.

[e-graphs](https://egraphs-good.github.io/), as popularized by [Cranelift](https://cranelift.dev/), try to tackle this problem, but to my knowledge, there hasn't been much success in this field. I'm still hopeful, though.


### Documentation

For x86 processors, [uops.info](https://uops.info/table.html) provides timing and port information for each instruction and many Intel and AMD CPUs. [Agner Fog](https://www.agner.org/optimize/) wrote a manual on optimization for x86 processors and publishes his own tables. [Intel Software Developer's Manual](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html) contains more than 5000 pages documenting not only the instruction set but many internal workings of their CPUs as well.

Apple Silicon has *nothing* like that. I have no goddamn idea how to work with M1+ processors. There's [Apple Silicon CPU Optimization Guide](https://dn721600.ca.archive.org/0/items/apple-silicon-cpu-optimization-guide/Apple-Silicon-CPU-Optimization-Guide.pdf), which contains only 169 pages and reads like something people would write for novices, not experts. It reads like a tutorial you might find on HN, not something I would be interested in. It contains estimates of latencies and throughputs for some categories of instructions, but there's frustratingly little tabular data, and it doesn't mention [uop fusion](https://stackoverflow.com/questions/56413517/what-is-instruction-fusion-in-contemporary-x86-processors) or provide port information. [Dougall Johnson's research](https://dougallj.github.io/applecpu/firestorm.html) is immensely valuable but only covers M1, not newer CPUs, and it still doesn't answer many questions.

Even [Apple's LLVM fork](https://github.com/swiftlang/llvm-project/tree/next/llvm/lib/Target/AArch64) lacks scheduling annotations for Apple Silicon. How am I supposed to write efficient code when Apple doesn't bother to tune their own compiler? Optimizing code for such a platform is 90% reverse engineering and 10% writing meaningful code -- and writing meaningful code is already hard.

The right fix for this is to commit intellectual property theft, but I'm not allowed to say that, so I won't. Oops.


### Conclusion

Performance optimization is hard because you have to:

- Explore dozens of cases manually without losing your mind.
- Iterate with inadequate tooling. (Profilers and [MCA](https://llvm.org/docs/CommandGuide/llvm-mca.html) are useful, but they're still toys that can't match the underlying complexity.)
- ~~Jam squares into round holes until they fit.~~ Merge incompatible optimizations.
- Deal with both corporate greed and cultural apathy.

It's not easy by any means, but it's still something I enjoy doing, even though people often consider anything short of radical improvements a waste of time. To me, a 10% optimization is a form of art, but it's not just that. Small improvements compound and help form a better user experience, even if no single optimization seems valuable on its own -- much like improving data transfer rates has led to structural changes in how we process and utilize information.

Optimizations save time, and time is the one resource people don't get enough of.
