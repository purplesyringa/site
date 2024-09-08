---
title: You might want to use panics for error handling
time: August 13, 2024
intro: |
    Rust's approach to error handling comes at a cost. The `Result` type often doesn't fit in CPU registers, and callers of fallible functions have to check whether the returned value is `Ok` or `Err`. That's a stack spill, a comparison, a branch, and a lot of error handling code intertwined with the hot path that *just shouldn't be here*, which inhibits inlining, the most important optimization of all.
    
    Exceptions and panics make it easy to forget about the occasional error, but they don't suffer from inefficiency. Throwing an exception unwinds the stack automatically, without any cooperation from the functions except the one that throws the exception and the one that catches it. Wouldn't it be *neat* if a mechanism with the performance of `panic!` and the ergonomics of `Result` existed?
---

Rust's approach to error handling is neat, but it comes at a cost. Fallible functions return this type:

```rust
// A sum type. Defined in the standard library.
enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

So the `Result` type is almost always larger than the actual returned value:

```
                                     Discriminant
                                          vv
                                     +-----------+--------------------------+
                       Ok variant:   | 0x00...00 |       actual data        |
                                     +-----------+--------------------------+

                                     +-----------+--------------------------+
                       Err variant:  | 0x00...01 |       actual error       |
                                     +-----------+--------------------------+
```

Oftentimes it doesn't fit in CPU registers, so it has to be spilled to stack.

Callers of fallible functions have to check whether the returned value is `Ok` or `Err`:

```rust
// What the programmer writes:
f()?
// What the compiler sees:
match f() {
    Ok(value) => value, // Handle the Ok output
    Err(err) => return Err(err), // Forward the error
}
```

That's a comparison, a branch, and a lot of error handling code intertwined with the hot path that *just shouldn't be here*. And I don't mean that lightly: large code size inhibits inlining, the most important optimization of all.

### Alternatives

Checked exceptions -- the closest thing there is to `Result`s -- have different priorities. They simplify the success path at the expense of the failure path, so it's easy to forget about the occasional error. This is an explicit anti-goal of Rust.

Rust has panics that use the same mechanism, but guides against using them for fallible functions, because they are almost unusable for that purpose:

```rust
//                     vvv  Does not specify the error type.
fn produces(n: i32) -> i32 {
    if n > 0 {
        n
    } else {
        panic!("oopsie")
    }
}
// Compare with Result:       vvvvvvvvvvvvvvvvvvvvvvvvv
fn produces_result(n: i32) -> Result<i32, &'static str> {
    if n > 0 {
        Ok(n)
    } else {
        Err("oopsie")
    }
}

fn forwards(n: i32) -> i32 {
    //                 v  Implicitly forwards the error.
    let a = produces(n);
    let b = produces(n + 1);
    a + b
}
// Compare with Result:
fn forwards_result(n: i32) -> Result<i32, &'static str> {
    //                        v  Requires a simple but noticeable sigil.
    let a = produces_result(n)?;
    let b = produces_result(n + 1)?;
    Ok(a + b)
}

fn catches(n: i32) -> i32 {
    //   vvvvvvvvvvvvvvvvvvv  What?
    std::panic::catch_unwind(|| forwards(n)).unwrap_or(0)
}
// Compare with Result:
fn catches_result(n: i32) -> i32 {
    forwards_result(n).unwrap_or(0)
}
```


### Forbidden fruit

However, panics don't suffer from inefficiency! Throwing an exception unwinds the stack automatically, without any cooperation from the functions except the one that throws the exception and the one that catches it.

Wouldn't it be *neat* if a mechanism with the performance of `panic!` and the ergonomics of `Result` existed?


### #[iex]

I'm quite familiar with the Rust macro ecosystem, so I devised a way to [fix that with a crate](https://docs.rs/iex/latest/iex/). Here's how it works, roughly:

```rust
//        vvv  Import a macro from the iex crate.
use iex::{iex, Outcome};

