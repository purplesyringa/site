---
title: Rust's v0 mangling scheme in a nutshell
time: December 15, 2025
discussion:
  - https://www.reddit.com/r/rust/comments/1pn08ja/v0_mangling_scheme_in_a_nutshell/
  - https://lobste.rs/s/khozwx/rust_s_v0_mangling_scheme_nutshell
intro: |
    Functions in binary files need to have unique names, so Rust needs to decide what to call its functions and `static`s. This format needs to be standardized so that debuggers and profilers can recover the full names (e.g. `alloc::vec::Vec` instead of just `Vec`).

    About a month ago, [Rust switched to symbol mangling v0 on nightly](https://blog.rust-lang.org/2025/11/20/switching-to-v0-mangling-on-nightly/). The linked announcement describes some benefits of the new scheme compared to the previous ad-hoc hack:

    - Mangled names of generic functions now include generic parameters.
    - There are almost no opaque hashes, meaning that it's easier to make a hypothetical alternative Rust compiler produce identical mangled names.
    - Mangled names no longer include characters like `$` and `.`, which some platforms don't support.

    That's pretty interesting, but not very deep. I want to highlight some non-obvious details that weren't mentioned in the post.
---

:::aside
[Original meme](https://social.linux.pizza/@detondev/114942195938099088)
:::

![Picture of a nutshell captioned "v0 mangling scheme in a nutshell". A speech bubble with text "Help!" indicates a sound from inside the nutshell. An example of mangled symbol (_RNvCs15kBYyAo9fc_7mycrate7example) is superimposed on the nutshell.](meme.png)

<aside-inline-here />

Functions in binary files need to have unique names, so Rust needs to decide what to call its functions and `static`s. This format needs to be standardized so that debuggers and profilers can recover the full names (e.g. `alloc::vec::Vec` instead of just `Vec`).

About a month ago, [Rust switched to symbol mangling v0 on nightly](https://blog.rust-lang.org/2025/11/20/switching-to-v0-mangling-on-nightly/). The linked announcement describes some benefits of the new scheme compared to the previous ad-hoc hack:

- Mangled names of generic functions now include generic parameters.
- There are almost no opaque hashes, meaning that it's easier to make a hypothetical alternative Rust compiler produce identical mangled names.
- Mangled names no longer include characters like `$` and `.`, which some platforms don't support.

That's pretty interesting, but not very deep. I want to highlight some non-obvious details that weren't mentioned in the post.


### Why v0?

Why is the old mangling called `legacy` and the new mangling is called `v0`, instead of the more sensible `v1` and `v2`?

The new standard includes the mangling version in the symbol name. If the scheme ever needs to be updated, the general encoding structure will be reused and the version field will be incremented. The distinction is not between old and new schemes, but rather between the pre-versioning and post-versioning eras. The current version is 0.


### Punycode

[Punycode](https://en.wikipedia.org/wiki/Punycode) is used to fit identifiers with Unicode into the `[a-zA-Z0-9_]` range. You're likely familiar with Punycode from DNS, which only supports pure-ASCII hostnames. For example, `münchen.de` is encoded as `xn--mnchen-3ya.de`.

Unlike `base64`, Punycode keeps the ASCII portion of the string readable (`mnchen` in the previous example) and only encodes the non-ASCII subsequence. This improves human readability of mangled symbols. Punycode is also highly optimized for space.


### Base-62

Most integers (`const` generic parameters, array sizes, crate IDs, etc.) are encoded in base-62 for compactness. As an exception, bytes of `const` values are encoded in hex for stream parsing, and identifiers are prefixed with their length in base 10: since identifiers can't start with decimal digits, this saves a byte by avoiding a separator.


### Backrefs

To reduce repetitions within the symbol, `B<offset>` can be used to repeat the value at position `offset` from the beginning of the mangled name. Compared to the Itanium ABI used by C++, which addresses AST nodes instead of byte positions, this allows v0 symbols to be demangled without allocation.


### Disambiguators

"Disambiguators" are opaque numbers that ensure uniqueness of objects that would otherwise have identical names. This is used for closures (which don't have names by definition), different versions of the same crate, and methods in `impl` blocks with different `where` bounds.

```rust
// Both `foo` methods are called `<T as Trait>::foo`, so a disambiguator is necessary.

impl<T> Trait for T
where
    T: Trait2<Assoc = i8>
{
    fn foo() { /* impl 1 */ }
}

impl<T> Trait for T
where
    T: Trait2<Assoc = u32>
{
    fn foo() { /* impl 2 */ }
}
```


### Primitives

Primitive types are encoded with a single letter:

- `a` = `i8`
- `b` = `bool`
- `c` = `char`
- `d` = `f64`
- `e` = `str`
- ...
- `z` = `!`

`d` clearly stands for `double`, but what does `e` mean?

For types defined in C, the mapping was directly taken from the Itanium ABI. For the rest of the types, the letters were assigned mostly sequentially. `c` corresponds to `char` in both standards, even though the types are very different.


### Placeholders

Generic parameters allow v0 to encode names like `<i32 as Add<i32>>::add`. But consider the `STATIC` in:

```rust
impl<T, U> Trait<T> for U {
    fn f() {
        static STATIC: i32 = 0;
    }
}
```

Since `STATIC` isn't monomorphized, it will be named `<_ as Trait<_>>::f::STATIC` with a placeholder instead of generic parameters.


### Lifetimes

Due to HRTB, two types can be distinct in runtime, but only differ in lifetimes. Compare:

```rust
type T1 = for<'a> fn(&'a mut i32, &'a mut i32);
type T2 = for<'a, 'b> fn(&'a mut i32, &'b mut i32);
```

In v0, "binders" can define anonymous lifetimes, much like `for` in surface Rust syntax, and there is syntax for mentioning such lifetimes by index.
