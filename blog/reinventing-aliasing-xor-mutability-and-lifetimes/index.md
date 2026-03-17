---
title: Reinventing aliasing XOR mutability and lifetimes
time: March 17, 2026
discussion: https://www.reddit.com/r/rust/comments/1rw4h2q/reinventing_aliasing_xor_mutability_and_lifetimes/
intro: |
    Two years ago, I was pondering ways to enhance borrowck with proof-based analysis. At the time, I was only aware of formal verification tools for *functional* languages, while Rust is impure. For the purposes of borrow checking, though, we can almost pretend that impurity doesn't exist, with the notable exception of mutable references.

    Can mutable references be simulated in a functional language? Representing memory as a byte array doesn't compose: while call order is usually irrelevant in functional code, adding address-indexed memory to the mix requires proving non-aliasing even in trivial cases. And using path-indexed memory, like `root.var.field[index]`, doesn't work for stack locals and objects with complex ownership.

    I couldn't find a fix and dropped the topic. Until a few days ago, when I needed to implement mutable references in functional code for a pet project and realized just how instrumental aliasing XOR mutability and lifetimes are in achieving this.
---

Two years ago, I was pondering ways to enhance borrowck with proof-based analysis. At the time, I was only aware of formal verification tools for *functional* languages, while Rust is impure. For the purposes of borrow checking, though, we can almost pretend that impurity doesn't exist, with the notable exception of mutable references.

Can mutable references be simulated in a functional language? Representing memory as a byte array doesn't compose: while call order is usually irrelevant in functional code, adding address-indexed memory to the mix requires proving non-aliasing even in trivial cases. And using path-indexed memory, like `root.var.field[index]`, doesn't work for stack locals and objects with complex ownership.

I couldn't find a fix and dropped the topic. Until a few days ago, when I needed to implement mutable references in functional code for a pet project and realized just how instrumental aliasing XOR mutability and lifetimes are in achieving this.

**An important disclaimer**: while my method captures many operations with mutable references correctly, there's a missing detail that makes it unsuitable for typical Rust code. I don't know if it can be fixed, and I'd like to hear your thoughts on the topic.

Regardless, it feels odd how a tool invented for Rust applies so well to functional code, and I hope you'll agree the similarity is eerie once you read this post. I think there's some convergence here: my approach is reminiscent of [Polonius](https://smallcultfollowing.com/babysteps/blog/2018/04/27/an-alias-based-formulation-of-the-borrow-checker/), the borrow checker that will replace the current NLL, to the point where when [I first heard of how borrowck works in rustc](https://www.youtube.com/watch?v=_agDeiWek8w), I was confused why it wasn't Polonius from day one.


### Simple case

Suppose we want to rewrite this function without references:

```rust
fn f(r: &mut i32) {
    *r += 1;
}
```

An easy way to do this is to receive the old value of `r` and return the new one:

```rust
fn f_rewritten(r: i32) -> i32 {
    r + 1
}
```

Callers can then replace `f(&mut x)` with `x = f_rewritten(x)`. Note that I allow assignment to locals and focus specifically on removing references. In other words, my goal is only to make all functions pure, not change the programming paradigm. Replacing imperative constructs like `if` and `for` statements and reassignments can occur as a later stage if necessary.

With more input references, we just take and return more values:

```rust
fn f(r1: &mut i32, r2: &mut i32) {
    *r1 += 1;
    *r2 += 1;
}

fn f_rewritten(r1: i32, r2: i32) -> (i32, i32) {
    (r1 + 1, r2 + 1)
}
```

This rewrite works as long as `r1` and `r2` don't alias. This reminds me of *unique ownership*: an alternative formulation for what we're doing here is that taking a mutable reference temporarily transfers ownership of the value to the reference. A place cannot be mutably borrowed twice at the same time, or we'll encounter stale data, and for the same reason, the value can't be read from the original location until the mutable reference dies. This is the same check that borrowck implements.


### 1st class type

For references to be true types, we need to be able to put them in containers and move them around. For example:

```rust
fn f(r1: &mut i32, r2: &mut i32) {
    let mut vec: Vec<&mut i32> = vec![r1, r2];
    shuffle(&mut vec);
    *vec[0] += 1;
    *vec[1] += 2;
}
```

To rewrite this, we need to somehow know which order to return the new values in:

```rust
fn f_rewritten(r1: i32, r2: i32) -> (i32, i32) {
    let mut vec: Vec<i32> = vec![r1, r2];
    vec = shuffle(vec);
    vec[0] += 1;
    vec[1] += 2;
    // (vec[0], vec[1])? (vec[1], vec[0])?
}
```

To avoid tracking it manually, let's return an unordered set of mutated references, and require the caller to add a *source marker* to reference values to distinguish them:

```rust
fn f_rewritten(r1: (Source, i32), r2: (Source, i32)) -> Vec<(Source, i32)> {
    let mut vec: Vec<(Source, i32)> = vec![r1, r2];
    vec = shuffle(vec);
    vec[0].1 += 1;
    vec[1].1 += 2;
    vec
}

fn example_call() -> i32 {
    let mut a = 1;
    let mut b = 2;
    f(&mut a, &mut b);
    a - b
}

fn example_call_rewritten() -> i32 {
    let mut a = 1;
    let mut b = 2;
    for (source, new_value) in f_rewritten((Source::A, a), (Source::B, b)) {
        match source {
            Source::A => a = new_value,
            Source::B => b = new_value,
        }
    }
    a - b
}
```

Since the caller adds the marker, not the callee, the rewritten argument and return types match, and there is no special-casing. `(Source, T)` becomes our representation of `&mut T`, and we require that functions return all references from their arguments:

```rust
struct RefMut<T> {
    source: Source,
    value: T,
}
```


### Linearity

We're assuming here that references passed into a function can always be recovered and returned back -- that is, they are not lost or forgotten. This is the concept of [linear types](https://en.wikipedia.org/wiki/Substructural_type_system#Linear_type_systems). Put simply, values of linear types need to be "destroyed", whatever this means, not just *at most* once (that's just memory safety), but *exactly* once. For mutable references, this corresponds to write-back at places where lifetimes run out.

