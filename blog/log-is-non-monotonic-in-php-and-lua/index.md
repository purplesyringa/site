---
title: log is non-monotonic in PHP and Lua
time: July 22, 2026
intro: |
    If $a > b > 1$ and $x > 1$, you can prove that $\log_a x < \log_b x$. (As a reminder, $\log_a x$ denotes the value $t$ such that $x = a^t$.) This is very intuitive if you think about it: for "normal" numbers, the greater $a$, the smaller $t$ you will need to get the same $x$.

    Yet if you ask PHP what it thinks, it will tell you you're wrong in a couple rare cases:

    ```php
    <?php
    $x = 2.93;
    $a = 10 + 2 ** -49;
    $b = 10;
    assert($a > $b);
    var_dump(log($x, $a) < log($x, $b));
    var_dump(log($x, $a) == log($x, $b));
    ```

    To be clear, this is **not** the usual floating-point inaccuracy. Everyone already knows floating-point operations are imprecise and it wouldn't be fun to blog about. This example was deliberately engineered to trigger something different.
---

If $a > b > 1$ and $x > 1$, you can prove that $\log_a x < \log_b x$. (As a reminder, $\log_a x$ denotes the value $t$ such that $x = a^t$.) This is very intuitive if you think about it: for "normal" numbers, the greater $a$, the smaller $t$ you will need to get the same $x$.

Yet if you ask PHP what it thinks, it will tell you you're wrong in a couple rare cases:

```php
<?php
$x = 2.93;
$a = 10 + 2 ** -49;
$b = 10;
assert($a > $b);
var_dump(log($x, $a) < log($x, $b));
var_dump(log($x, $a) == log($x, $b));
```

To be clear, this is **not** the usual floating-point inaccuracy. Everyone already knows floating-point operations are imprecise and it wouldn't be fun to blog about. This example was deliberately engineered to trigger something different.

It would be completely natural if the produced result indicated $\log_a x = \log_b x$ instead of $<$: not all real numbers can be exactly represented as floats, so rounding can cause close results to appear equal. In fact, on the same numbers, Python's `math.log` produces the "equals" result. In PHP, however, the result somehow *flips*. And in Lua, too! But not in Rust or C#. All on the same machine and OS! How come?

Also note that I'm only changing the base: if I also changed the argument, I would find lots of counterexamples that work in any language, e.g.:

```php
log(243 ** 3, 3 ** 3) != log(243, 3)
```

...because the formula used for `log` is imperfect. That's not what we're discussing.


### Log base

<aside-start-here />

To find why this happens, let's discuss how languages *usually* implement `math.log`.

:::aside
"Usually" because `libc` has multiple independent implementations.
:::

`libm`, the library that provides [transcendental functions](https://en.wikipedia.org/wiki/Transcendental_function), exposes multiple functions for computing logarithms: `log`, `log10`, `log2`, etc. Each function handles a single base: `log` uses base $e$, `log10` uses base $10$, and so on. While there are no guarantees on their precision, they are usually pretty good, and at least monotonic ([brute-force](https://play.rust-lang.org/?version=stable&mode=release&edition=2024&gist=0451f055ca6b29eb8f24287f7f699dea)).

But there is no function for an arbitrary base, so languages providing a two-argument `log` have to cheat. Mathematically, $\log_a x = \frac{\ln x}{\ln a}$, so you can compute any logarithm from two base-$e$ ones.

It's a little imprecise, which is why `math.log(243^3, 3^3) == math.log(243, 3)` fails: double rounding from `ln` and the division makes the computed value slightly off.

But this doesn't explain the case when $x$ is unchanged, which the post opens with. In that example, the numerator stays the same ($\ln x$) and the denominator decreases (at least symbolically), and somehow that *decreases* the result? Weird, even for floats!

Even more weird is the fact that if you actually *compute* $\ln a$ and $\ln b$ in PHP or Lua, you'll find out that they round to the same value! So somehow neither the numerator, nor the denominator changed, but the result changed??


### Solution

You probably already see the reason. $b = 10$ is a pretty specific counter-example, and there's a suspicious `log10`-shaped hole in the double-`log` precision error.

PHP and Lua don't *always* use the $\frac{\ln x}{\ln a}$ formula. For the bases that `libm` implements directly, i.e. base $10$ and base $2$, they call the corresponding functions (`log10` and `log2`) without going through the natural logarithm. So the code in question doesn't compare two $\frac{\ln x}{\ln a}$ calculations, but rather `log(x) / log(10 + eps)` to `log10(x)`. Since these are completely different methods, it shouldn't be a surprise that they may result in errors in different directions.

The intention is good: when applicable, `log10` provides a more precise, faster result. But combining two evaluation methods results in a discontinuity at the boundary, which breaks reasonable assumptions that the methods satisfy separately!


### Rambling

I wouldn't necessarily call this a bug, but it certainly is an overlooked issue. This is, unfortunately, quite common in the floating-point world: [IEEE-754](https://en.wikipedia.org/wiki/IEEE_754) itself is incredibly robust, but thoughtless implementation decisions here and there contaminate it to the point where people attribute just about any FP bug to intrinsic imprecision.

For Lua, which only provides `math.log`, but not `math.log10`, the *right* thing to do would be to add `math.log10` and remove the special casing from `math.log`. This way, people who use fixed base $10$ can use the faster and simpler `math.log10` method, which could provide better precision boundaries, while people who use a variable base will benefit from not having an edge case to worry about. Oh wait, [they did pretty much the exact opposite in Lua 5.2](https://www.lua.org/manual/5.2/manual.html#8.2)! I like it when people value correctness.

PHP, on the other hand, special-cases bases $10$ and $2$ despite already having `log10`, supposedly for people who don't RTFM, which I wouldn't put past their target audience, if only anyone could be insane enough to try to guess PHP function names. I am getting mixed signals, so I'd better finish this post before I get a headache.

Curiously, PHP and C# also special-case `log(x, 1)` to return `NaN` regardless of `x`, while Lua and Python typically return $\pm \infty$. Truly a diverse ecosystem.

Oh, and also: [PHP's signature for `log`](https://www.php.net/manual/en/function.log.php) says the default `base` is `M_E` (the *approximation* of $e$), but docs say `log` without a base returns the natural logarithm. So does `log(x)` compute $\log_{\mathrm{round}(e)} x$ or $\log_e x$? (This question is more important than it seems: for instance, `sin(M_PI)` *correctly* returns a non-zero value, because `M_PI` is not exactly $\pi$.) Lua at least has the decency to [weasel-word around it](https://www.lua.org/manual/5.2/manual.html#pdf-math.log). Spoiler: it's the latter, but in fact `log(x, M_E)` would produce the same result, because the rounding in $\frac{\ln x}{\ln a}$ is bad enough that the two coincide.