#[iex]
//                     vvvvvvvvvvvvvvvvvvvvvvvvv  The signature includes the error...
fn produces(n: i32) -> Result<i32, &'static str> {
    if n > 0 {
        Ok(n)
    } else {
        Err("oopsie")
    }
}
// ...but this code is actually compiled to:
// fn produces(n: i32) -> i32 {
//     if n > 0 {
//         n
//     } else {
//         // vvvvvvvv  ✨ Magic ✨. Don't worry about it. Actually throws a panic.
//         throw_error("oopsie")
//     }
// }

#[iex]
fn forwards(n: i32) -> Result<i32, &'static str> {
    //                 v  The code is rewritten to rely on unwinding instead of matching.
    let a = produces(n)?;
    let b = produces(n + 1)?;
    Ok(a + b)
}

fn catches(n: i32) -> i32 {
    //         vvvvvvvvvvvvvv  Switch back to Result.
    forwards(n).into_result().unwrap_or(0)
}
```

This was just a joke experiment at first. It *should* work quite efficiently. Microbenchmarks are bound to show that.

But the design allows `Result`-based code to work with `#[iex]` with minimal changes. So I can slap `#[iex]` on a *real* project and benchmark it on *realistic data*.

### Benchmarks

:::aside
The code [is available](https://github.com/orgs/iex-rs/repositories) for independent reproduction.
:::

One simple commonly used project is [serde](https://serde.rs). After fixing some glaring bugs, I got these benchmark results on JSON deserialization tests:

<table>
    <thead>
        <tr>
            <td rowspan="2">Speed (MB/s, higher is better)</td>
            <th colspan="2"><code>canada</code></th>
            <th colspan="2"><code>citm_catalog</code></th>
            <th colspan="2"><code>twitter</code></th>
        </tr>
        <tr>
            <th>DOM</th>
            <th>struct</th>
            <th>DOM</th>
            <th>struct</th>
            <th>DOM</th>
            <th>struct</th>
        </tr>
    </thead>
    <tbody>
        <tr>
            <td><code>Result</code></td>
            <td align="center">282.4</td>
            <td align="center">404.2</td>
            <td align="center">363.8</td>
            <td align="center">907.8</td>
            <td align="center">301.2</td>
            <td align="center">612.4</td>
        </tr>
        <tr>
            <td><code>#[iex] Result</code></td>
            <td align="center">282.4</td>
            <td align="center">565.0</td>
            <td align="center">439.4</td>
            <td align="center">1025.4</td>
            <td align="center">317.6</td>
            <td align="center">657.8</td>
        </tr>
        <tr>
            <td>Performance increase</td>
            <td align="center">0%</td>
            <td align="center">+40%</td>
            <td align="center">+21%</td>
            <td align="center">+13%</td>
            <td align="center">+5%</td>
            <td align="center">+7%</td>
        </tr>
    </tbody>
</table>

<aside-inline-here />

This might not sound like a lot, but that's a *great* performance increase *just* from error handling. And this is a universal fix to a global problem.

### That includes you

To be clear, this benchmark only measures the success path. In realistic programs, the error path may be reached more often than the success path in some cases, so this is not a generic optimization.

However, it is applicable in almost every project to some degree: for example, querying a database is almost always successful. Optimizing such paths is trivial with `#[iex]`:

- Slap `#[iex]` onto all functions that return `Result`,
- Whenever you need to match on a `Result` or apply a combinator, try to rewrite code without that, and if you can't, add `.into_result()`,
- Occasionally replace `return e` with `return Ok(e?)` for... reasons.

### Afterword

`#[iex]` is a very young project. It might not be the best solution for production code, and it would certainly be great if rustc supported something like a `#[cold_err]` attribute to propagate errors by unwinding without external crates.

But I think it's a move in the right direction.

[The crate documentation](https://docs.rs/iex/latest/iex/) includes instructions on how to use `#[iex]` in your project. If you find this library useful, please tell me [on the issue tracker](https://github.com/iex-rs/iex/issues).
