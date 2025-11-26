---
title: A look at Rust from 2012
time: November 25, 2025
discussion:
    - https://www.reddit.com/r/rust/comments/1p6k380/a_look_at_rust_from_2012/
    - https://lobste.rs/s/0rvqxt/look_at_rust_from_2012
intro: |
    Recently I was scrolling through [brson's Rust quote database](https://brson.github.io/rust-qdb/) and stumbled upon a link to the [official Rust tutorial](https://web.archive.org/web/20130118053116/http://static.rust-lang.org/doc/tutorial.html) from the very beginning of 2013. It says Rust 0.6 in the corner, but it lists many things that were removed in 0.6, so it's likely closer to 0.5.

    I heard tales of old Rust before, but not of how the language felt to programmers. So I thought it'd be cool to give a (relatively) quick summary of Rust as presented in the tutorial and yap a bit about how far we've come since then.
---

Recently I was scrolling through [brson's Rust quote database](https://brson.github.io/rust-qdb/) and stumbled upon a link to the [official Rust tutorial](https://web.archive.org/web/20130118053116/http://static.rust-lang.org/doc/tutorial.html) from the very beginning of 2013. It says Rust 0.6 in the corner, but it lists many things that were removed in 0.6, so it's likely closer to 0.5.

I heard tales of old Rust before, but not of how the language felt to programmers. So I thought it'd be cool to give a (relatively) quick summary of Rust as presented in the tutorial and yap a bit about how far we've come since then.


### Installation

First impressions matter, and Rust doesn't disappoint:

> The Rust compiler currently must be built from a tarball, unless you are on Windows, in which case using the installer is recommended.

...followed by the classical `./configure && make && make install` tutorial. The building process also relied on Python 2.6. Installing Rust on Windows also required manually installing MinGW. Modern `rustup` is a blessing!


### Hello, world!

Here's our "Hello, world!":

```rust
fn main() {
    io::println("hello?");
}
```

`io` was part of `core`, and modules from `core` were globally visible. There was no `alloc`, so e.g. `vec` was part of `core`. The difference between `core` and `std` was more about low vs high level than objective limitations.

There were no pretty errors yet -- [the helpful diagnostics were a later addition](https://kobzol.github.io/rust/rustc/2025/05/16/evolution-of-rustc-errors.html):

```
hello.rs:2:4: 2:16 error: unresolved name: io::print_with_unicorns
hello.rs:2     io::print_with_unicorns("hello?");
               ^~~~~~~~~~~~~~~~~~~~~~~
```

There was no `println!`, but there was `fmt!`, which took an `sprintf`-like format string (glad we moved away from that):

```rust
io::println(fmt!("%s is %d", "the answer", 43));

// %? will conveniently print any type
io::println(fmt!("what is this thing: %?", mystery_object));
```

On the topic of macros, it's surprising how little [the `macro_rules!` syntax](https://web.archive.org/web/20130225044101/http://static.rust-lang.org/doc/tutorial-macros.html) has changed. Present-day macros were called "syntax extensions", and "macros" only referred to declarative macros.


### Syntax

IMO, the book focused too much on syntax and not enough on ownership and borrowing -- which makes sense, since the current model didn't exist back then. Modern Rustbook gets to the point faster and does a better job integrating realistic examples between sections.

`usize` was written `uint` and `isize` was written `int`, which I can imagine causing much confusion to C developers. Unconstrainted integer literals defaulted to `int` instead of `i32`. `()` was inconsistently called "nil type" or "unit type".

There was a Python-style `assert` statement:

```rust
let x: float = 4.0;
let y: uint = x as uint;
assert y == 4u;
```

`continue` was called `loop` for some reason:

> Inside a loop, the keyword `break` aborts the loop, and `loop` aborts the current iteration and continues with the next.

`enum` variants were unscoped, just like in C:

> ```rust
> enum Direction {
>     North,
>     East,
>     South,
>     West
> }
> ```
>
> This declaration defines `North`, `East`, `South`, and `West` as constants, all of which have type `Direction`.

Since the variants were unscoped, `enum`s could be used to simulate tuple-like structs:

> There is a special case for enums with a single variant, which are sometimes called "newtype-style enums" (after Haskell's "newtype" feature). [...] If you say:
>
> ```rust
> enum GizmoId = int;
> ```
>
> That is a shorthand for this:
>
> ```rust
> enum GizmoId { GizmoId(int) }
> ```

Why was this useful? As far as I can tell, neither tuples nor tuple-like structs could have fewer than 2 elements! `(T,)` didn't exist, and `()` wasn't considered a tuple. There was no `.0` syntax, so you had to use destructuring to access tuple contents. Alternatively, newtype-style enums could be dereferenced with `*`.

We're getting ahead of ourselves, but there was a `copy` operator instead of `.clone()`:

> If you really want to copy an owned box you must say so explicitly.
>
> ```rust
> let x = ~10; // NOTE(purplesyringa): don't worry about it :)
> let y = copy x;
>
> let z = *x + *y;
> assert z == 20;
> ```

All arrays were called "vectors". `[T; N]` was `[T * N]`, eventually changed to enable the `[expr; N]` syntax:

```rust
// A fixed-size stack vector
let stack_crayons: [Crayon * 3] = [Almond, AntiqueBrass, Apricot];
```

Trait implementations were written `impl Type: Trait`. I actually quite like it.


```rust
impl TimeBomb : Drop {
    fn finalize(&self) {
        for iter::repeat(self.explosivity) { // NOTE(purplesyringa): don't mind this :)
            io::println("blam!");
        }
    }
}
```

`Drop`'s method was called `finalize`, which will make sense in a bit.

`Self` was written `self`, which added confusion:

```rust
// In a trait, `self` refers both to the self argument
// and to the type implementing the trait
trait Eq {
    fn equals(&self, other: &self) -> bool;
}
```

There were no pluses between trait bounds:

```rust
fn print_all<T: Printable Copy>(printable_things: ~[T]) {
    // [...]
}
```

Before `use path as alias`, there was `use alias = path`. I don't know which one I prefer: `as` allows multiple imports to be on one line, but why isn't it spelled `:` like in patterns?

```rust
// Bring `chicken` into scope
use farm::chicken;

fn chicken_farmer() {
    // The same, but name it `my_chicken`
    use my_chicken = farm::chicken;
    ...
}
```


### Trait objects

There was no `dyn Trait`, just `Trait`, so it wasn't explicit which pointers were fat. This was abused: instead of `Fn*` traits, there was `fn()`, roughly identical to `dyn FnMut()`. You'd usually write `&fn(...) -> ...` as a callback type. `move` in closures was inferred.

I think `&` before `fn()` was implied if there was no sigil, but you also didn't have to write `&` in the callee, so call sites looked just like today despite dynamic dispatch:

```rust
fn call_closure_with_ten(b: fn(int)) { b(10); }

let captured_var = 20;
let closure = |arg| println(fmt!("captured_var=%d, arg=%d", captured_var, arg));

call_closure_with_ten(closure);
```


### Blocks

Did you know that Rust had a feature for implementing control flow structures?

> The `do` expression provides a way to treat higher-order functions (functions that take closures as arguments) as control structures. [...] Consider this function that iterates over a vector of integers, passing in a pointer to each integer in the vector:
>
> ```rust
> fn each(v: &[int], op: fn(v: &int)) {
>     let mut n = 0;
>     while n < v.len() {
>         op(&v[n]);
>         n += 1;
>     }
> }
> ```
>
> As a caller, if we use a closure to provide the final operator argument, we can write it in a way that has a pleasant, block-like structure.
>
> ```rust
> each([1, 2, 3], |n| {
>     do_some_work(n);
> });
> ```
>
> This is such a useful pattern that Rust has a special form of function call that can be written more like a built-in control structure:
>
> ```rust
> do each([1, 2, 3]) |n| {
>     do_some_work(n);
> }
> ```

It's still supported by languages like Ruby and Kotlin, and it's pretty cool. But the *really* interesting implication of this pattern being natively supported is push iterators:

```rust
fn each(v: &[int], op: fn(v: &int) -> bool) { // NOTE(purplesyringa): named argument in `fn(...)`!
    let mut n = 0;
    while n < v.len() {
        if !op(&v[n]) {
            break;
        }
        n += 1;
    }
}

// [...]

for each([2, 4, 8, 5, 16]) |n| {
    if *n % 2 != 0 {
        println("found odd number!");
        break;
    }
}
```

The `for` loop uses the same mechanism, adding only a `bool` to support `break` and `return` from the loop body. Why did Rust switch to pull iterators? I don't know! I couldn't find any corroborating source, so I'd love to hear your thoughts.

> *Edited on November 26*: [This post on the mailing list](https://web.archive.org/web/20140716172928/https://mail.mozilla.org/pipermail/rust-dev/2013-June/004599.html) seemingly was the catalyst of this change. The argument was that multi-iterator transforms like `zip` didn't work with push iterators, that iterator state couldn't be saved (and would thus eventually be incompatible with async), and that pull iterators could be simulated with coroutines when those were added. Here we are, still waiting for that last bit.


### Tasks

Old Rust had green threads. I think it was closer to Erlang than any other language.

> Rust's lightweight tasks do not share memory, instead communicating through messages.
>
> (from [Rust Tasks and Communication Tutorial](https://web.archive.org/web/20130225044058/http://static.rust-lang.org/doc/tutorial-tasks.html)) Rust tasks have dynamically sized stacks. A task begins its life with a small amount of stack space (currently in the low thousands of bytes, depending on platform), and acquires more stack as needed.

Panics were called exceptions and were triggered with `fail!()`. They brought down the whole task, and there was no `std::panic::catch_unwind`, but you could spawn a lightweight task just to catch its panics:

```rust
let result: Result<int, ()> = do task::try {
    if some_condition() {
        calculate_result()
    } else {
        die!(~"oops!");
    }
};
assert result.is_err();
```

...though there was no `Box<dyn Any + Send + 'static>` error yet. Note the use of `do`.

There was a built-in `spsc` pipe, and tasks could automatically halt other tasks:

> In Rust parlance, a *channel* is a sending endpoint of a pipe, and a *port* is the receiving endpoint. [...] All tasks are, by default, *linked* to each other. That means that the fates of all tasks are intertwined: if one fails, so do all the others.

```rust
let (receiver, sender): (Port<int>, Chan<int>) = stream();
do spawn |move receiver| {  // Bidirectionally linked
    // Wait for the supervised child task to exist.
    let message = receiver.recv();
    // Kill both it and the parent task.
    assert message != 42;
}
do try |move sender| {  // Unidirectionally linked
    sender.send(42);
    sleep_forever();  // Will get woken up by force
}
// Flow never reaches here -- parent task was killed too.
```

The decision to remove tasks arguably shaped the Rust's future more than any other change. It eventually allowed Rust to drop the language runtime, allowing it to be integrated in embedded, OS kernels, and existing C codebases. And now that it's low-level enough, stackful coroutines can be brought back [with library code](https://docs.rs/may/latest/may/).


### Crates

There was no cargo and thus no `Cargo.toml`. Crate metadata was specified in the root file, called `<cratename>.rc`, which acted like today's `lib.rs`/`main.rs`:

```rust
// Crate linkage metadata
#[link(name = "farm", vers = "2.5", author = "mjh")];

// Make a library ("bin" is the default)
#[crate_type = "lib"];

// Turn on a warning
#[warn(non_camel_case_types)]

// Link to the standard library
extern mod std;

// Load some modules from other files
mod cow;
mod chicken;
mod horse;

fn main() {
    ...
}
```

Note the explicit linking to `std` and the use of `extern mod` instead of `extern crate`. It could also search crates by specific criteria:

```rust
extern mod farm;
extern mod my_farm (name = "farm", vers = "2.5");
extern mod my_auxiliary_farm (name = "farm", author = "mjh");
```

...though you had to compile them with `rustc` and pass the library path by hand.


### Structs

Since there was no `#[repr]`, all `struct`s were C-compatible:

> Structs are quite similar to C structs and are even laid out the same way in memory (so you can read from a Rust struct in C, and vice-versa).

`struct` fields could be marked as mutable with `mut`. This affected the rest of the type system: instead of `&` and `&mut` like we have today, there were `&`, `&mut`, and `&const`:

- `&const` was read-only, like today's `&`. You could take `&const` to any binding.
- `&mut` allowed replacing the entire object, like today's `&mut`. You could only take `&mut` to `let mut` bindings or `mut` fields, together known as *mutable memory*.
- `&` allowed modifying `mut` fields, but not immutable fields, and could only be taken to `let` bindings or immutable fields (immutable memory). This is why `&fn` allowed the closure to mutate its environment, for example. This also meant that adding mutability did not monotonically increase capabilities, i.e. `let` vs `let mut` affected [more than a lint](https://internals.rust-lang.org/t/lack-of-mut-in-bindings-as-a-deny-by-default-lint/15818).

`&` was reasonably universal and thus the "default" reference type. Most methods took `&self`, so the receiver parameter was optional. You would often see this [in the documentation](https://web.archive.org/web/20130121044500/http://static.rust-lang.org/doc/std/index.html). On the flip side, associated methods had to be annotated explicitly:

> Implementations may also define *static* methods, which don't have an explicit `self` argument. The `static` keyword distinguishes static methods from methods that have a `self`:
>
> ```rust
> impl Circle {
>     fn area(&self) -> float { ... }
>     static fn new(area: float) -> Circle { ... }
> }
> ```

Fields and methods were `pub` by default, so there was also the `priv` visibility:

```rust
mod farm {
    pub struct Farm {
        priv mut chickens: ~[Chicken],
        priv mut cows: ~[Cow],
        farmer: Human
    }

    // Note - visibility modifiers on impls currently have no effect
    impl Farm {
        priv fn feed_chickens(&self) { ... }
        priv fn feed_cows(&self) { ... }
        fn add_chicken(&self, c: Chicken) { ... }
    }

    // [...]
}
```


### Memory model

`&T` wasn't the only kind of references. The other two kinds, `@T` and `~T`, seem to be almost singlehandedly responsible for people's hate of sigils (sharing the throne with [modes](https://smallcultfollowing.com/babysteps/blog/2011/12/08/why-not-modes/), which were already phased out by 0.6).

`@T` corresponded to objects on the task-local garbage-collected heap. Such references could be freely copied, but not sent to other tasks. This is most similar to today's `Rc<T>` and simplified the garbage collector. `~T` was for global, sendable objects with a unique owner, i.e. `Box<T>`. Both could be converted to `&T`, which was not sendable, so the only way to communicate across tasks was with `~T`.

```rust
// A fixed-size stack vector
let stack_crayons: [Crayon * 3] = [Almond, AntiqueBrass, Apricot];

// A borrowed pointer to stack allocated vector
let stack_crayons: &[Crayon] = &[Aquamarine, Asparagus, AtomicTangerine];

// A local heap (managed) vector of crayons
let local_crayons: @[Crayon] = @[BananaMania, Beaver, Bittersweet];

// An exchange heap (owned) vector of crayons
let exchange_crayons: ~[Crayon] = ~[Black, BlizzardBlue, Blue];
```

The meaning of `~T`/`@T` was mostly controlled by the type `T`. `~[T]` corresponded to `Vec<T>`, not `Box<[T]>`. `String` was spelled `~str`. `@[T]`/`@str` didn't seem to work well:

> _**Note:**_ [...] Some operations on slices and stack vectors are not yet well-supported. Owned vectors are often the most usable.


### Lifetimes

There was no NLL. Lifetimes, back then often called "regions", were lexical and corresponded to specific blocks in source code:

```rust
fn example3() -> int {
    let mut x = ~{f: 3};
    if some_condition() {
        let y = &x.f;      // -+ L
        return *y;         //  |
    }                      // -+
    x = ~{f: 4};
    ...
}
```

Lifetime annotations looked like `&r/Point`, not `&'r Point`, where the lifetime name `r` didn't have to be explicitly listed as a generic parameter of the function:

```rust
struct Point {x: float, y: float}
fn get_x(p: &r/Point) -> &r/float { &p.x }
```

That was actually consistent, since types couldn't have lifetime parameters either. If you wanted to store pointers to local data, you'd use `@T` instead of `&T`.


### Aliasing

The rest of the post is me trying to make sense of [the tutorial on borrowing](https://web.archive.org/web/20130225044051/http://static.rust-lang.org/doc/tutorial-borrowed-ptr.html#borrowing-unique-boxes). It has fried my brain and negatively affected my skills in modern Rust, so be wary. I'm *soooo* happy [Niko Matsakis](https://smallcultfollowing.com/babysteps/blog/2012/11/18/imagine-never-hearing-the-phrase-aliasable/) replaced this mess with aliasing XOR mutability.

References were mostly used to track validity, not to prevent aliasing. Not even `&mut` implied unique access. You could take two `&mut` references to one object and write to both, or two `&` references and write to mutable fields through both. Old `&T` was most similar to today's `&UnsafeCell<T>`.

You might ask why writing through a `&T` (or `&mut T`) wasn't racy. Since `&T` was task-local, it must have been borrowed earlier in the same task from `@T` (also task-local) or `~T` (whose uniqueness guaranteed that only one task could access the object), so references could only alias within one task.

What about UAF? Since you couldn't take `&` to mutable memory, if you were given a `&T`, you'd know that the object wouldn't be replaced. Hence it was safe to project through `&T` to `struct` fields, `enum` variants, array elements, and `~`/`@` as long as there were no mutable fields or bindings in the projection path, as the `enum` variant couldn't be changed and the boxes could not be rebound without replacing the object.

If the path passed through `@T` in mutable memory, the `@T` was temporarily cloned locally for the duration of the borrow to ensure the refcount of the referenced object stayed positive, and mutability in that prefix could be ignored.

If mutable memory was still involved, the compiler made sure no operations could invalidate the borrow. Since such operations could only be task-local, borrowck only had to look for reassignments in the region where the borrow was taken:

```rust
fn example3() -> int {
    struct R { g: int }
    struct S { mut f: ~R }

    let mut x = ~S {mut f: ~R {g: 3}};
    let y = &x.f.g;
    x = ~S {mut f: ~R {g: 4}}; // Error reported here.
    x.f = ~R {g: 5};           // Error reported here.
    *y
}
```

If the new reference was obtained by only passing through fields and `~`, like in the previous example, it was guaranteed to be a unique path, and so borrowck could match paths straightforwardly. For example, this could get you from `~mut [T]` to `&T`.

But if the reference originated from `@` or `&`, the path might have been non-unique. To prevent the borrow from becoming dangling due to some reassignment through a different reference, mutations in the region were not allowed to use `@`/`&`. Permitted operations were called *pure* and could only access data owned by the current frame. You could annotate functions as `pure` to make them usable in this context; since their arguments were validated by the caller, the callee could access `&T`s from parameters:

```rust
struct R { g: int }
struct S { mut f: ~R }

pure fn add_one(x: &int) -> int { *x + 1 }

fn example5a(x: @S) -> int {
    let y = &x.f.g;
    add_one(y) // wouldn't be allowed without `pure`
}
```

As you can probably tell, different reference types didn't really compose. If you tried to go from `&~[T]` to `&T`, you could do that, but you were limited to pure functions to prevent the vector from being accidentally cleared. The fix was to use `~[T]` or `&[T]`.


### Conclusion

Compared to whatever we've just been through, I'm happy with how Rust turned out. It's in good hands. Thanks to all those who worked on it over the years and made it as user-friendly and simple as it is today.
