---
title: Finding duplicated code with tools from your CS course
time: November 17, 2025
intro: |
    Suppose that you're writing a static analyzer and you want to write a diagnostic for `match` arms with equal bodies:

    ```rust
    match number {
        1 => { // <--
            let x = 1;
            f(x)
        }
        2 => f(g(h())),
        3 => "",
        4 => { // <--
            let x = 1;
            f(x)
        }
        _ => unreachable!(),
    }
    ```
---

Suppose that you're writing a static analyzer and you want to write a diagnostic for `match` arms with equal bodies:

```rust
match number {
    1 => { // <--
        let x = 1;
        f(x)
    }
    2 => f(g(h())),
    3 => "",
    4 => { // <--
        let x = 1;
        f(x)
    }
    _ => unreachable!(),
}
```

Well, that looks simple enough: serialize each arm into a string and throw the strings into a hash map. Then someone renames a variable:

```rust
match number {
    1 => { // <--
        let x = 1;
        f(x)
    }
    2 => f(g(h())),
    3 => "",
    4 => { // <--
        let y = 1;
        f(y)
    }
    _ => unreachable!(),
}
```

Now the strings no longer match, but the arms are still clearly equivalent. Scary! It's not immediately obvious how to handle this correctly, let alone efficiently.

It turns out that this problem has interesting connections to the theory of computation and algebra. This is what this post is about: how odd tricks come together to tackle this challenge.


### Problem

