---
title: Division is hard, but it does not have to be
time: August 24, 2024
intro: |
    dasdasdas.
---

Developers don't usually divide numbers all the time, but hashmaps often need to compute [remainders](https://en.wikipedia.org/wiki/Remainder) modulo a prime. Hashmaps are really common, so fast division is useful.

For instance, rolling hashes might compute `u128 % u64` with a fixed divisor. Compilers just drop the ball here:

```rust
fn modulo(n: u128) -> u64 {
    (n % 0xffffffffffffffc5) as u64
}
```

```x86asm
modulo:
    push    rax
    mov     rdx, -59
    xor     ecx, ecx
    call    qword ptr [rip + __umodti3@GOTPCREL]
    pop     rcx
    ret
```

`__umodti3` is a generic long division implementation, and it's slow and ugly.

I prefer my code the opposite of slow and ugly.


### And I know math

`0xffffffffffffffc5` is $2^{64} - 59$; in fact it's the largest prime below $2^{64}$, but what I really care about is that it's oh so close to $2^{64}$.

So I'm going to use a trick. I'll subtract a multiple of $2^{64} - 59$ from `n`, which won't affect the reminder. Namely, I wish to transform $n$ to $n - \left\lfloor \frac{n}{2^{64}} \right\rfloor \cdot (2^{64} - 59)$, which can be simplified to $n \bmod 2^{64} + \left\lfloor \frac{n}{2^{64}} \right\rfloor \cdot 59$.

```rust
fn modulo(mut n: u128) -> u64 {
    n = n % (1 << 64) + (n >> 64) * 59;
    (n % 0xffffffffffffffc5) as u64
}
```

Computers divide by $2^{64}$ all the time; this operation is almost free. So at the cost of a single multiplication by $59$, $n$ up to $2^{128} - 1$ is transformed to an equivalent $n$ up to $(2^{64} - 1) \cdot 60$.

Then I apply the same operation twice, further reducing to $n$ up to $2^{64} + 59^2 - 1$. *This* remainder can be computed with a single `if`.

```rust
fn modulo(mut n: u128) -> u64 {
    n = n % (1 << 64) + (n >> 64) * 59;
    n = n % (1 << 64) + (n >> 64) * 59;
    if n >= 0xffffffffffffffc5 {
        n -= 0xffffffffffffffc5;
    }
    n as u64
}
```

```x86asm
modulo:
    mov     rax, rsi
    mov     esi, 59
    mul     rsi
    mov     rcx, rax
    add     rcx, rdi
    adc     rdx, 0
    mov     rax, rdx
    mul     rsi
    add     rax, rcx
    adc     rdx, 0
    xor     ecx, ecx
    mov     rsi, -60
    cmp     rsi, rax
    sbb     rcx, rdx
    lea     rcx, [rax + 59]
    cmovb   rax, rcx
    ret
```

---

Oh, and it's not like hard-coding $2^{64} - 59$ was necessary. Two iterations suffice for any divisor $\ge 2^{64} - 2^{32} + 1$. Need more primes? Choose away, there's a lot of them in the $2^{32}$-long region.

Need a smaller divisor? Three iterations work for $n \ge 2^{64} - 6981461082631$ (42.667 bits compared to 32 for two iterations), four for $n \ge 2^{64} - 281472113362716$ (48 bits). Sounds like a lot? That's still better than `__umodti3`.

And this method works for division too, not just modulo:

```rust
fn divide(mut n: u128) -> u128 {
    let mut quotient = n >> 64;
    n = n % (1 << 64) + (n >> 64) * 59;
    quotient += n >> 64;
    n = n % (1 << 64) + (n >> 64) * 59;
    if n >= 0xffffffffffffffc5 {
        quotient += 1;
    }
    quotient
}
```


### It gets better

What if you *don't* need a large prime? What if you just need something large that isn't a power of two? Say, $2^{64} - 1$? Let's start anew:

```rust
fn modulo(mut n: u128) -> u64 {
    n = n % (1 << 64) + (n >> 64) * 1;
    (n % u64::MAX as u128) as u64
}
```

The first line now just sums two halves of the number! It's not really hard to get optimal code starting from here; in fact, even the compiler realizes this immediately:

```rust
fn modulo(n: u128) -> u64 {
    (n % u64::MAX as u128) as u64
}
```

```x86asm
modulo:
    add     rdi, rsi
    adc     rdi, 0
    xor     eax, eax
    cmp     rdi, -1
    cmovne  rax, rdi
    ret
```


### This isn't all

Rolling hashes in particular don't need *the* remainder. They need *a* representation of a number. It's not like it matters if a number divisible by $2^{64} - 59$ is represented by $0$ or $2^{64} - 59$ during computation of the hash, as long as it's all mapped to $0$ at the very end of the computation.

So we can modify the code *just so*:

```rust
fn reduce(mut n: u128) -> u64 {
    n = n % (1 << 64) + (n >> 64) * 59;
    n = n % (1 << 64) + (n >> 64) * 59;
    if n >= 0x10000000000000000 /* n >= 0xffffffffffffffc5 */ {
        n -= 0xffffffffffffffc5;
    }
    n as u64
}
```

...which reduces the assembly a bit. The effect is most prominent with $2^{64} - 1$:

```rust
fn reduce(mut n: u128) -> u64 {
    n = n % (1 << 64) + (n >> 64);
    if n >= 1 << 64 {
        // Actually n - ((1 << 64) - 1), but that's not optimized well enough.
        (n + 1) as u64
    } else {
        n as u64
    }
}
```

```x86asm
reduce:
    mov     rax, rdi
    add     rax, rsi
    adc     rax, 0
    ret
```


### Benchmarks

I'm going to compare these implementations:

```rust
fn modulo_naive(n: u128) -> u64 {
    (n % 0xffffffffffffffc5) as u64
}

fn modulo_optimized(mut n: u128) -> u64 {
    n = n % (1 << 64) + (n >> 64) * 59;
    n = n % (1 << 64) + (n >> 64) * 59;
    if n >= 0xffffffffffffffc5 {
        n -= 0xffffffffffffffc5;
    }
    n as u64
}

fn reduce(mut n: u128) -> u64 {
    n = n % (1 << 64) + (n >> 64) * 59;
    n = n % (1 << 64) + (n >> 64) * 59;
    if n >= 0x10000000000000000 {
        n -= 0xffffffffffffffc5;
    }
    n as u64
}

fn divide_naive(n: u128) -> u128 {
    n / 0xffffffffffffffc5
}

fn divide_optimized(mut n: u128) -> u128 {
    let mut quotient = n >> 64;
    n = n % (1 << 64) + (n >> 64) * 59;
    quotient += n >> 64;
    n = n % (1 << 64) + (n >> 64) * 59;
    if n >= 0xffffffffffffffc5 {
        quotient += 1;
    }
    quotient
}
```

|Test                    |Time/iteration (ns)|Speedup                  |
|------------------------|-------------------|-------------------------|
|`modulo_naive`          |25.421             |(base)                   |
|`modulo_optimized`      |2.6755             |9.5x                     |
|`reduce`                |2.2016             |11.5x                    |
|`divide_naive`          |25.366             |(base)                   |
|`divide_optimized`      |2.8677             |8.8x                     |



### So what?

In all honesty, this is not immediately useful when applied to rolling hashes. `reduce` is still a little slower than 1/2 of a `u64 % u32` modulo, so if computing the hash modulo two 32-bit primes rather than one 64-bit prime suffices for you, do that. Still, if you need the best guaranteed collision rate as fast as possible, this is the way.

It's a free optimization for compilers to perform too. It's quite possible that I'm not just unfamiliar with practical applications. Also, hey, it's one more trick you might be able to apply elsewhere now that you've seen it.