This may look limiting -- for instance, it makes [`std::mem::forget`](https://doc.rust-lang.org/std/mem/fn.forget.html) unsound -- but it's actually not that bad. Rust already inserts calls to drop glue when values need to be destroyed to prevent memory leaks, and that covers everything but `Rc`/`Arc` and `unsafe` code. And putting mutable references in `Rc` is not that common.

So all we need to do is replace drop glue with something that can surface references into callers, so that references are consumed in scopes where the corresponding locals can be updated. We'll discuss how exactly to do this in a bit.


### Reborrowing

For now, let me show you something cool. Let's pass a reference across functions:

```rust
fn f(r: &mut i32) { ... }

fn g(r: &mut i32) {
    f(r);
    f(r);
}
```

In Rust, `&mut T` is `!Copy`, so it looks like the first call `f(r)` should move `r` into `f`'s arguments and make `r` unusable for the rest of `g`. Luckily, due to [reborrowing](https://haibane-tenshi.github.io/rust-reborrowing/), the first call is effectively rewritten to `f(&mut *r)`, so this works.

In functional code, such reborrowing is unnecessary <img class="emoji" src="/images/ferrisClueless.webp" title=":ferrisClueless:" /> because we have linear types, so `f` is guaranteed to return `r` back:

```rust
fn f_rewritten(r: RefMut<i32>) -> RefMut<i32> { ... }

fn g_rewritten(mut r: RefMut<i32>) -> RefMut<i32> {
    r = f_rewritten(r);
    f_rewritten(r)
}
```


### Sources

It's better not to think of `Source` as any specific type. It should just be something that uniquely points to a borrow expression in the source code of the function that created the reference. For example:

```rust
fn f(r: &mut i32) { ... }
fn f_rewritten(r: RefMut<i32>) -> RefMut<i32> { ... }

fn example_call() {
    let mut a = [[1, 2], [3, 4]];
    f(&mut a[0][0]);
}
fn example_call_rewritten() {
    let mut a = [[1, 2], [3, 4]];
    a = [[f(RefMut { source: ???, value: a[0][0] }).value, a[0][1]], a[1]];
}
```

The source here doesn't have to include the entire path `a[0][0]`, it only needs to distinguish this reference from other references passed to `f`. In this case, there are no such references, so it can be empty.


### Lifetimes

A source does not need to distinguish between *all* borrow expressions in the caller. We originally introduced sources because mutable references could be reordered. But if they have different lifetimes, they cannot be swapped, and so there is no need to distinguish them with runtime values:

```rust
fn f<'a, 'b, T>(r1: &'a mut T, r2: &'b mut T) {
    // doesn't compile: core::mem::swap(r1, r2);
}

// Always returns reference associated with `'a` in `.0`, reference associated with `'b` in `.1`.
fn f_rewritten<T>(r1: RefMut<T>, r2: RefMut<T>) -> (RefMut<T>, RefMut<T>) { ... }
```

So a source corresponds to all borrow expressions with a given lifetime. With a sleight of hand, we can associate a lifetime with a set of borrow expressions it includes, and say that `&'a mut T` is just `('a, T)`.

This is actually less crazy than it sounds. For Polonius, a lifetime *is* a set of borrow expressions. (For comparison, NLL says that a lifetime is a program location where the reference may be accessed, that is, it tracks consumers rather than producers.)


### Dropping

We can now figure out how to bubble up dropped references. Each function needs to return, for each generic lifetime parameter, the set of dropped references associated with this parameter:

```rust
fn f<'a, 'b, T>(r1: &'a mut T, r2: &'b mut T) {}

fn f_rewritten<'a, 'b, T>(r1: ('a, T), r2: ('b, T)) -> (Set<'a>, Set<'b>) {
    (Set::new([r1]), Set::new([r2]))
}
```

When `f` is invoked, its caller knows which lifetimes correspond to borrows within the caller, and which lifetimes forward external borrows. It can then consume some references within the function, and merge others into its own returned set:

```rust
fn g<'a>(r1: &'a mut i32) {
    let mut x = 123;
    f(r1, &mut x);
}

fn g_rewritten<'a>(r1: ('a, i32)) -> Set<'a> {
    let mut x = 123;
    let (set_a, set_local) = f(r1, (Source::X, x));
    for (source, new_value) in set_local {
        match source {
            Source::X => x = new_value,
        }
    }
    set_a
}

fn h() {
    let mut x = 123;
    let mut y = 456;
    f(&mut x, &mut y);
}

fn h_rewritten() {
    let mut x = 123;
    let mut y = 456;
    let (set_x, set_y) = f((Source::X, x), (Source::Y, y));
    for (source, new_value) in set_x {
        match source {
            Source::X => x = new_value,
        }
    }
    for (source, new_value) in set_y {
        match source {
            Source::Y => y = new_value,
        }
    }
}

fn i<'a>(r1: &'a mut i32, r2: &'a mut i32) {
    f(r1, r2);
}

fn i_rewritten<'a>(r1: &'a mut i32, r2: &'a mut i32) -> Set<'a> {
    let (set_a, set_b) = f(r1, r2);
    set_a.merge(set_b)
}
```

Drop glue can behave similarly, returning the per-lifetime sets of dropped references stored within the dropped value.


### Projections

And now for the part that this model doesn't handle well. Rust allows code like this:

```rust
fn f(r: &mut (i32, i32)) -> &mut i32 {
    r.0 += 1;
    &mut r.1
}
```

We can return a reference to a subfield of the passed value, while mutating the original value. In this case, there is no one to perform the write-back, because `f` has already completed by the time the returned `&mut i32` is destroyed. And we can't just write-back `r` after calling `f` because it can still be modified through the field reference.

So the new value of `r`, the source of `r`, and the field name can only be stored in the returned `&mut i32` -- more specifically, in its source:

```rust
enum Source {
    Projection(RefMut, Field),
    ...
}

fn f_rewritten(r: RefMut<(i32, i32)>) -> RefMut<i32> {
    r.value.0 += 1;
    (Source::Projection(r, 1), r.value.1)
}
```

When the field reference is destroyed, we update everything up to the original object:

```rust
for r in set {
    while let Self::Projection(r, field) = r.source {
        r.value[field] = r.value;
        r = r.to_base();
    }
    match r.source {
        ...
    }
}
```

This ties into some of my other points:

- When talking about lifetimes, I brought up separating references into sets by lifetimes, but I ignored pointee types. This is because when projections are added to the mix, the types of the original pointee and the current pointee need not agree, so this might draw a distinction that doesn't exist.

- Simulating reborrows with linear types becomes trickier: when calling `f_rewritten(r)` in question with `r` from an argument, you can't do write-back it with `while let ...` because `r` itself might be a projection, so you either need `while r.source != original_source`, or perform reborrowing explicitly with `f(&mut *r)`. The latter is simpler and closer to how Rust actually behaves.

Anyway, this is hacky, but at least it seemingly works. The issue is that it fails spectacularly when more than one field is projected:

```rust
fn f(r: &mut (i32, i32, i32)) -> (&mut i32, &mut i32) {
    r.0 += 1;
    (&mut r.1, &mut r.2)
}
```

With my design, you'd have to duplicate `r` to return two projected references, which would cause collisions on write-back. Worst-case scenario, we can have a highly nested tree of updates that need to be committed at once. Maybe we could look at the set of dying references to recover the tree? But this requires object-addressed maps, i.e. either a hash map (slow), or straight up mutating objects (impure).


### Conclusion

This is just a quick note, so I haven't really thought it through.

It obviously demonstrates that it's possible to simulate mutable references almost entirely in functional code. Also, this ties neatly into the linear type-based aliasing story by showing that it's mostly equivalent to lifetimes (e.g. by [the Vale guy](https://verdagon.dev/blog/linear-types-borrowing)).

Whether this has other applications or I'm reinventing the wheel, I don't know.