We'll limit ourselves to syntactic analysis: snippets are considered equivalent if they match exactly, up to variable names. A smarter algorithm would realize that $2 + 2$ and $4$ are identical, but that's tricky in the general case (are `a + b` and `b + a` equivalent? what if it's an overloaded operator?), so we'll focus on something more rigid. There's still plenty to talk about.

We can't just ignore all variable names, since that's going to trigger false positives. But as we've just seen, we can't use them directly either.

One possibility is to abolish hash tables. Looking at two arms individually, we can record the correspondence between variables declared in one arm and the other, and then validate that all variable accesses match that correspondence:

<table>
    <thead>
        <tr>
            <th>Snippet A</th>
            <th>Snippet B</th>
            <th>Correspondence</th>
        </tr>
    </thead>
    <tbody>
        <tr>
            <td>
                <code>let x = 1;</code> (defines&nbsp;<code>x</code>)
            </td>
            <td>
                <code>let y = 1;</code> (defines&nbsp;<code>y</code>)
            </td>
            <td>
                <code>x</code> maps to <code>y</code>
            </td>
        </tr>
        <tr>
            <td>
                <code>f(x)</code> (uses&nbsp;<code>x</code>)
            </td>
            <td>
                <code>f(y)</code> (uses&nbsp;<code>y</code>)
            </td>
            <td>
                consistent
            </td>
        </tr>
    </tbody>
</table>

But that means that in a situation with, say, 20 arms, which is *exactly* the kind of situation where this diagnostic would be useful to catch copy-paste errors, we're going to need 200-ish comparisons to verify that all arms are unique. That's quite slow, even for a static analyzer. So how can we integrate this idea into the original hash table-based approach?

Luckily, we're not alone in our struggles. We don't have to solve this problem ourselves -- or, at least, this specific variation -- because that's already been done ages ago. The notion we've been struggling to formulate is called $\alpha$-equivalence. The only issue is that it's defined outside of "typical" programming. So I have two options:

- I can show you an ad-hoc solution that works only for this problem and is ultimately a dead end, or
- I can take you back through time and teach you something new.

Let's take a detour, shall we?


### Side quest

Back at the dawn of computation, when people like Alan Turing were only starting their research, it was not yet clear what the word "computation" was even supposed to mean.

Almost every system to date, including set theory and natural numbers, was based on techniques that were unusable for formalizing computation. Mathematicians often use first-order logic quantifiers like $\exists$ ("there exists a value ... such that ...") and $\forall$ ("for all values of ..., the following holds"). However, the fact that an object exists does not necessarily mean that it's easy to find. Similarly, a proof branching on a condition cannot be translated to an algorithm if that condition cannot be determined efficiently. $\exists a, b, c, n > 2: a^n + b^n = c^n$ can only be either "true" or "false", but a program cannot determine that.

So someone had to introduce a new framework: a model of a real-world computer that could be studied in isolation. One of such models was the [Turing machine](https://en.wikipedia.org/wiki/Turing_machine), which you've likely already heard about. But it wasn't the only one. Another candidate, distinct but equal in computational power, was [lambda calculus](https://en.wikipedia.org/wiki/Lambda_calculus).

If Turing machine is an esoteric imperative language, $\lambda$-calculus is a functional one. Just like the Turing machine, it's not supposed to be used directly, but rather acts as a substitute for complex real-world languages during research. And research it we did: thank $\lambda$-calculus for advances in language design, type inference, and formal verification.

So why do we care about $\lambda$-calculus while studying a lint for an imperative language? A core property of functional languages is that they enable local reasoning. While a mutating operation in an imperative language can potentially affect state at any later point, expressions in functional languages are pure, depend exclusively on their inputs, and don't "leak" anything but their return value. If you're starting to figure out how this locality helps, good; otherwise, I'll answer this explicitly in a bit.


### $\lambda$-calculus

So what *is* $\lambda$-calculus? It's a language that allows you to create anonymous functions and invoke functions, and do nothing else. To create functions, you use what other languages often call lambda functions. Python says `lambda x: ...`, C++ says `[](auto x) { ... }`, and $\lambda$-calculus says $\lambda x. \dots$. To invoke functions, you write the name of the function followed by its arguments: $f \, a$. Obviously, arguments are functions too.

It may seem surprising that this mechanism is powerful enough to implement *anything*, given that the only type we have is "function", and the only way to create named objects is to capture them as function arguments. That's right -- you can't even define globals here!

I'm not going to teach you $\lambda$-calculus today, but I'll show two examples so that you aren't completely blindsided. Here's how booleans are typically defined:

$$
\begin{align*}
\textrm{true} := \lambda x. (\lambda y. x) \\
\textrm{false} := \lambda x. (\lambda y. y)
\end{align*}
$$

Note that $\lambda$-calculus only supports single-argument functions: to simulate more arguments, you [define internal functions](https://en.wikipedia.org/wiki/Currying) capturing the outside context. Now, if you have a boolean $b$ and you want to get the number $5$ if it's `true` and $8$ if it's `false`, you can write $(b \, 5) \, 8$. For example, if $b$ is equal to `true`, this will evaluate to:

$$
((\lambda x. (\lambda y. x)) \, 5) \, 8 = (\lambda y. 5) \, 8 = 5.
$$

In effect, programs in $\lambda$-calculus encode data into the control flow of functions: it's common for a function to take both "real" arguments and "callbacks", which are either selected or combined depending on what the function wants to return. You can use this trick to implement pairs as functions that pass their contents as two arguments to the callback:

$$
(x, y) := \lambda f. ((f \, x) y).
$$

With makeshift ternaries, you can implement boolean logic (e.g. `a && b` is `a ? b : false`), and with pairs, you can implement linked lists. That gives you numbers, arrays, and all finite computation. Loops and recursion are also [possible to implement](https://en.wikipedia.org/wiki/Fixed-point_combinator), but ultimately offtopic for this article, so I'll wrap this up for now.


### Representations

Only being able to "declare" a variable by defining a function is restrictive, but on the flip side, it simplfies code analysis.

Since a variable can only be accessed within the $\lambda$-function that declares it, each variable access can be identified with the nesting level of the corresponding definition. For example, in $\lambda x. (\lambda y. y)$, the access $y$ corresponds to the most nested definition visible at the time of access, so we'll write that as $\underline1$. For $\lambda x. (\lambda y. x)$, it would be $\underline2$, and so on. These numbers are called [de Bruijn indices](https://en.wikipedia.org/wiki/De_Bruijn_index), and they're sufficient to resolve variables without ever mentioning their names.

Any $\lambda$-calculus program can be rewritten to this format. For example:

$$
\begin{gather*}
\lambda f. (\lambda x. f (x \, x)) (\lambda x. f (x \, x)) \\
\Downarrow \\
\lambda. (\lambda. \underline2 (\underline1 \, \underline1)) (\lambda. \underline2 (\underline1 \, \underline1))
\end{gather*}
$$

The variable names can be removed from the $\lambda$-function definition, since they are now irrelevant. And that's the superpower: the resulting expression is independent of variable names, but represents the same program, just in a different encoding. This means that such programs can be compared for equality straightforwardly without accounting for differences in names of locals. Yay!

Note that, with this scheme, the same variable can be seen with different indices from different scopes: $\lambda f. f (\lambda x. f)$ is converted to $\lambda. \underline1 (\lambda. \underline2)$. Just keep that in mind for now and don't let it trick you.

Of course, not all variables are local, i.e. defined within the snippet. For example, we can only rewrite $\lambda f. (f \, a) \, b$ to $\lambda. (\underline1 \, a) \, b$, leaving the names of the free (i.e. non-local) variables $a$ and $b$ intact. This mixed representation is called *locally nameless* for obvious reasons.


### Lint

Back to the original problem: writing a lint for repeating `match` arms. We don't actually need to convert the imperative code to $\lambda$-calculus, since it's just a framework: we can extract the core ideas and attach them to the imperative IR. All we need is for each variable to have an associated definition scope, where each scope defines at most one variable. For example,

```rust
// `a` is defined outside
let x = 2 + 2;
let y = "hello";
if x == a {
    let z = "world";
    println!("{y} {z}");
}
```

...could be interpreted as:

```rust
let x {
    let y {
        x = 2 + 2;
        y = "hello";
        if x == a {
            let z {
                z = "world";
                println!("{y} {z}");
            }
        }
    }
}
```

...and then converted to:

```rust
let {
    let {
        _2 = 2 + 2;
        _1 = "hello";
        if _2 == a {
            let {
                _1 = "world";
                println!("{_2} {_1}");
            }
        }
    }
}
```

...which can be directly inserted into a hash map.


### Simpler path?

Oh, but couldn't we do something simpler earlier? Couldn't we scan each `match` arm for a list of variable definitions and rewrite local variable accesses into indices from that list, resulting in something like:

```rust
let _1 = 2 + 2;
let _2 = "hello";
if _1 == a {
    let _3 = "world";
    println!("{_2} {_3}");
}
```

Yes, yes we could. But you see, I too have a superpower, called "lying".

We aren't *just* trying to find repetitions among the arms of a single `match`. We're doing that over the whole program. Maybe we're writing more lints. Maybe we're trying to compress code. Maybe we're a theorem prover trying to reduce the size of the graph. Maybe we're [writing a Java decompiler](../jvm-exceptions-are-weird-a-decompiler-perspective/) ~~(ahem)~~.

Either way, we're doing this *at scale*. And that means that we can't just insert the full representations of all subtrees into a single hash map: we're going to run out of memory really fast. Barring that, this process is going to take an embarrassing amount of time, since we'll be constantly iterating over deeply nested nodes.

And that's where the gamble pays out. What we want is some clever way to compute the nameless encoding of a large block from the encodings of its constituents, so that we don't have to scan subtrees repeatedly. Preferably, we want simple concatenation to work:

$$
\mathrm{repr}(f \, a) = \mathrm{repr}(f) \, \mathrm{repr}(a).
$$

With de Bruijn indices, that's exactly the case. For example, for $f = \lambda x. x \, x$ and $a = \lambda y. y$, the representations are:

$$
\begin{align*}
\mathrm{repr}(f) &= \lambda. \underline1 \, \underline1 \\
\mathrm{repr}(a) &= \lambda. \underline1 \\
\mathrm{repr}(f \, a) &= (\lambda. \underline1 \, \underline1) (\lambda. \underline1)
\end{align*}
$$

Meanwhile, had we tried to use indices into a list of local variables, that wouldn't work nearly as well. We'd get the same representations for $f$ and $a$, but for $f \, a$, we'd expect to get $(\lambda. \underline1 \, \underline1) (\lambda. \underline2)$, which doesn't match the concatenation.

1-0 for learning math.

The only trouble is that function calls are only one part of the story -- there's also function declarations. It sure seems like

$$
\mathrm{repr}(\lambda x. t) = \lambda. \mathrm{repr}(t)
$$

...should work too, but that's not quite true. Consider $\lambda x. x$. The locally nameless representation of $x$ is just $x$, since it's a non-local variable, thus accessed by name. But when we handle $\lambda x. x$, it becomes local, so the name should be replaced with an index, resulting in $\lambda. \underline1$. So a more honest algorithm looks like:

$$
\mathrm{repr}(\lambda x. t) = \lambda. \mathrm{repr}(t)[x := \underline1],
$$

where $[x := \underline1]$ denotes replacement, and the index in $\underline1$ is implicitly understood to be adjusted to the correct one depending on the nesting level.

Note that you don't have to iterate over the subexpression to find the locations to change. Before analyzing the program, you can accumulate the positions at which each variable is mentioned, and then only mutate those locations with random access. So it's going to take linear time in total.

This is a lot to take in, so let me put it in code so that we're on the same page:

```python
range_of_expr: dict[Expr, tuple[int, int]] = {}
variable_nesting: dict[VariableName, int] = {}
variable_accesses: dict[VariableName, list[tuple[Expr, int]]] = {}
current_location: int = 0

def collect_locations(expr: Expr, nesting: int):
    global current_location
    start = current_location
    match expr:
        case Variable(x):
            # x
            current_location += 1
            variable_accesses[x].append((start, nesting - variable_nesting[x]))
        case Function(x, body):
            # \, body
            current_location += 1
            variable_nesting[x] = nesting
            variable_accesses[x] = []
            collect_locations(body, nesting + 1)
        case Call(f, a):
            # (, f, a, )
            current_location += 1
            collect_locations(f, nesting)
            collect_locations(a, nesting)
            current_location += 1
    end = current_location
    range_of_expr[expr] = (start, end)

collect_locations(root, 0)
output = [None] * current_location

def calculate_representations(expr: Expr):
    start, end = range_of_expr[expr]
    match expr:
        case Variable(x):
            output[start] = x
        case Function(x, body):
            output[start] = "\\"
            calculate_representations(body)
            for location, de_bruijn_index in variable_accesses[x]:
                output[location] = de_bruijn_index
        case Call(f, a):
            output[start] = "("
            calculate_representations(f)
            calculate_representations(a)
            output[end - 1] = ")"
    print("The representation of", expr, "is", output[start:end])

calculate_representations(root)
```


### Hashing

Ironically, the only non-linear part of the algorithm is `output[start:end]` in the `print` statement, since the total length of representations is $\mathcal{O}(n^2)$. Yes, we're getting bottlenecked by output. Now that we're producing representations quickly, we need to figure out how to consume them at the same speed. But fixing this is easier said than done: we can't insert the representations into a hash map or even properly hash them, since we simply don't have the time.

However, we can still hash the representations *improperly*. Here's what I mean by that. Suppose that we used the following -- very stupid, I know -- hash function:

:::aside
$\oplus$ denotes XOR in math formulas. $||$ denotes concatenation.
:::

$$
\mathrm{hash}(c_0 c_1 \dots c_{n-1}) = c_0 \oplus c_1 \oplus \dots \oplus c_{n-1}
$$

<aside-inline-here />

This formula makes it easy to compute the hash of concatenation of two hashed strings, without knowing their exact contents:

$$
\mathrm{hash}(s_1 \mathop{||} s_2) = \mathrm{hash}(s_1) \oplus \mathrm{hash}(s_2)
$$

But more importantly, it can handle replacements:

$$
\mathrm{hash}(s_1 \mathop{||} c' \mathop{||} s_2) = \mathrm{hash}(s_1 \mathop{||} c \mathop{||} s_2) \oplus c \oplus c'
$$

Together, this allows us to efficiently compute hashes of all representations, without even storing those representations in memory. That's great, except for the fact that XOR makes for a terrible hash.

:::aside
Polynomial hashing provides strong guarantees. If you treat a string $s$ as a polynomial $s(x)$ with coefficients matching individual characters, you'll find that its hash is the value $s(b)$. The hashes of two strings $s_1$, $s_2$ collide if $s_1(b) - s_2(b) = 0$, i.e. if $b$ is the root of the polynomial $s_1(x) - s_2(x)$. For strings of length $k$, this polynomial has degree $k - 1$ and thus at most $k - 1$ roots, so the collision rate is $< \frac{k}{p}$.
:::

But there's better hashes that still support these operations. The most common one is the [polynomial hash](https://en.wikipedia.org/wiki/Rolling_hash#Polynomial_rolling_hash), defined as follows:

$$
\mathrm{hash}(c_0 c_1 \dots c_{n-1}) = c_0 + c_1 b + c_2 b^2 + \dots + c_{n-1} b^{n-1} \mod p
$$

Here, $p$ and $b$ are parameters of the hash. $p$ is a large prime number, typically chosen by the programmer and fixed during execution. $b$ is an arbitrary number from $0$ to $p - 1$, typically chosen randomly at runtime. The polynomial hash is safer than XOR because characters don't trivially cancel out.

<aside-inline-here />

To concatenate strings, we only need to know their hashes and lengths:

$$
\mathrm{hash}(s_1 \mathop{||} s_2) = \mathrm{hash}(s_1) + \mathrm{hash}(s_2) \, b^{|s_1|} \mod p
$$

To replace a character, we need to know the original hash, the position of the affected character, and what is replaced with what:

$$
\mathrm{hash}(s_1 \mathop{||} c' \mathop{||} s_2) = \mathrm{hash}(s_1 \mathop{||} c \mathop{||} s_2) + (c' - c) \, b^i \mod p
$$

All in all, this allows the hashes of all subexpressions to be computed in $\mathcal{O}(n)$:

```python
powers_of_b: list[int] = [1]

# Computes `h * b ** count % p` in amortized constant time.
def shift(h: int, count: int) -> int:
    while len(powers_of_b) <= count:
        powers_of_b.append(powers_of_b[-1] * b % p)
    return h * powers_of_b[count] % p

# Not shown: a function capable of hashing variable names, de Bruijn indices, and characters
# \, (, ) without collisions.
def hash_small(x) -> int: ...

def calculate_hashes(expr: Expr) -> int:
    start, end = range_of_expr[expr]
    match expr:
        case Variable(x):
            h = hash_small(x)
        case Function(x, body):
            h = hash_small("\\") + shift(calculate_hashes(body), 1)
            for location, de_bruijn_index in variable_accesses[x]:
                h += shift(hash_small(de_bruijn_index) - hash_small(x), location - start)
                h %= p
        case Call(f, a):
            h = (
                hash_small("(")
                + shift(calculate_hashes(f), 1)
                + shift(calculate_hashes(a), range_of_expr[a][0] - start)
                + shift(hash_small(")"), end - 1 - start)
            )
    h %= p
    print("The hash of", expr, "is", h)
    return h

calculate_hashes(root)
```


### Summing up

We've figured out how to compare any subtrees for equality in constant time, at the cost of some linear-time precalculation. So that's good.

Unfortunately, we had to use hashes, which means that there's a small chance of a false positive: we might consider two distinct trees identical if their hashes match by accident. What can we do about it?

1. Sometimes, such a false positive is perfectly acceptable. If the chosen prime $p$ is large enough and $b$ is truly random, there's a very low chance of getting the wrong answer -- lower than [a bit flip due to a stray particle](https://en.wikipedia.org/wiki/Single-event_upset).

2. We can avoid false positives by comparing the subexpressions for equality if the hashes match. This gives us the ability to discard checks that we know would fail, and only focus on successes. Depending on your goal, this may either be an asymptotic slow-down back to $\mathcal{O}(n^2)$ in the worst case, or not. For example, if you delete one of the duplicates as soon as you encounter it, it's going to be $\mathcal{O}(n)$; otherwise you might be in danger.

3. If you're drawn to constant-time comparison, there is a way to *validate* the obtained hashes for lack of collisions in $\mathcal{O}(n)$. If a collision is found, just regenerate the hashes with a different seed. This has expected linear time, but is slower than blind trust.

4. If you don't like randomness, there's a deterministic algorithm producing collision-free hashes in guaranteed $\mathcal{O}(n \log n)$ time.

The algorithms in 3. and 4. are tricky, and this post is already quite long, so I'll cover them in a later, more technical post. [Subscribe to my RSS](../feed.rss) if you don't want to miss it.


### References

This post is partially based on the paper [Hashing Modulo Alpha-Equivalence](https://arxiv.org/abs/2105.02856), though the algorithm described in the paper is slightly different:

- The time complexity in the paper is listed as $\mathcal{O}(n \log^2 n)$ due to the use of a balanced tree instead of a hash table. With a hash table, the expected time becomes $\mathcal{O}(n \log n)$.
- The paper's algorithm can be computed incrementally over subtrees instead of having to see the whole program at once, at the cost of the $\log n$ factor.
- The paper uses a different hashing scheme. This post uses single-layer polynomial hashing, while the paper uses XOR, but passes the arguments through a pseudo-random hash function. This leads to slightly different collision rates and practical performance.
