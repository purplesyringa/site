---
title: Superoptimizing an FPU emulator
ogTitle: "Superoptimizing\nan FPU emulator"
time: October 16, 2026
intro: |
    > **Superoptimization** is the process where a compiler automatically finds the optimal sequence for a loop-free sequence of instructions.
    > 
    > -- [Wikipedia](https://en.wikipedia.org/wiki/Superoptimization)

    Compilers are good at making mechanical transformations that make code faster. Unfortunately, most of them are not as good at non-trivial reasoning, so they can't invent a completely novel approach to solving your problem.

    When designing a library for floating-point calculations, I had to implement a function that takes a float $f$ and produces a float corresponding to the lowest set bit in $f$. For example, if $f = 1.01101 \cdot 2^{10}$ in binary, it would return $0.00001 \cdot 2^{10} = 2^5$.

    This can be implemented with bit twiddling, but the straightforward algorithm breaks for powers of two, so I had to insert an `if`:

    ```rust
    if f32::from_bits(x.to_bits() & !f32::MANTISSA_MASK) == x { // is x a power of two?
        return x; // x itself is the lowest bit
    } else {
        return x - f32::from_bits(x.to_bits() & (x.to_bits() - 1)); // x, minus x without lowest bit
    }
    ```

    This `if` really annoyed me. It's slowed down for the common case, and branches could be mispredicted. I wanted to find a formula that automatically handled powers of two, but try as I might, I couldn't find anything convincingly optimal.

    So I made a non-orthodox play.
---

> **Superoptimization** is the process where a compiler automatically finds the optimal sequence for a loop-free sequence of instructions.
> 
> -- [Wikipedia](https://en.wikipedia.org/wiki/Superoptimization)

Compilers are good at making mechanical transformations that make code faster. Unfortunately, most of them are not as good at non-trivial reasoning, so they can't invent a completely novel approach to solving your problem.

When designing a library for floating-point calculations, I had to implement a function that takes a float $f$ and produces a float corresponding to the lowest set bit in $f$. For example, if $f = 1.01101 \cdot 2^{10}$ in binary, it would return $0.00001 \cdot 2^{10} = 2^5$.

This can be implemented with bit twiddling, but the straightforward algorithm breaks for powers of two, so I had to insert an `if`:

```rust
if f32::from_bits(x.to_bits() & !f32::MANTISSA_MASK) == x { // is x a power of two?
    return x; // x itself is the lowest bit
} else {
    return x - f32::from_bits(x.to_bits() & (x.to_bits() - 1)); // x, minus x without lowest bit
}
```

This `if` really annoyed me. It's slowed down for the common case, and branches could be mispredicted. I wanted to find a formula that automatically handled powers of two, but try as I might, I couldn't find anything convincingly optimal.

So I made a non-orthodox play.


### Brute-force

What if there was a way to find such a formula automatically? No, not by spawning an LLM swarm, but by brute-forcing all possible formulas. LLMs can miss stuff or overindex on popular patterns; brute-force can't.

But how do we check if a formula is correct? Theorem provers are too slow, but we can design a few representative test cases and hope that whatever formula we find is correct. And if it has a counterexample, just add it to the list of test cases and repeat.

This is not as insane as it sounds: by this moment I had already implemented an algorithm with $6$ operations, so I just needed to iterate over formulas with $5$ ops or fewer. I knew that the following operations could be useful:

- Integer addition/subtraction and bitwise AND/OR/XOR for bit tricks.
- Floating-point subtraction as the last operation.
- Maybe comparisons?

There are $6^5 = 7776$ ways to choose $5$ operations, each of which can be one of $6$ variants, and [$C_5 = 42$ possible parentheses placements](https://en.wikipedia.org/wiki/Catalan_number). That's not a lot!

But we also need to choose the inputs to each operation, which can be either the input value $x$ or some constant. I guessed that we'll only need constants that cover various fields of the `float` binary representation, and not completely random ones, reducing the list to $64$ different constants. That's $65^6$ combinations, which sounds like a lot, but in practice we can cull many of them.


### Search

To implement the search efficiently, I build up the formula set inductively.

Formulas with $0$ operations are just $x$ and constants; formulas with $n$ operations can be found by combining all pairs of formulas with $k$ and $n - 1 - k$ operations:

```rust
for n_ops in 1..N_OPS {
    let mut new_exprs = FxHashSet::default();
    for a_ops in 0..n_ops {
        let b_ops = n_ops - 1 - a_ops;
        let a_exprs = &exprs_per_op_count[a_ops];
        let b_exprs = &exprs_per_op_count[b_ops];
        for a_expr in a_exprs {
            for b_expr in b_exprs {
                for op in OPS {
                    // New expression: `a_expr op b_expr`
                    new_exprs.insert(Expr {
                        values: core::array::from_fn(|i| {
                            op(a_expr.values[i], b_expr.values[i])
                        }),
                    });
                }
            }
        }
    }
}
```

Induction allows us to prune duplicates. For example, $(x + 1) + 2$ and $(x + 2) + 1$ are different expressions, but they return the same values. Keeping just one expression for each set of outputs allows further iterations to complete faster. This is the reason why brute-force can complete in just $4$ minutes! (I'm sure it can be optimized further, but I preferred watching YouTube over optimizing a one-off script.)

To save time and memory, I avoid deduplication on the last iteration and output all matching candidates. Memory usage matters because I had $20$ test cases by this point, so I had to store $80$ bytes per expression, and there are *a lot* of expressions.


### Results

So, did this exhaustive search find anything interesting? Yes! There were several formulas with $5$ operations, though none with $4$ or fewer.

But this approach has a limitation: it doesn't take into account that calculations can be reused. So, for example, if `x & 1` is used in two places, it will count that as two operations, not one. I was hoping that one of the $5$-op formulas could actually be expressed as $4$ operations via reuse, and after manually checking every formula, the very last one turned out to have this property! Here's what it looks like:

```rust
let a = x.to_bits() & !f32::MANTISSA_MASK; // x without mantissa bits
return f32::from_bits(x.to_bits() & (a - x.to_bits())) - f32::from_bits(a);
```

It doesn't make much sense at first glance, and it's not supposed to.

The reason why only this formula can reuse `a` is the subexpression `a - x.to_bits()`. It's equivalent to `-(x & f32::MANTISSA_MASK)`, which is how other formulas refer to it. All formulas compute `x & !f32::MANTISSA_MASK` and `x & f32::MANTISSA_MASK`, but this one rewrites the latter as `x - (x & f32::MANTISSA_MASK)`, and then merges that with negation to save one operation!

I was flabbergasted when I realized it found such a clever rewrite. You don't need an LLM to be amazed by what computers can invent! And it doesn't destroy industries.

Anyway, this code now [lives in my work-in-progress FPU emulator](https://github.com/purplesyringa/fpu-v/blob/d3c3f72c691698e5095d259a79610e5293911651/src/ops/multiplication.rs#L485) and I'm proud of it.
