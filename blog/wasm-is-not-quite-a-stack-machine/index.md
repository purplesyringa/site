---
title: Wasm is not quite a stack machine
ogTitle: "Wasm is not quite\na stack machine"
time: April 27, 2026
discussion:
- https://lobste.rs/s/h8hlp7/wasm_is_not_quite_stack_machine
- https://news.ycombinator.com/item?id=47930493
intro: |
    *Everyone* knows Wasm is a stack machine. Wikipedia says so, the official [Wasm design specification](https://github.com/WebAssembly/design/blob/376bcc4b9cba79280d79be023d71e30d0b00ba47/Semantics.md) says so, you get it. I thought so too.

    That is, until I started writing Wasm code -- not compiling for Wasm, but writing the instructions by hand. And I found out that there exists a major difference between Wasm and all other stack-based languages, that makes this claim misleading.
---

*Everyone* knows Wasm is a stack machine. Wikipedia says so, the official [Wasm design specification](https://github.com/WebAssembly/design/blob/376bcc4b9cba79280d79be023d71e30d0b00ba47/Semantics.md) says so, you get it. I thought so too.

That is, until I started writing Wasm code -- not compiling for Wasm, but writing the instructions by hand. And I found out that there exists a major difference between Wasm and all other stack-based languages, that makes this claim misleading.


### Register vs stack

Let's back up a bit. What *is* a stack machine, even?

Say you write a program in a high-level language, and at some point you want to calculate `2 * 3 + 5 * 7`. Low-level languages don't have a notion of compound expressions: they can only perform one operation at a time. So you need to do two multiplications, save their results, and then perform addition.

Many low-level languages, like x86 assembly, would represent these steps as follows:

- `a = 2`
- `b = 3`
- `c = a * b`
- `d = 5`
- `e = 7`
- `f = d * e`
- `g = c + f`

This is called a *register machine*. You have variables (called *registers*), which can be used to store both persisted values and temporary results, and each instruction has form `var1 = var2 op var3`.

Other languages, like [Forth](https://en.wikipedia.org/wiki/Forth_(programming_language)) or [Hex Casting](https://hexcasting.hexxy.media), use a *stack* for this purpose. The stack can store a sequence of values in an ordered manner, so that already computed subexpressions can lie around while you're working on other parts. In a stack-based language, the same calculation would look like:

- `push(2)`
- `push(3)`
- `mul()` -- pops the last two values from the stack and pushes their product
- `push(5)`
- `push(7)`
- `mul()`
- `add()`

Note that there's a similarity between the two programs: they have the same number of steps, and the corresponding steps perform the same operation. The major difference is that with a stack machine, the values operated upon are implicitly encoded in the program order, while the register machine always encodes indices.


### Rearrangement

We know always-shrinking lossless compression doesn't exist, though, so what expression power is lost by making indices implicit? For simple expressions, not much. But when values are *reused*, the difference becomes clear.

Say you're a compiler, and you're asked to compile this program:

```python
x = 1 + 2 + 3 + 4
y = x * x * x
```

With a register machine, you can do:

- (calculate `x` as usual)
- `tmp = x * x`
- `y = tmp * x`

A stack machine as described above, however, does not offer a way to refer to the same value twice: `mul` always multiplies two values on different positions in the stack. To enable this calculation, real stack machines introduce *stack manipulation* operations in addition to pure calculation. The one we're looking for is called `dup`, and it *dup*licates the value on top of the stack:

- (calculate `x` as usual)
- `dup()` -- the stack now contains `x, x`
- `dup()` -- the stack now contains `x, x, x`
- `mul()` -- the stack now contains `x, x*x`
- `mul()` -- the stack now contains `x*(x*x)`

You might notice that the register machine calculated `(x*x)*x`, while the stack machine calculated `x*(x*x)`. These two are the same thing for multiplication, but may be different for other operations. To fix this, we also need to introduce `swap`, which, as the name implies, swaps the two values on top of the stack:

- (calculate `x` as usual)
- `dup()` -- the stack now contains `x, x`
- `dup()` -- the stack now contains `x, x, x`
- `mul()` -- the stack now contains `x, x*x`
- `swap()` -- the stack now contains `x*x, x`
- `mul()` -- the stack now contains `(x*x)*x`

In practice, more operations are usually used to facilitate computation: `over` (copy second-last value to the top), `2dup` (duplicate two values), `drop` (pop last value), `rot` (move third-last value to the top), etc.

From this perspective, stack machines can be seen as decoupling *operations* from *indices* they operate on. Whereas register machines always encode indices and pay a higher price when they're redundant, stack machines encode them on an as-needed basis, but at the cost of a higher instruction count. If I wanted to be fancy, I'd say stack machines implement entropy-encoded compression for register machines.


### Wasm

If you look at JVM, a well-known stack machine Wikipedia compares WebAssembly to, you'll find basically this exact [list of bytecode instructions](https://en.wikipedia.org/wiki/List_of_JVM_bytecode_instructions):

- Value producers and consumers: `iaload`, `iastore`, `iconst`.
- Unary and binary operations: `d2f`, `iadd`.
- Stack manipulation instructions: `dup`, `dup_x1` (aka `over`), `pop` (aka `drop`), `swap`.

JVM is not a pure stack machine: there are also instructions for accessing local variables, like `iload` and `istore`. But it's possible to write powerful JVM programs without their use, and `javac` mostly only uses them for variables explicitly created by the Java programmer.

Now let's look at the [Wasm instruction set](https://webassembly.github.io/spec/core/appendix/index-instructions.html):

- Value producers and consumers: `i32.load`, `i32.store`, `i32.const`.
- Unary and binary operations: `f32.demote_f64`, `i32.add`.
- Stack manipulation instructions: `drop`, uhh, ???.

Well, now isn't that interesting? Wasm has plenty of instructions that receive arguments and place return values on the stack, but almost no instructions that can rearrange it -- and, as far as I can tell, `drop` only exists because otherwise you wouldn't have a way to ignore a function output.

Pretty much the only thing pure Wasm can do is evaluate simple expressions exactly as written in source code. An optimizing compiler can't perform [common subexpression elimination](https://en.wikipedia.org/wiki/Common_subexpression_elimination) or optimize `expr^2` to `expr * expr` without introducing new variables. The moment you need anything non-trivial, you have to reach for variables -- and thus end up with a register machine, the "stack machine" illusion falling apart.


### Semantics

In my opinion, the right way to look at Wasm is as a register machine with operations generalized to compound expressions.

In binary Wasm, the expressions are encoded in [Reverse Polish notation](https://en.wikipedia.org/wiki/Reverse_Polish_notation), which *can* be evaluated with a stack, but this is just an encoding. In [textual Wasm](https://developer.mozilla.org/en-US/docs/WebAssembly/Guides/Understanding_the_text_format), for example, they are instead represented in a [LISP-like notation](https://en.wikipedia.org/wiki/Polish_notation) -- not any less or more efficient. One can imagine a world where binary Wasm used prefix notation as well, with little impact; if I had to guess, postfix notation was preferred to simplify non-optimizing interpreters, or perhaps the experience with stack-based VMs was a tie-breaker.

This perspective is further confirmed by the fact that, until Wasm got [the `multi-value` extension](https://github.com/WebAssembly/multi-value/blob/master/proposals/multi-value/Overview.md), control flow blocks pretty much couldn't interact with the stack: values pushed onto the stack before `if` could not be accessed within the `if` body, and the `if` body could only return one value, so `if` was effectively just a ternary, and even values with a single consumer had to go through locals.


### Conclusion

Does it really matter? Pretty much any machine can be converted to SSA, at which point the input format is not a consideration; and I suppose the simplicity of stack-based implementation was a good thing for Wasm adoption. But I think it's fair to highlight that experience with stack-based VMs doesn't translate well to Wasm, since it's not *quite* a stack machine.

> Soon after writing this post, I found [this awesome post](http://troubles.md/posts/wasm-is-not-a-stack-machine/) covering the same problem from a different, optimization-focused angle. Give it a read as well!
