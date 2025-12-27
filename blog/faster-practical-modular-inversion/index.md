---
title: Faster practical modular inversion
time: December 20, 2025
discussion:
  - https://news.ycombinator.com/item?id=46341904
intro: |
    Last year, [Lemire wrote](https://lemire.me/blog/2024/04/13/greatest-common-divisor-the-extended-euclidean-algorithm-and-speed/) about an optimized variation of [the Euclidean algorithm](https://en.wikipedia.org/wiki/Euclidean_algorithm) for computing [the greatest common divisor](https://en.wikipedia.org/wiki/Greatest_common_divisor) of two numbers, called *binary Euclidean algorithm* or *Stein's algorithm*. It's a best-of-class implementation, though it's currently only used by libc++.

    The post also briefly mentions [the extended Euclidean algorithm](https://en.wikipedia.org/wiki/Extended_Euclidean_algorithm), a related algorithm most often used to compute the [modular multiplicative inverse](https://en.wikipedia.org/wiki/Modular_multiplicative_inverse) (given a remainder $a$ and a modulus $m$, find $x$ such that $a \cdot x \bmod m = 1$):

    > There is also a binary version of the extended Euclidean algorithm[,] although it is quite a bit more involved and it is not clear that it [...] can be implemented at high speed, leveraging fast instructions, when working on integers that fit in general-purpose registers. [...]
    > 
    > My implementation of the binary extended Euclidean algorithm is quite a bit slower and not recommended. I expect that it should be possible to optimize it further.

    That's a big shame, because the extended Euclidean algorithm can be optimized in a very similar manner, and the underlying ideas were described [in a 2020 paper](https://eprint.iacr.org/2020/972.pdf). It's probably not well-known because the paper focuses on constant-time evaluation and long arithmetic, so people might have assumed it's irrelevant.

    I'm hoping to bring justice to the extended Stein's algorithm with this post. I'll cover how the algorithm works, its limitations, some optimizations compared to Pornin's paper, and potential further improvements.

    My implementation is [available on GitHub](https://github.com/purplesyringa/mod2k/blob/104603af3866ac274073a5b2af28f7a41550add1/src/xgcd.rs) as part of a Rust modular arithmetic library.
---

:::aside
If you're looking to contribute to OSS, that's your cue.
:::

Last year, [Lemire wrote](https://lemire.me/blog/2024/04/13/greatest-common-divisor-the-extended-euclidean-algorithm-and-speed/) about an optimized variation of [the Euclidean algorithm](https://en.wikipedia.org/wiki/Euclidean_algorithm) for computing [the greatest common divisor](https://en.wikipedia.org/wiki/Greatest_common_divisor) of two numbers, called *binary Euclidean algorithm* or *Stein's algorithm*. It's a best-of-class implementation, though it's currently only used by libc++.

<aside-inline-here />

The post also briefly mentions [the extended Euclidean algorithm](https://en.wikipedia.org/wiki/Extended_Euclidean_algorithm), a related algorithm most often used to compute the [modular multiplicative inverse](https://en.wikipedia.org/wiki/Modular_multiplicative_inverse) (given a remainder $a$ and a modulus $m$, find $x$ such that $a \cdot x \bmod m = 1$):

> There is also a binary version of the extended Euclidean algorithm[,] although it is quite a bit more involved and it is not clear that it [...] can be implemented at high speed, leveraging fast instructions, when working on integers that fit in general-purpose registers. [...]
>
> My implementation of the binary extended Euclidean algorithm is quite a bit slower and not recommended. I expect that it should be possible to optimize it further.
>
> *-- Lemire*

That's a big shame, because the extended Euclidean algorithm can be optimized in a very similar manner, and the underlying ideas were described [in a 2020 paper](https://eprint.iacr.org/2020/972.pdf). It's probably not well-known because the paper focuses on constant-time evaluation and long arithmetic, so people might have assumed it's irrelevant.

I'm hoping to bring justice to the extended Stein's algorithm with this post. I'll cover how the algorithm works, its limitations, some optimizations compared to Pornin's paper, and potential further improvements.

My implementation is [available on GitHub](https://github.com/purplesyringa/mod2k/blob/104603af3866ac274073a5b2af28f7a41550add1/src/xgcd.rs) as part of a Rust modular arithmetic library.


### Disclaimer

The textbook algorithm can be used not only to compute inverses, but also to solve [linear Diophantine equations](https://en.wikipedia.org/wiki/Diophantine_equation). I will focus on the former in this post, since that's where the optimizations shine at. I'll briefly cover the general case at the end of the post.

I won't make claims on exact performance, because something strange is going on with the Lemire's benchmarking results and I don't want to add to the mess. I've measured that my implementation of the algorithm is $1.3$ -- $2$ times faster than the textbook implementation on average, even on M4, but you may see a completely different picture if your compiler produces slightly different codegen.

> Lemire's benchmark seems to be skewed by the choice of the compiler (GCC vs Clang), its version (Clang 18 vs Clang 21), optimization flags (`-O2` vs `-O3`), the microarchitecture (Haswell vs Ice Lake vs Zen 2), and minutiae of the benchmarking code. Results don't make much sense mathematically and look disproportionately affected by microarchitectural conditions.
>
> If you want to get the fastest implementation, I suggest you inspect the assembly more closely than me, because I have no idea what's going on.

Nevertheless, here is some raw data for transparency. The benchmark measures the time per inversion (in ns), the cell format is "Stein's algorithm / Euclidean algorithm".

<style>
    td {
        font-size: 0.9rem;
    }
    @media screen and (min-width: 1001px) {
        td > span {
            display: inline-block;
            min-width: 42px;
        }
        td > .left {
            text-align: right;
        }
        td:nth-child(5) {
            & > span {
                min-width: 50px;
            }
        }
    }
    .positive {
    }
    .negative {
        color: #d92a2a;
    }
</style>

<table>
    <thead>
        <tr>
            <td></td>
            <th>8 bits</th>
            <th>16 bits</th>
            <th>32 bits</th>
            <th>64 bits</th>
        </tr>
    </thead>
    <tbody>
        <tr>
            <th>Haswell</th>
            <td class="positive"><span class="left">11.38</span> / <span class="right">19.21</span> (-41%)</td>
            <td class="positive"><span class="left">17.48</span> / <span class="right">33.96</span> (-49%)</td>
            <td class="positive"><span class="left">29.76</span> / <span class="right">61.69</span> (-52%)</td>
            <td class="positive"><span class="left">67.18</span> / <span class="right">152.19</span> (-56%)</td>
        </tr>
        <tr>
            <th>Alder Lake</th>
            <td class="positive"><span class="left">8.20</span> / <span class="right">10.19</span> (-20%)</td>
            <td class="positive"><span class="left">13.77</span> / <span class="right">16.87</span> (-18%)</td>
            <td class="positive"><span class="left">21.47</span> / <span class="right">31.00</span> (-31%)</td>
            <td class="positive"><span class="left">50.38</span> / <span class="right">69.57</span> (-28%)</td>
        </tr>
        <tr>
            <th>Zen 5</th>
            <td class="positive"><span class="left">7.77</span> / <span class="right">10.56</span> (-26%)</td>
            <td class="positive"><span class="left">9.43</span> / <span class="right">14.80</span> (-36%)</td>
            <td class="positive"><span class="left">13.96</span> / <span class="right">23.98</span> (-42%)</td>
            <td class="positive"><span class="left">34.58</span> / <span class="right">49.24</span> (-30%)</td>
        </tr>
        <tr>
            <th>M1</th>
            <td class="negative"><span class="left">14.58</span> / <span class="right">13.05</span> (+12%)</td>
            <td class="positive"><span class="left">11.48</span> / <span class="right">18.63</span> (-38%)</td>
            <td class="positive"><span class="left">19.74</span> / <span class="right">35.47</span> (-44%)</td>
            <td class="positive"><span class="left">43.14</span> / <span class="right">71.14</span> (-39%)</td>
        </tr>
        <tr>
            <th>M2</th>
            <td class="positive"><span class="left">8.93</span> / <span class="right">10.26</span> (-13%)</td>
            <td class="positive"><span class="left">11.00</span> / <span class="right">17.90</span> (-39%)</td>
            <td class="positive"><span class="left">19.38</span> / <span class="right">33.78</span> (-43%)</td>
            <td class="positive"><span class="left">41.33</span> / <span class="right">68.03</span> (-39%)</td>
        </tr>
        <tr>
            <th>M4</th>
            <td class="positive"><span class="left">5.28</span> / <span class="right">8.60</span> (-39%)</td>
            <td class="positive"><span class="left">8.07</span> / <span class="right">14.77</span> (-45%)</td>
            <td class="positive"><span class="left">13.63</span> / <span class="right">28.05</span> (-51%)</td>
            <td class="positive"><span class="left">28.68</span> / <span class="right">56.22</span> (-49%)</td>
        </tr>
        <tr>
            <th>Cortex-A72</th>
            <td class="positive"><span class="left">29.80</span> / <span class="right">33.48</span> (-11%)</td>
            <td class="positive"><span class="left">38.30</span> / <span class="right">49.36</span> (-22%)</td>
            <td class="positive"><span class="left">61.28</span> / <span class="right">83.63</span> (-27%)</td>
            <td class="negative"><span class="left">162.55</span> / <span class="right">151.77</span> (+7%)</td>
        </tr>
        <tr>
            <th>Snapdragon 8 Gen 3</th>
            <td class="positive"><span class="left">9.72</span> / <span class="right">12.13</span> (-20%)</td>
            <td class="positive"><span class="left">14.97</span> / <span class="right">21.91</span> (-32%)</td>
            <td class="positive"><span class="left">28.51</span> / <span class="right">39.89</span> (-29%)</td>
            <td class="positive"><span class="left">70.11</span> / <span class="right">75.46</span> (-7%)</td>
        </tr>
        <tr>
            <th>Kryo 485</th>
            <td class="positive"><span class="left">15.08</span> / <span class="right">19.36</span> (-22%)</td>
            <td class="positive"><span class="left">21.54</span> / <span class="right">30.41</span> (-29%)</td>
            <td class="positive"><span class="left">33.63</span> / <span class="right">50.96</span> (-34%)</td>
            <td class="positive"><span class="left">90.32</span> / <span class="right">94.76</span> (-5%)</td>
        </tr>
    </tbody>
</table>


### GCD

Let's start with the algorithm for computing the GCD of $a$ and $b$. Suppose for now that $b$ is odd. Here's the core idea:

- If $a$ is divisible by $2^k$, this factor can be removed: $\mathrm{gcd}(2^k a, b) = \mathrm{gcd}(a, b)$. This decreases the bit length of $a$ by at least $1$, guaranteeing $\mathcal{O}(\log a)$ time complexity if we can apply this reduction consistently.
- If both $a$ and $b$ are odd, rewriting $\mathrm{gcd}(a, b) = \mathrm{gcd}(a - b, b)$ guarantees $a' = a - b$ will be even and reducible on the next iteration. To avoid negative integers, swap $a$ and $b$ if $a < b$ beforehand; new $b$ remains odd because $a$ was odd.

The implementation is very short:

```rust
while a != 0 {
    a >>= a.trailing_zeros();
    if a < b {
        (a, b) = (b, a);
    }
    a -= b;
}
return b;
```

If the initial $b$ is not guaranteed to be odd, some adjustments are necessary:

```rust
let shift = (a | b).trailing_zeros(); // == min(a.trailing_zeros(), b.trailing_zeros())
b >>= b.trailing_zeros();
/* loop from the previous snippet */
return b << shift;
```

But for modular inversion, the modulus is usually odd, so I won't dwell on this.



### Optimizations

This covers the general structure of the algorithm, but some optimizations are crucial for getting good performance.

The conditional swap should be compiled to branchless code to avoid branch misprediction. Compiler hints like [\__builtin_unpredictable](https://clang.llvm.org/docs/LanguageExtensions.html#builtin-unpredictable) or [core::hint::select_unpredictable](https://doc.rust-lang.org/stable/core/hint/fn.select_unpredictable.html) may be useful.

The loop has a high latency because `trailing_zeros`, `>>=`, `if`, and `-=` are computed sequentially. But since `(-a).trailing_zeros() == a.trailing_zeros()`, `a.trailing_zeros()` can in principle be computed before the swap on the previous iteration:

```rust
let mut q = a.trailing_zeros();
while a != 0 {
    a >>= q;
    q = (a - b).trailing_zeros();
    if a < b {
        (a, b) = (b - a, a);
    } else {
        (a, b) = (a - b, b);
    }
}
```

This brings the latency down to 3 operations: `>>=`; `a - b` and `b - a` computed in parallel; `trailing_zeros` and `if` computed in parallel. It also slightly increases the number of operations (computing `b - a` and `a - b` and only using one), but the tradeoff pays off.

Pay close attention to `trailing_zeros` if you're implementing this in C. The algorithm can invoke it with a zero input on the last iteration. This is well-defined in Rust, which maps $0$ to the bit width of the data type, but in C `__builtin_clz(0)` is UB. Use `__builtin_clzg` to avoid issues. In C++, `std::countr_zero(0)` is well-defined.

> GCC [documents](https://gcc.gnu.org/onlinedocs/gcc/Bit-Operation-Builtins.html) `__builtin_clz(0)` as having an "undefined result", so I initially assumed it means an indeterminate value. In reality, [GCC maintainers consider it UB](https://gcc.gnu.org/bugzilla/show_bug.cgi?id=116989) and [LLVM documents it as UB](https://clang.llvm.org/docs/LanguageExtensions.html#builtin-clzg-and-builtin-ctzg)... but the optimizers seem to model it exactly like an indeterminate value? (e.g. LLVM considers `@llvm.cttz(0)` to produce `poison`) This is frankly ridiculous, someone do something about it.


### Extending

:::aside
Here's a linear algebraic restatement if you want a different perspective.

Weighted sums are actually [linear combinations](https://en.wikipedia.org/wiki/Linear_combination), and the coefficients can be tracked because subtraction and division by a constant are both [linear operators](https://en.wikipedia.org/wiki/Linear_map).

The algorithm tracks a matrix $A$ of a specific form, mapping $(a_0 \, b_0)^T$ to $(a \, b)^T$.

Treating values as opaque, $a$ and $b$ are elements of a [vector space](https://en.wikipedia.org/wiki/Vector_space) with [basis](https://en.wikipedia.org/wiki/Basis_(linear_algebra)) $\langle a_0, b_0 \rangle$, $A$ is a [change-of-basis matrix](https://en.wikipedia.org/wiki/Change_of_basis), and the integers $a$ and $b$ in code are values of some [evaluation functional](https://en.wikipedia.org/wiki/Linear_form) at $a$ and $b$.

Simplifying $\langle a_0, b_0 \rangle$ to $\langle 0, \mathrm{gcd}(a_0, b_0) \rangle$ is essentially [lattice reduction](https://en.wikipedia.org/wiki/Lattice_reduction), which can be seen as a generalization of GCD to higher-dimensional spaces.
:::

You might be wondering how this algorithm is related to modular inversion.

The trick is to express the values of $a$ and $b$ at each point as weighted sums of the original $a$ and $b$ (denoted $a_0, b_0$) with some coefficients $k_i, l_i$:

$$
\begin{cases}
a = k_0 a_0 + l_0 b_0 \\
b = k_1 a_0 + l_1 b_0
\end{cases}
$$

If $a_0$ is invertible modulo $b_0$, their GCD is $1$, and so at the end of the algorithm $b = 1$. This gives us:

$$
k_1 a_0 + l_1 b_0 = 1 \implies k_1 a_0 = 1 \pmod{b_0}
$$

That is, $k_1$ is the inverse of $a_0$ modulo $b_0$. So all we need to do is track the coefficients across iterations. We start with:

$$
\begin{cases}
a = a_0 = 1 \, a_0 + 0 \, b_0 \\
b = b_0 = 0 \, a_0 + 1 \, b_0
\end{cases}
$$

When $a$ is divided by $2^q$, the coefficients are divided by the same value:

$$
a = k_0 a_0 + l_0 b_0 \implies \frac{a}{2^q} = \frac{k_0}{2^q} a_0 + \frac{l_0}{2^q} b_0
$$

When $a$ and $b$ are swapped, the pairs $(k_0, l_0)$ and $(k_1, l_1)$ are swapped.

When $b$ is subtracted from $a$, the coefficients are subtracted:

$$
a - b = (k_0 - k_1) a_0 + (l_0 - l_1) b_0
$$

In other words, whatever we do to $a$ and $b$, we also do to the coefficient pairs $(k_0, l_0)$.

<aside-inline-here />


### Limitations

Implementation attempts quickly reveal a problem: coefficients are not necessarily divisible by $2^q$, so it's not clear how to represent them. Surely not with floats.

This is actually a core difference between Stein's algorithm and the textbook Euclidean algorithm, which is implemented as $\mathrm{gcd}(a, b) = \mathrm{gcd}(b, a \bmod b)$.

The Euclidean algorithm uses division (`q = a / b`), but only to compute constant factors. The values are updated with subtraction and multiplication alone (`a -= b * q`). Stein's algorithm divides values (`a /= 2^q`), causing non-integer coefficients.

This is likely why the extended Stein's algorithm is unpopular. We'll use tricks tailored to modular inverse, but the general-purpose case covered at the end of the post essentially boils down to "compute modular inverse and post-process". I believe it can still be faster than the textbook implementation, but I haven't tested it.


### Fractions

We can track coefficients as fractions to stay in integers. The most efficient approach uses the same denominator $2^p$ for all variables:

$$
\begin{cases}
a = 2^{-p} \, (k_0 a_0 + l_0 b_0) \\
b = 2^{-p} \, (k_1 a_0 + l_1 b_0)
\end{cases}
$$

We start with $p = 0$. Instead of dividing $k_0, l_0$ by $2^q$, we increase $p$ by $q$ and multiply $k_1, l_1$ by $2^q$. Subtraction can ignore $p$ because all coefficients use the same precision.

This seems pointless at first, since we need to know $2^{-p} \bmod b_0$, but if the modulus is fixed, we can precompute it. Each iteration reduces the total bit length of $a$ and $b$ by at least $q$, and after the last right-shift $a, b \ne 0$, so if the input numbers fit in $k$ bits, the sum of $q$ (and thus $p$) is limited by $2k - 2$. This means that we can increase precision to $2k - 2$ at the end and use a single precomputed value $2^{-(2k - 2)} \bmod b_0$.

The multitude of variables is getting confusing, so let's simplify it. We're looking for $k_1 \bmod b_0$ and don't care about $l_i$, so tracking just $k_0$ and $k_1$ suffices. Let's rename these variables to $u$ and $v$ respectively to get rid of indices. This gives us:

```rust
// Example for 32-bit inputs (k = 32).
let mut u = 1;
let mut v = 0;
let mut p = 0;
let mut q = a.trailing_zeros();
while a != 0 {
    a >>= q;
    v <<= q;
    p += q;

    q = (a - b).trailing_zeros();

    if a < b {
        (a, b) = (b - a, a);
        (u, v) = (v, u);
    } else {
        (a, b) = (a - b, b);
    }
    u -= v;
}
assert!(b == 1, "not invertible");
v <<= 62 - p;
return (v * inverse_of_2p62) % b0;
```

We don't apply the latency-reducing trick to $u$ and $v$ because the latency is dominated by other calculations. Computing both `u - v` and `v - u` would most likely reduce performance, since we're already pushing the CPU limit of parallel operations.


### Types

:::aside
Note that $u$ is one bit longer than $v$ at this point because $u$ is increased at the end of the previous iteration, while $v$ is increased at the beginning of the next iteration.
:::

It's easy to prove by induction that at the beginning of each iteration,

$$
\begin{cases}
-2^{p+1} < u < 2^{p+1} \\
-2^p < v \le 2^p
\end{cases}
$$

<aside-inline-here />

This means that $u$ and $v$ fit in signed $p + 2$-bit integers. Since $p \le 2k - 2$, that amounts to $2k$-bit types, i.e. twice as wide as the input. And that's a problem: while it works just fine for $32$-bit inputs, $64$-bit inputs require `i128` arithmetic, which slows down the algorithm considerably. We'll discuss what to do about it in a bit.


### Montgomery

Before we do this, though, let's finish the $32$-bit case. There's just one thing left to improve: computing $v \cdot 2^{-62} \bmod b_0$.

On the face of it, this is one multiplication and one reduction, but [Montgomery multiplication](https://en.wikipedia.org/wiki/Montgomery_modular_multiplication) demonstrates that these operations can be performed faster together.

Assume for a moment that $v$ is non-negative. The idea is to subtract a multiple of $b_0$ from $v$ such that the bottom $62$ bits become zero, so that the remainder remains the same, but division by $2^{62}$ can be performed with a shift. We're looking for $t$ such that

$$
v - t \cdot b_0 = 0 \pmod{2^{62}}
$$

This is equivalent to $t = v \cdot b_0^{-1} \pmod{2^{62}}$, and by precomputing $j = b_0^{-1} \bmod 2^{62}$, we obtain $v - (v j \bmod 2^{62}) b_0$ as the easily divisible value. Since $v$ and $(v j \bmod 2^{62}) b_0$ have equal bottom $62$ bits,

$$
\frac{v - (v j \bmod 2^{62}) b_0}{2^{62}} = \left\lfloor \frac{v}{2^{62}} \right\rfloor - \left\lfloor \frac{(v j \bmod 2^{62}) b_0}{2^{62}} \right\rfloor
$$

We've just found that $v \le 2^p \le 2^{62}$, so unless $v = 2^{62}$ exactly, this is just

$$
-\left\lfloor \frac{(v j \bmod 2^{62}) b_0}{2^{62}} \right\rfloor = -\left\lfloor \frac{(v (4 j) \bmod 2^{64}) b_0}{2^{64}} \right\rfloor
$$

This number is in range $[-b_0 + 1; 0]$. We know that $0$ can never be an inverse, so it's actually $[-b_0 + 1; -1]$, and by adding $b_0$, we obtain the exact remainder. This can be computed with only two multiplications and some glue:

```rust
fn redc62(v: i64) -> u32 {
    if v == (1 << 62) {
        1
    } else {
        let x = v.unsigned_abs().wrapping_mul(j << 2).widening_mul(b0 as u64).1 as u32;
        if v > 0 { b0 - x } else { x }
    }
}
```

That's it for $32$-bit and smaller inputs. Yay! Buy yourself a cupcake.


### 64-bit inputs

For $64$-bit inputs, coefficients only fit in `i128`. This makes each operation twice as slow. We can reduce $u$ and $v$ modulo $b_0$ on each iteration so that coefficients fit in $64$ bits, since we only need $v \bmod b_0$, but this tanks performance too.

Hmm. Notice that at the beginning of the algorithm, $u$ and $v$ fit in $1$ bit and then grow slowly. Only once their length exceeds $64$ bits do we need long integers. What if we could somehow reset the length every few iterations, so that $64$-bit integers suffice?

Just like $a$ and $b$ can be represented as weighted sums of $a_0, b_0$, $u$ and $v$ can be represented as weighted sums of their earlier versions $u_0, v_0$:

$$
\begin{cases}
u = f_0 u_0 + g_0 v_0 \\
v = f_1 u_0 + g_1 v_0
\end{cases}
$$

The trick is to save $u_0, v_0$ and update short coefficients $f_i, g_i$ instead of long values $u, v$ in the loop. We start with $u_0 = 1, v_0 = 0$ and trivial coefficients:

$$
\begin{cases}
u = u_0 = 1 \, u_0 + 0 \, v_0 \\
v = v_0 = 0 \, u_0 + 1 \, v_0
\end{cases}
$$

When the coefficients $f_i, g_i$ grow past $64$ bits, we pause, compute $u, v$ based on these formulas, replace $u_0, v_0$ with $u, v$, and reset the coefficients $f_i, g_i$ back to trivial, bringing the length back to $1$.

```rust expansible
let mut u0 = 1;
let mut v0 = 0;
let mut q = a.trailing_zeros();

while a != 0 {
    // The coefficients relating (u, v) to (u0, v0).
    let mut (f0, g0) = (1, 0);
    let mut (f1, g1) = (0, 1);
    let mut p = 0;

    // Run the algorithm until p reaches the limit.
    while a != 0 && p + q <= 62 {
        a >>= q;
        f1 <<= q;
        g1 <<= q;
        p += q;

        q = (a - b).trailing_zeros();

        if a < b {
            (a, b) = (b - a, a);
            (f0, f1) = (f1, f0);
            (g0, g1) = (g1, g0);
        } else {
            (a, b) = (a - b, b);
        }
        f0 -= f1;
        g0 -= g1;
    }

    // This section means different things depending on the reason the loop stopped:
    // - If we ran out of precision, this performs as much of the last action as possible and
    //   adjusts `q` so that the operation completes on the next iteration.
    // - If `a = 0`, this effectively raises the precision of f1/g1 to 62. It doesn't adjust
    //   `f0, g0` correctly, but this doesn't matter because `u` is not read on the exit path.
    a >>= 62 - p;
    f1 <<= 62 - p;
    g1 <<= 62 - p;
    q -= 62 - p;

    // Apply the coefficients.
    let f0 = redc62(f0);
    let g0 = redc62(g0);
    let f1 = redc62(f1);
    let g1 = redc62(g1);
    (u0, v0) = ((f0 * u0 + g0 * v0) % b0, (f1 * u0 + g1 * v0) % b0);
}

assert!(b == 1, "not invertible");
return v0;
```


### Vectorization

The astute among you might realize this doesn't improve much, since we went from updating two $128$-bit numbers in a loop to updating four $64$-bit numbers in a loop. But since we apply the exact same operations to $f_i$ and $g_i$, we can vectorize them.

:::aside
This technique is called [SWAR](https://en.wikipedia.org/wiki/SWAR). It was invented before hardware SIMD support existed, but it's useful to this day. I wrote about another application of SWAR [here](../i-sped-up-serde-json-strings-by-20-percent/).
:::

We can't use SIMD because x86 doesn't have `cmov` for vector registers, but we can decrease the coefficient length to $32$ bits and pack two coefficients into one integer:

$$
\begin{cases}
c_0 = f_0 + 2^{32} \, g_0 \\
c_1 = f_1 + 2^{32} \, g_1
\end{cases}
$$

<aside-inline-here />

This simplifies the inner loop to:

```rust
while a != 0 && p + q <= 30 {
    a >>= q;
    c1 <<= q;
    p += q;

    q = (a - b).trailing_zeros();

    if a < b {
        (a, b) = (b - a, a);
        (c0, c1) = (c1, c0);
    } else {
        (a, b) = (a - b, b);
    }
    c0 -= c1;
}
```

Just like $u$ and $v$, $c_0$ and $c_1$ take $p + 2$ bits, so we limit $p$ by $32 - 2 = 30$. But with care, we can squeeze out one more bit. Recall the inequalities:

$$
\begin{cases}
-2^{p+1} < u < 2^{p+1} \\
-2^p < v \le 2^p
\end{cases}
$$

Only $u$ takes $p + 2$ bits. $v$ fits in $p + 1$, if barely: signed integer types represent the range $[-2^p; 2^p - 1]$, while this is $[-2^p + 1; 2^p]$, but the number of distinct values is the same. So even if we run out of the $30$-bit limit, we can shift $v$ once more. This affects the code after the inner loop:

```rust
// 31 would be 30 without this optimization
a >>= 31 - p;
c1 <<= 31 - p;
q -= 31 - p;

let (f0, g0) = parse_coefficients(c0);
let (f1, g1) = parse_coefficients(c1);
let f0 = redc31(f0);
let g0 = redc31(g0);
let f1 = redc31(f1);
let g1 = redc31(g1);
(u0, v0) = ((f0 * u0 + g0 * v0) % b0, (f1 * u0 + g1 * v0) % b0);
```

Note that the inner loop is still limited by $30$, since it not only shifts $v$, but also subtracts from $u$, which could cause an overflow with a limit of $31$.

:::aside
Pornin uses a simpler way to pack $f_i, g_i$ into $c_i$: by adding $2^{31} - 1$ to $f_i$ and $g_i$, we can ensure the two parts don't interfere with each other's bits. But this makes arithmetic slower due to conversions between the biased and non-biased forms.
:::

Parsing coefficients from $c_i$ is slightly tricky due to the unusual signed integer format, but not impossibly so:

$$
\mathrm{int}(x) = \begin{cases}
    x & \text{if } x \le 2^{31} \\
    x - 2^{32} & \text{if } x > 2^{31}
\end{cases}
$$

$$
\begin{cases}
f_i = \mathrm{int}(c_i \bmod 2^{32}) \\
g_i = \mathrm{int} \left( \left\lfloor \dfrac{c_i + 2^{31} - 1}{2^{32}} \right\rfloor \right)
\end{cases}
$$

This assumes that $c_i$ is stored in an unsigned type.

<aside-inline-here />


### Symmetry

With packed coefficients, the inner loop is similar to the unoptimized version, differing only in $c_0, c_1$ vs $u, v$. This allows us to cheaply combine two approaches: track the true values $u, v$ for the first $62$ iterations and then switch to coefficients. It's faster than relying on coefficients alone because it recalculates $u_0, v_0$ less often.

The final implementation looks something like this:

```rust expansible
let mut u0 = 1;
let mut v0 = 0;
let mut q = a.trailing_zeros();
let mut is_first_iteration = true;

while a != 0 {
    // Either coefficients in SWAR format, or the values u/v, depending on the iteration.
    let mut c0 = 1;
    let mut c1 = if is_first_iteration { 0 } else { 1 << 32 };
    let mut p_left = if is_first_iteration { 63 } else { 31 };

    while a != 0 && q < p_left { // < instead of <= is load-bearing
        a >>= q;
        c1 <<= q;
        p_left -= q;

        q = (a - b).trailing_zeros();

        if a < b {
            (a, b) = (b - a, a);
            (c0, c1) = (c1, c0);
        } else {
            (a, b) = (a - b, b);
        }
        c0 -= c1;
    }

    a >>= p_left;
    c1 <<= p_left;
    q -= p_left;

    if is_first_iteration {
        u0 = redc63(c0);
        v0 = redc63(c1);
    } else {
        let (f0, g0) = parse_coefficient(c0);
        let (f1, g1) = parse_coefficient(c1);
        let f0 = redc31(f0);
        let g0 = redc31(g0);
        let f1 = redc31(f1);
        let g1 = redc31(g1);
        (u0, v0) = ((f0 * u0 + g0 * v0) % m, (f1 * u0 + g1 * v0) % m);
    }

    is_first_iteration = false;
}

assert!(b == 1, "not invertible");
return v0;
```

We store `p_left` instead of `p` so that `p_left -= q` and `q < p_left` can be computed with a single instruction.

The $32$-bit and $64$-bit cases can use the same implementation, as replacing `q < p_left` with `true` makes it identical to the $32$-bit algorithm, and compilers recognize this.

`redc31(x)` can be implemented as `redc63(x << 32)`.

And that's it! You now know a cool way to compute $64$-bit modular inverses.


### General case

To support variable $b_0$, we can compute $j = b_0^{-1} \bmod 2^{64}$ in runtime. This can be done very quickly with [an algorithm by Jeffrey Hurchalla](https://arxiv.org/pdf/2204.04342).

$j$ only exists if $b_0$ is odd. If it's even, swap $a_0$ and $b_0$. If both are even, divide them by their common power of two and choose whichever becomes odd as $b_0$.

To replace the extended Euclidean algorithm, we need to find *integers* $x, y$ such that:

$$
a_0 x + b_0 y = \mathrm{gcd}(a_0, b_0)
$$

Luckily, our $v$ is no longer a fraction, but rather a remainder modulo $b_0$, so we can substitute $x = v \bmod b_0$. $y$ can then be computed from the equation:

$$
y = \frac{\mathrm{gcd}(a_0, b_0) - a_0 x}{b_0} = \frac{b - a_0 x}{b_0}
$$

Since this division is exact, it can be calculated with multiplication by $j$:

$$
y = j \cdot (b - a_0 x) \pmod{2^{64}}
$$

Despite this complexity, I believe this method can be faster than the extended Euclidean algorithm, since the auxiliary logic takes constant time, except for computing $j$ in $\mathcal{O}(\log k) = \mathcal{O}(\log \log a)$, which is still pretty good.


### Outro

As a reminder, you can find my code [on GitHub](https://github.com/purplesyringa/mod2k/blob/104603af3866ac274073a5b2af28f7a41550add1/src/xgcd.rs). The source of latency-optimized GCD is [this post](https://lemire.me/blog/2024/04/13/greatest-common-divisor-the-extended-euclidean-algorithm-and-speed/). Using coefficients to reset bit lengths of $u, v$ comes from [this paper](https://eprint.iacr.org/2020/972.pdf), which also covers the case when values don't fit in general-purpose registers.

Thanks to many friends of mine for contributing to the benchmarking results, to Ian Qvist for the motivation to complete this post and editorial comments, and to Yuki for saving me from going insane over unexplainable performance phenomena.
