---
title: The expression problem and Rust
time: September 30, 2025
intro: |
  A few days ago, I stumbled upon [a Hacker News discussion](https://news.ycombinator.com/item?id=45155877) about the expression problem -- a conundrum that occasionally arises in software design. Some of the commenters noted that Rust completely avoids this problem thanks to trait objects, and initially I agreed with them, but I'm now realizing it's not at all as straightforward as it looks.

  The goal of this post is to explain what the expression problem is, how Rust seemingly avoids it, why this solution doesn't actually work, and what a Rusty solution might look like.
---

A few days ago, I stumbled upon [a Hacker News discussion](https://news.ycombinator.com/item?id=45155877) about the expression problem -- a conundrum that occasionally arises in software design. Some of the commenters noted that Rust completely avoids this problem thanks to trait objects, and initially I agreed with them, but I'm now realizing it's not at all as straightforward as it looks.

The goal of this post is to explain what the expression problem is, how Rust seemingly avoids it, why this solution doesn't actually work, and what a Rusty solution might look like.


### Setting the stage

I'm notoriously bad at explaining things in simple terms, so I'd advise you to read [the post by Eli Bendersky](https://eli.thegreenplace.net/2016/the-expression-problem-and-its-solutions/) that sparked this discussion in the first place. I'll repeat the gist of it here, but in more abstract terms, since I've found that they give good intuition about which solutions work and which don't.

Suppose that you have various *data types* and various *operations* performed on those types, and the correspondence is many-to-many, i.e. each operation can be performed on each data type.

Since the expression problem often arises when working on programming languages, I'll give a motivating example from programming language theory: the *data types* would be various AST nodes, like "integer", "string", "array", "add", etc., and the *operations* would be "stringify", "dump", "interpret", "analyze", etc. The behavior of each operation differs depending on the data type it's applied to.

For simplicity, we'll assume that each implementation of an operation deals with exactly one data type. In other words, if you have $N$ data types and $M$ operations, you'll have to define exactly $N \times M$ behaviors.


### Extensibility

As your codebase grows larger, you might want to split out some parts into other modules, or maybe even libraries. Alternatively, you might want to allow other libraries to add capabilities to your language.

To do that, you need to allow external code to add new data types and operations to your program. The expression problem states that, in most languages, it's trivial to design the API such that *either* new data types *or* new operations can be added, but it's oddly complicated to enable both.


### Examples in Rust

Suppose that you didn't care about extensibility. How would you design your program?

Two ways come to my mind. The first one is `enum`-based:

```rust
enum AstNode {
    Integer(i32),
    Str(String),
    Array(Vec<AstNode>),
    Add(AstNode, AstNode),
    ..
}

fn stringify(node: AstNode) -> String {
    match node { .. }
}
fn dump(node: AstNode, fmt: &mut Formatter<'_>) {
    match node { .. }
}
fn interpret(node: AstNode) -> Value {
    match node { .. }
}
..

fn parse(code: &str) -> AstNode {
    ..
}
```

The second one is `trait`-based:

```rust
struct Integer(i32);
struct Str(String);
struct Array(Vec<Box<dyn AstNode>>);
struct Add(Box<dyn AstNode>, Box<dyn AstNode>);
..

trait AstNode {
    fn stringify(self) -> String;
    fn dump(self, fmt: &mut Formatter<'_>);
    fn interpret(self) -> Value;
    ..
}

impl AstNode for Integer {
    fn stringify(self) -> String { .. }
    fn dump(self, fmt: &mut Formatter<'_>) { .. }
    fn interpret(self) -> Value { .. }
}
..

fn parse(code: &str) -> Box<dyn AstNode> {
    ..
}
```

With the `enum`-based implementation, it's trivial for external code to add new operations -- just define a function -- but adding a new data type requires modifying an existing `enum`, which might be defined in a whole 'nother crate.

With the `trait`-based implementation, it's trivial for external code to add new data types -- just define a `struct` and implement the trait for it -- but adding a new operation requires modifying an existing `trait`, which might be defined in a whole 'nother crate.


### "Solution"

The "solution" to this problem discussed on HN is to split a single `AstNode` trait into many: one per operation. This makes data types and operations orthogonal: the former is represented by types and the latter is represented by traits. Since Rust allows foreign traits to be implemented for local types, any crate can add a new data type; and since Rust allows local traits to be implemented for foreign types, any crate can add a new operation.

```rust
struct Integer(i32);
struct Str(String);
..

trait Stringify {
    fn stringify(self) -> String;
}
trait Dump {
    fn dump(self, fmt: &mut Formatter<'_>);
}
..

impl Stringify for Integer {
    fn stringify(self) -> String { .. }
}
impl Dump for Integer {
    fn dump(self, fmt: &mut Formatter<'_>) { .. }
}
..
```

Do you see the problem?

There's something I didn't show you -- two things, actually. Let's start with the obvious one: what does the definition of `Array` look like? It can't be `struct Array(Vec<Box<dyn AstNode>>);` because there's no single trait anymore; but then *what* traits do we list?

```rust
struct Array(Vec<Box<dyn Stringify + Dump + ..>>);
```

Whatever we choose, the definitions of data types will hard-code the list of operations that can be performed on those types. It's impossible for external crates to add new operations to existing data types since it's impossible to use a trait method that isn't declared in the `dyn` annotation. In fact, it's in no way better than a single trait, it just obscures the underlying problem more.

Of course, we could go back to `enum`s, but then the definition of each data type will hard-code the list of all possible data types, which would lead to the same issue.


### Domains

But let's talk about the other problem.

Back when we had a single `enum`, the `parse` function returned that enum. When we had a single trait, it returned `Box<dyn AstNode>`. But what should it return now?

Perhaps counterintuitively, I claim that returning `enum` is actually *good*. Think about it like this.

When functions receive AST nodes as *arguments*, they consume them by applying *operations* to them. The function couldn't care less about the specific data type, the only thing that matters is that it supports the given set of operations.

When functions *return* AST nodes, they construct them by initializing concrete *data types*. The function couldn't care less about the operations you might want to apply to this data later, the only thing that matters is that the consumer knows how to handle this specific set of data types.

In other words, the argument of a function is in *the operation domain*, while its return value is in *the data type domain*.

Returning `enum` allows `parse` to exhaustively specify which data types the consumers need to deal with, so that they can statically validate that the necessary operations are implemented for all of them.


### Responsibility

The good thing about this distinction is that it clearly defines who is responsible for validating that each emitted type supports each requested operation.

Suppose that I have two crates: `mylang-parse` and `mylang-analyze`. If I add new syntax to the parsing crate, which requires me to add a new data type, and forget to teach `mylang-analyze` how to handle this type, neither of the two crates is in the wrong. The problem only surfaces when I pass the return value of `parse` to the input of `analyze`, i.e. when I try to move the node from *the data type domain* to *the operation domain*. In Rust, this is the exact place where the compiler will check that a type implements a trait.

The key part is that the line `dump(parse(code))` appears *outside* the two crates. This means that I lied, and there's actually a third crate, `mylang-cli`, which depends on `mylang-parse` and `mylang-analyze` and declares the relationship between the two.

If `mylang-parse` updates `parse` to return a new data type, that should be a breaking change, since there is no proof that this new type implements all the operations that someone might depend on. And if `parse` returns `enum`, that *is* the case, because adding a variant to an `enum` is breaking, whereas implementing a trait for a type isn't.

If `mylang-analyze` requires the argument of `analyze` to implement a new operation, this should also be a breaking change. If `analyze` takes `impl Op1 + Op2`, adding a new trait to the mix changes the signature of the function, which is also considered breaking.

Whenever something like that happens, the type check will fail in the parent `mylang-cli` crate, which bears the responsibility of rolling back the update to one of the crates, or upgrading the other crate, or adding the missing implementation by hand, etc.


### Crossing domains

Obviously, crossing this domain boundary is ugly: converting `enum` to `Box<dyn Trait>` is nasty and full of boilerplate, even though it's theoretically possible.

But there are more pressing matters. If you assume that functions take nodes in *the operation domain* and return nodes in *the data type domain*, then any node transformer, including an "identity" function, would have to cross this boundary in the other direction and convert *any* type that implements a certain trait to a *specific* data type. This is clearly impossible and semantically meaningless.

The "identity" bit here is more than just an example. Note how `core::convert::identity` sidesteps this problem by using generics. Consider its signature:

```rust
fn identity<T>(x: T) -> T;
```

In effect, this says that the input and the output are the same *data type*. The fact that they must therefore support identical *operations* is a corollary. If Rust supported generic trait parameters, you could imagine an alternative signature:

```rust
fn identity<trait Trait>(x: impl Trait) -> impl Trait;
```

In other words, the input and the output support the same *operations*. This time, the correspondence is swapped: you can infer that the input and the output must be the same *data type* because you can substitute `Trait = Is<T>`, where `Is<T>` is implemented for `T` and only for `T`.

Generics allow the two domains to be merged together.


### Solution

And that's the core of the solution. Instead of returning enums and taking trait objects, functions should take a generic parameter, `Node`, and assert that this `Node` either supports the required *operations* with `Node: Operation` in the `where` clause, or that this `Node` can store a given concrete *data type* with `Node: From<DataType>`. Function signatures would look like this:

```rust
fn analyze<Node: Stringify + Dump + Statistics + ..>(node: Node) -> String;

// This is a bit long compared to returning an `enum`, but consider that the variants of an `enum`
// are fundamentally also part of the signature.
fn parse<Node: From<Integer> + From<Str> + ..>(code: &str) -> Node;
```

You could even imagine bounds like `TryInto<DataType>` to downcast types. Say we want to optimize each call `ord("<character>")` into a number, then the signature will look as follows:

```rust
fn transform<Node>(node: Node) -> Node
where
    Node: GetChildren, // operate recursively
    Node: TryInto<FunctionCall> + TryInto<Str>, // recognize function calls and strings
    Node: From<Integer>; // construct numbers
```

Same for recursive data types -- they should take `Node` as a generic parameter without requiring it to implement any operations:

```rust
struct Array<Node>(Vec<Node>);
```

Modeling operations is a little trickier. For operations that only consume nodes, we can keep the API simple:

```rust
trait Stringify {
    fn stringify(self) -> String;
}

impl Stringify for Integer { .. }

impl<Node: Stringify> Stringify for Array<Node> { .. }
```

Some operations might need to mention `Node` in the method signature (e.g. those that create new data, think a helper operation for `parse`). In that case, we can't make the method itself generic over `Node`, because that would require parametrized data types, e.g. `Array<T>`, to implement the method for *any* choice of the generic parameter `Node`, even those not equal to `T`.

Instead, we can either make `Node` an associated type of the trait or parametrize the trait over `Node`. The latter turns out to be more orthogonal, since it allows primitive types like `Str` to implement `Operation<Node>` regardless of `Node`.

```rust
trait Optimize<Node> {
    fn optimize(self) -> Node;
}
```


### Implementation

In a nutshell, the idea is to allow the end user, i.e. a high-level crate, to dictate the exhaustive list of data types and operations, and pass it down to lower-level crates. It acts as an orchestrator, allowing individual components not to hard-code specific representations. A straightforward implementation of this concept might look like this:

```rust
use mylang_parse::data_types::*;
use mylang_analyze::operations::*;
..

enum AstNode {
    Integer(Integer),
    Str(Str),
    Array(Array<AstNode>),
    ..
}

impl From<Integer> for AstNode { .. }
impl From<Str> for AstNode { .. }
impl From<Array<AstNode>> for AstNode { .. }
..

impl Stringify for AstNode {
    fn stringify(self) -> String {
        match self {
            Self::Integer(node) => node.stringify(),
            Self::Str(node) => node.stringify(),
            Self::Array(node) => node.stringify(),
            ..
        }
    }
}
..
```

That's not to say that this exhaustive list has to be present in source code. There's no real reason why all this mess cannot be inferred. After all, if we passed an opaque type `AstNode` to `parse` and `dump`, Rust would know the exhaustive list of trait bounds `AstNode` is required to satisfy. Bounds of kind `From<T>` would go in the `enum` hole, and other bounds would go in the `impl` dispatch hole.

Note that this problem is fundamentally different from everything we've encountered so far. Every other problem has been semantic, i.e. unsolvable in general regardless of features of the language, be that due to compatibility issues, problems with variance, etc.; this time, it's merely Rust not supporting a pattern.


### Some hacks

But we can get pretty close. Unfortunately, there are two ways to do this; and the last time there were two ways to do something, neither worked well.

Let's start with a greedy solution. Since we're already hard-coding the list of supported operations, we can use trait objects to at least avoid naming data types directly:

```rust
struct AstNode(Box<dyn Stringify<AstNode> + ..>);

impl<T> From<T> for AstNode where T: Stringify<AstNode> + .. + 'static {
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}

impl Stringify for AstNode {
    fn stringify(self) -> String {
        self.0.stringify()
    }
}
..
```

Note that the data types are still *there*, they still affect compatibility and whether the code passes type check, they're just effectively implicit.

Now let's try to reduce the repetition of operation names. We'd love to define

```rust
type AstNode = dyn Stringify<AstNode> + ..;
```

...which would automatically implement all of those traits, and push the `Box` wrapper down into consumers. But Rust doesn't support recursive type aliases, so we'd have to wrap it in a newtype `struct`, which doesn't forward trait implementations.

In a desperate attempt to apply the tools we're given to the wrong job, we can replace `Node: Operation` bounds with `Node: Deref<Target: Operation>` and implement `Deref` for `struct AstNode`:

```rust
trait Operations<Node> = Stringify<Node> + ..; // pretend this is a normal trait with a blanket impl

struct AstNode(Box<dyn Operations<AstNode>>);

impl<T> From<T> for AstNode where T: Operations<AstNode> + 'static {
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}

impl Deref for AstNode {
    type Target = dyn Operations<AstNode>;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}
// ...and similarly for `DerefMut` and some kind of polyfill for `DerefMove`.
```

We're effectively lifting traits to types via `dyn` and simulating supertrait relationships with `dyn Trait1: Trait2`.


### Linking crates

But we're still naming operations, even if only once. If you think about this, it's kind of similar to when we experimented with `parse` returning an `enum` vs taking `Node: From<Integer> + From<Str> + ..`. An exhaustive list isn't necessarily bad architecture, it might just be an explicit annotation of what would otherwise be forwarded implicitly.

In fact, if all listed operations are ones that the high-level crate itself utilizes, it's not a big deal at all. If there's a direct correspondence between this list and function invocations in the same crate, it's a bit ugly, but ultimately not a maintenance hazard or an expressiveness gap.

What we really want to avoid here is internal operations used by a different crate leaking into this list. (Though it's still fundamentally not a compatible abstraction boundary, and the only thing we can do is implicitly update this list when the dependency updates.) To fix this, we can let crates export the list of operations their APIs use. For example, `mylang-analyze` might export what amounts to:

```rust
trait Operations<Node> = Stringify<Node> + ..; // again, assume this is a trait + blanket impl
```

...and the high-level crate might write:

```rust
trait Operations<Node> = mylang_analyze::Operations<Node> + ..; // same here
```

So now we're only hard-coding the list of dependencies, which is more than fine, really.


### Hacking types

But there's another way to simplify the `enum` with a ton of variants + ton of `impl`s combo. This time, we can try to apply the `Deref` trick to simplify `impl`s first, in hopes that this will let us avoid naming traits at all.

We want the consumer of `AstNode` to provide the specific type they are trying to deref to, i.e. `Deref::Target`, since we can't name it ourselves. This abstraction is called `AsRef`, and what we're trying to do is:

```rust
enum AstNode {
    Integer(Integer),
    Str(Str),
    Array(Array<AstNode>),
    ..
}

impl From<Integer> for AstNode { .. }
impl From<Str> for AstNode { .. }
impl From<Array<AstNode>> for AstNode { .. }
..

impl<trait Trait> AsRef<dyn Trait> for AstNode
where
    Integer: Trait,
    Str: Trait,
    Array<AstNode>: Trait,
    ..
{
    fn as_ref(&self) -> &dyn Trait {
        match self {
            Self::Integer(node) => node,
            Self::Str(node) => node,
            Self::Array(node) => node,
            ..
        }
    }
}
```

Of course, Rust doesn't support generic trait parameters, but nightly Rust has `CoerceUnsized`, which allows us to represent the notion of upcasting to an arbitrary trait object:

```rust
#![feature(coerce_unsized)]

impl<T: ?Sized> AsRef<T> for AstNode
where
    for<'a> &'a Integer: CoerceUnsized<&'a T>,
    for<'a> &'a Str: CoerceUnsized<&'a T>,
    for<'a> &'a Array<AstNode>: CoerceUnsized<&'a T>,
    ..
{
    fn as_ref(&self) -> &T {
        match self {
            Self::Integer(node) => node,
            Self::Str(node) => node,
            Self::Array(node) => node,
            ..
        }
    }
}
```

Instead of specifying the exact data types, we can specify the crates they're provided by:

```rust
enum AstNode {
    Parsed(mylang_parse::Node),
    Extension1(mylang_extension1::Node),
    ..
}
// ^ you'd need to take care to avoid diamond inheritance here, e.g. require that
// `mylang_somecrate::Node` exclusively lists data types defined within that crate, not just data
// types it depends on.

impl From<mylang_parse::Node> for AstNode { .. }
impl From<mylang_extension1::Node> for AstNode { .. }
..
```

And now, just like with the other solution, we're hard-coding dependency crates, but not data types or operations. The only difference is that we're hard-coding the list of crates that provide new data types, rather than the list of crates that require new operations.


### Wrapping up

We started with a design that either forced us to exhaustively hard-code data types or operations. Then we used generics to pass down a god `AstNode` type that supports all the data types and operations, and each module could specify what data types it produces and which operations it consumes with generic bounds. We then used generics to simplify the definition of the god type, requiring us to only enumerate either crates providing data types or crates consuming operations.

So... why wasn't this solution obvious from the very beginning, and why is there still an "either-or" clause?

To answer the former question, consider *where* these data types/operations were hard-coded. Before we started passing `Node` down the crate hierarchy, the entire range of existing data types used at any moment during the program had to be hard-coded *in each dependency* -- or, rather, in their common dependency, e.g. `mylang-core`. Similarly, the operations used by *any* crate would be hard-coded in *every* crate that created new data types, or perhaps at the bottom of the crate hierarchy, rather than at the top. Simply merging `enum`s or `trait`s together to list crates instead of data types/operations wouldn't help, because this would just cause each crate to hard-code the name of every other crate, which is not an improvement.

For the latter, consider that `AstNode` acts as a broker between producers of data types and consumers of operations, and if some data type doesn't implement a certain operation, then one of these two links needs to be broken. Hard-coding crates providing data types vs crates consuming operations is just an arbitrary choice of which link to break. Ideally, we'd remove `AstNode` altogether and link producers to consumers directly, and in dynamically typed languages, we can *just do that*, because there's no need to decide on a single static type agreed upon by all crates. But Rust is statically typed, and so we're forced to provide this broker type.


### Con(cl|f)usion

This is a weird pattern. It *works*, I guess, but I'm pretty sure I was happier before I realized how `CoerceUnsized` can be applied here. It feels like overengineering, and it probably is. Until Rust supports some of the necessary features natively, I'd say that the cost of maintaining this glue code and teaching people how to work with types like `AstNode` can easily outweigh the benefits of orthogonality.

So, honestly, maybe you just shouldn't do that. If you want an idiomatic solution, you should probably look for a dynamically typed language. But sometimes you simply don't need all the expressive power and are better off committing to opinionated choices. And at least for Rust, I think that's the best way forward.
