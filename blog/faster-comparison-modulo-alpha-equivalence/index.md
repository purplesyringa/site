---
title: Faster comparison modulo α-equivalence
ogTitle: Faster comparison modulo alpha-equivalence
time: November 27, 2025
draft: true
intro: |
    Given a $\lambda$-calculus expression, suppose we want to quickly find all of its $\alpha$-equivalent subterms, i.e. subexpressions that are syntactically identical up to renaming variables defined inside the expression.

    For example, consider:

    $$
    \lambda a. (\lambda x. a x) (\lambda t. (\lambda y. a y) (\lambda b. \lambda x. b x))
    $$

    The terms $\lambda x. a x$ and $\lambda y. a y$ are $\alpha$-equivalent, since $x$ can be renamed to $y$, and both $x$ and $y$ are declared within the corresponding terms. The terms $\lambda x. a x$ and $\lambda x. b x$ are not $\alpha$-equivalent, since $a$ and $b$ are distinct variables declared outside the terms.

    This article describes how to:

    1. Calculate hashes of subterms in $\mathcal{O}(n)$ time, such that hashes of $\alpha$-equivalent subterms are guaranteed to match, and hashes of non-$\alpha$-equivalent subterms don't match with high probability.
    2. Calculate equivalence classes in $\mathcal{O}(n \log n)$ so that the comparison can be performed without risk of false positive.
---

This article is a technical counterpart of my previous post [Finding duplicated code with tools from your CS course](../finding-duplicated-code-with-tools-from-your-cs-course/). It is deliberately written in a terse manner, and I'm not going to hold your hand. Consider reading the previous post first and coming back here later.


### Introduction

Given a $\lambda$-calculus term, suppose we want to find all of its $\alpha$-equivalent subterms. Terms $t_1$ and $t_2$ are considered $\alpha$-equivalent (denoted as $t_1 \sim t_2$ in this article) if they are syntactically equal up to a bijection between their bound variables. For example, $t_1 = \lambda x. a \, x$ and $t_2 = \lambda y. a \, y$ are $\alpha$-equivalent because the bijection $\{ x \mapsto y \}$ translates $t_1$ to $t_2$, and $t_1 = \lambda x. a \, x$ and $t_2 = \lambda x. b \, x$ are not $\alpha$-equivalent because $a$ and $b$ are free in $t_1$ and $t_2$ respectively.

Terms are $\alpha$-equivalent if and only if their *locally nameless* forms are syntactically equal. The locally nameless form of a term $t$ represents variables free in $t$ by name, and variables bound in $t$ by de Bruijn index. For example, $\lambda x. a \, x$ is represented as $\lambda. a \, \underline1$. While computing the locally nameless form of a single term is straightforward, efficiently computing forms of all subterms of a term is tricky, since whether a variable is free or bound depends on the term whose form is being computed.

This article describes:

1. A linear-time algorithm for computing hashes of subterms up to $\alpha$-equivalence, i.e. hashes of their locally nameless forms. We prove a bound on the collision rate of non-$\alpha$-equivalent subterms.
2. A linear-time algorithm for validating the resulting hashes for lack of collisions. Together with 1., this produces a reliable classification algorithm with expected linear runtime.
3. An algorithm for computing $\alpha$-equivalence classes in $\mathcal{O}(n \log n)$ guaranteed time, as a deterministic alternative to 1.+2.


### Prior art

Our first algorithm is an adaptation of the algorithm developed in:

> Krzysztof Maziarz, Tom Ellis, Alan Lawrence, Andrew Fitzgibbon, and Simon Peyton Jones. 2021. [Hashing modulo alpha-equivalence](https://arxiv.org/abs/2105.02856). In Proceedings of the 42nd ACM SIGPLAN International Conference on Programming Language Design and Implementation (PLDI 2021). Association for Computing Machinery, New York, NY, USA, 960–973. https://doi.org/10.1145/3453483.3454088

Maziarz et al.'s algorithm has $\mathcal{O}(n \log^2 n)$ runtime, but can be straightforwardly adjusted to expected $\mathcal{O}(n \log n)$ time by replacing binary trees with hash tables. Crucially, this algorithm allows hashes to be computed *incrementally*. It achieves this by producing *e-summaries*, which represent the entire contents of a term up to $\alpha$-equivalence, and efficiently combining e-summaries in application terms. We believe this "purely functional" approach does not allow for faster algorithms, so our algorithm expects the entire expression to be provided upfront.

To the best of our knowledge, our algorithm for validating hashes is novel.

The third algorithm is an adaptation of:

> Lasse Blaauwbroek, Miroslav Olšák, and Herman Geuvers. 2024. [Hashing Modulo Context-Sensitive α-Equivalence](https://arxiv.org/abs/2401.02948). Proc. ACM Program. Lang. 8, PLDI, Article 229 (June 2024), 24 pages. https://doi.org/10.1145/3656459

Our algorithm has the same asymptotic complexity as described in the paper, but is adjusted to non-context-sensitive $\alpha$-equivalence and simplified, which hopefully leads to easier intuitive understanding and faster practical performance.


### Hashing

We start with a named form, where all variables are accessed by names. This ensures that the innermost terms are already in the locally nameless form. We then compute the locally nameless forms of other terms recursively:

$$
\begin{align*}
\mathrm{repr}(x) &= x \\
\mathrm{repr}(t_1 t_2) &= \mathrm{repr}(t_1) \, \mathrm{repr}(t_2) \\
\mathrm{repr}(\lambda x. t) &= \lambda. \mathrm{repr}(t)[x := \underline{0}]
\end{align*}
$$

$[x := \underline{0}]$ denotes that the form of $\lambda x. t$ is computed from the form of $t$ by replacing mentions of $x$ with de Bruijn indices. This replacement is the crux of the problem: while it can be easily performed on strings, the (possibly very long) strings then need to be rehashed on each iteration, since we want to compute the hash of each term.

However, some string hashes, most commonly rolling hashes, allow the hash to be recomputed efficiently if part of the string is changed. Adjusting $\mathrm{repr}$ to return such a hash allows the rewrite $[x := \underline{0}]$ to be performed directly on the hash. Consider in particular the polynomial hash parameterized by a constant $b$, chosen randomly, and a prime number $p$:

$$
\mathrm{hash}(c_0 c_1 \dots c_{n-1}) = \sum_i c_i b^i \mod p.
$$

A character at index $i$ can be changed from $x$ to $y$ by adding $(y - x) b^i$ to the hash value. The hash of $\lambda. \mathrm{repr}(t)$ and the "patch" replacing each mention of $x$ with a de Bruijn index can be computed separately and then merged at the abstraction, since the offset of a given variable mention within the corresponding abstraction can be calculated efficiently, and patches can be merged by adding them together.

An implementation of the algorithm is reproduced below. To avoid handling parentheses, we implicitly translate terms to postfix notation, denoting calls with `!`.

```python expansible
range_of_expr: dict[Expr, tuple[int, int]] = {}
variable_nesting: dict[VariableName, int] = {}
variable_accesses: dict[VariableName, list[tuple[int, int]]] = {}
current_location: int = 0

def collect_locations(expr: Expr, nesting: int):
    global current_location
    start = current_location
    match expr:
        case Variable(x):
            # x
            current_location += 1
            variable_accesses[x].append((start, nesting - variable_nesting[x]))
        case Abstraction(x, body):
            # body, \
            variable_nesting[x] = nesting
            variable_accesses[x] = []
            collect_locations(body, nesting + 1)
            current_location += 1
        case Application(f, a):
            # f, a, !
            collect_locations(f, nesting)
            collect_locations(a, nesting)
            current_location += 1
    end = current_location
    range_of_expr[expr] = (start, end)

collect_locations(root, 0)

powers_of_b: list[int] = [1]

# Computes `h * b ** count % p` in amortized constant time.
def shift(h: int, count: int) -> int:
    while len(powers_of_b) <= count:
        powers_of_b.append(powers_of_b[-1] * b % p)
    return h * powers_of_b[count] % p

# Functions capable of hashing variable names, de Bruijn indices, and the characters \, ! without
# collisions.
def hash_lambda() -> int: return 1
def hash_call() -> int: return 2
def hash_variable_name(x: VariableName) -> int: return x.int_id * 2 + 3
def hash_de_bruijn_index(i: int) -> int: return i * 2 + 4

def calculate_hashes(expr: Expr) -> int:
    start, end = range_of_expr[expr]
    match expr:
        case Variable(x):
            h = hash_variable_name(x)
        case Abstraction(x, body):
            h = calculate_hashes(body) + shift(hash_lambda(), end - start - 1)
            for location, de_bruijn_index in variable_accesses[x]:
                h += shift(
                    hash_de_bruijn_index(de_bruijn_index) - hash_variable_name(x),
                    location - start,
                )
                h %= p
        case Application(f, a):
            h = (
                calculate_hashes(f)
                + shift(calculate_hashes(a), range_of_expr[a][0] - start)
                + shift(hash_call(), end - start - 1)
            )
    h %= p
    print("The hash of", expr, "is", h)
    return h

calculate_hashes(root)
```

The probabilistic guarantees of this scheme depend entirely on the choice of the hash. The collision probability of rolling hashes typically scales linearly with the length of the input. In this case, the length of the input exactly matches the number of subterms $n$, and each element of the input is a $\log n + \mathcal{O}(1)$-bit number (assuming binary logarithm from now on).

For polynomial hashes, the collision probability is $\le \frac{n - 1}{p}$, assuming $b$ is chosen randomly. If $b$ is instead fixed and $p$ is chosen randomly, the probability is $\le C \frac{n \log n}{p}$, where $C$ depends on how wide the range $p$ is chosen from is. For Rabin fingerprints, the probability is $\lesssim \frac{n \log n}{2^{\deg p(x)}}$.


### Verification

To verify that the computed hashes don't produce collisions, we group terms by their hashes and validate that in each group of size $\ge 2$, all terms are $\alpha$-equivalent. We first check that terms within each group have equal sizes (i.e. the number of subterms, denoted $\left\lvert t \right\rvert$), and then iterate over groups in order of increasing size. This ensures that while validating terms of size $n$, terms of sizes $m < n$ can be compared for $\alpha$-equivalence by hash.

We now introduce some terminology.

- We call subterms with non-unique hashes (i.e. subterms that are not alone in their groups) *pivots*.

- We say an optimized predicate for $\alpha$-equivalence is *sound* if it implies $\alpha$-equivalence, and *complete* if it is implied by $\alpha$-equivalence.

- For a term $t$ and a subterm $u$, we define the *path* from $t$ to $u$ (written $t \leadsto u$) as a (possibly empty) string of characters $\downarrow$, $\swarrow$, $\searrow$, where $\downarrow$ means "proceed into the body of the abstraction" and $\swarrow$/$\searrow$ mean "proceed into the function/argument of the application" respectively. For fixed $t$, valid paths map bijectively to subterms of $t$.

- For a term $t$ and a subterm $u$, we denote by $\mathrm{repr}(u/t)$ a representation of $u$ that encodes variable mentions as follows:
    - Variables bound in $u$ use de Bruijn indices.
    - Variables free in $t$ use names.
    - Variables bound in $t$ but free in $u$ use paths from $t$ to the declaring abstraction.

  Note that $\mathrm{repr}(t/t) = \mathrm{repr}(t)$. The act of writing $u/t$ implicitly states that $u$ is a subterm of $t$ akin to $\frac{x}{y}$ implying $y \ne 0$ in arithmetic.

- We write $e_1 \sim e_2$, where each $e_i$ is either $u_i$ or $u_i/t_i$ independently, if $\mathrm{repr}(e_1) = \mathrm{repr}(e_2)$. For example, $x/\lambda x. x \sim y/\lambda y. y$ even though $x \not\sim y$.

We rely on the following propositions:

1. If $u_1 \sim u_2$ and $\left\lvert t_1 \right\rvert = \left\lvert t_2 \right\rvert$ are distinct terms, then $u_1/t_1 \sim u_2/t_2$. Indeed, $\mathrm{repr}(u_1)$ differs from $\mathrm{repr}(u_1/t_1)$ at mentions of variables that are free in $u_1$ but bound in $t_1$. But since $t_1$ and $t_2$ have the same size, they don't share subterms, so $u_1 \sim u_2$ implies $u_1$ doesn't mention any variables bound in $t_1$ but not $u_1$. Hence $u_1/t_1 \sim u_1 \sim u_2 \sim u_2/t_2$.

2. If $u_1/t_1 \sim u_2/t_2$ and there is a $u_1' \sim u_1$ that isn't a subterm of $t_1$, then $u_1 \sim u_2$. Indeed, $u_1'$ cannot mention variables declared within $t_1$, so $u_1$ can also only mention free variables declared outside $t_1$, hence $u_2$ can also only mention free variables declared outside $t_2$; thus $u_1 \sim u_1/t_1 \sim u_2/t_2 \sim u_2$.

3. If $u \sim u'$, then $u/t \sim u'/t$. Indeed, $\mathrm{repr}(u)$ differs from $\mathrm{repr}(u/t)$ at variables that are free in $u$, but bound in $t$. Such variables are accessed by name in $\mathrm{repr}(u)$, so by $u \sim u'$ they must be accessed by the same name in $\mathrm{repr}(u')$ and correspond to the same declaring abstraction $a$. Since the same $t$ is used in $u/t$ and $u'/t$, the same path $t \leadsto a$ will be used in both $\mathrm{repr}(u/t)$ and $\mathrm{repr}(u'/t)$.

4. If $u/t \sim u'/t$, then $u \sim u'$. Indeed, $\mathrm{repr}(u)$ differs from $\mathrm{repr}(u/t)$ at variables that are free in $u$, but bound in $t$. Since such variables are accessed by path in $\mathrm{repr}(u)$, by $u \sim u'$ they must be accessed by the same path in $\mathrm{repr}(u')$. Since the same $t$ is used in $u/t$ and $u'/t$, this path denotes the same abstraction $a$ in both cases, and so $\mathrm{repr}(u)$ and $\mathrm{repr}(u')$ will include the same name (namely, the name of $a$).

5. If $u_1/t_1 \sim u_2/t_2$, $u_1 \sim u_1'$, and $u_2 \sim u_2'$, then $u_1'/t_1 \sim u_2'/t_2$. Indeed, by proposition 3 we have $u_1'/t_1 \sim u_1/t_1$ and $u_2/t_2 \sim u_2'/t_2$, from which the statement follows by transitivity.

6. If $u_1/t_1 \sim u_2/t_2$, $u_1'/t_1 \sim u_2'/t_2$, and $u_1 \sim u_1'$, then $u_2 \sim u_2'$. Indeed, by proposition 3 we have $u_1/t_1 \sim u_1'/t_1$, thus $u_2/t_2 \sim u_1/t_1 \sim u_1'/t_1 \sim u_2'/t_2$, from which by proposition 4 $u_2 \sim u_2'$.

7. If $t_1 \sim t_2$ and $(t_1 \leadsto u_1) = (t_2 \leadsto u_2)$, then $u_1/t_1 \sim u_2/t_2$. Indeed, $\mathrm{repr}(u_1/t_1)$ and $\mathrm{repr}(u_2/t_2)$ are identical substrings of the string $\mathrm{repr}(t_1) = \mathrm{repr}(t_2)$.

8. If $t_1 \sim t_2$ and $u_1$ is a subterm of $t_1$, there exists a subterm $u_2$ of $t_2$ such that $u_1/t_1 \sim u_2/t_2$. Indeed, by $t_1 \sim t_2$ the terms $t_1$ and $t_2$ have identical tree structure, so the path $t_1 \leadsto u_1$ is valid in both $t_1$ and $t_2$. Rerooting it at $t_2$, we obtain an identical path $t_2 \leadsto u_2$, and by proposition 7 $u_1/t_1 \sim u_2/t_2$.

9. If a path $t \leadsto p$ does not contain any pivots except $t$ and $p$, $p' \sim p$ is a distinct term from $p$, and a path $t \leadsto u \leadsto p'$ exists, where $u$ is a pivot, then $p$ is not a subterm of $u$. Indeed, $p$ cannot be a strict subterm of $u$ because $t \leadsto p$ would contain another pivot $u$. $p = u$ is also impossible, since $p' \sim p$ would have to be a strict subterm of $p$ due to $p \ne p'$, but a term can never be $\alpha$-equivalent to its strict subterm.

To verify $t_1 \sim t_2$, where $t_1$ and $t_2$ are from the same group, we set $u_1 = t_1, u_2 = t_2$ and assert $u_1/t_1 \sim u_2/t_2$ recursively. At each step, we repeatedly verify that $u_1$ and $u_2$ are subterms of the same "kind" (variable/abstraction/application) and recurse, adjusting $u_1$ and $u_2$ accordingly. We apply two optimizations to ensure the time complexity is subquadratic. For every step except the first, if $u_2$ is a pivot:

- If $u_2$ has an $\alpha$-equivalent copy outside $t_2$, we immediately assert $u_1 \sim u_2$ by hash and don't recurse into $u_1/t_1 \sim u_2/t_2$. This is sound by proposition 1 and complete by proposition 2.

- Otherwise, we look for copies of $u_2$ within $t_2$ (there must be at least one more copy). If this is the first copy we've seen during the current comparison, we recurse into $u_1/t_1 \sim u_2/t_2$ and record the mapping $u_2 \mapsto u_1$. If there is an earlier copy $u_2'$ mapping to $u_1'$, we assert $u_1 \sim u_1'$ by hash and don't recurse. This is sound by proposition 5 and complete by proposition 6.

Note that in the latter case, if $u_2$ is entered, it's guaranteed to be the first copy in DFS order not only among visited terms, but among all terms. Indeed, suppose the earliest copy $u_2'$ was skipped because some of its ancestor pivots $p$ wasn't visited. There could be two reasons for that:

- $p$ has a copy $p'$ outside $t_2$. By proposition 8, there exists $u_2''$ in $p'$ such that $u_2'/p \sim u_2''/p'$. Since $u_2' \sim u_2$ and $u_2$ is not a subterm of $p$, by proposition 2 $u_2' \sim u_2''$. Since $u_2''$ is outside $t_2$ and $u_2 \sim u_2''$, $u_2$ could not be entered.

- $p$ has an earlier copy $p'$ inside $t_2$. Repeat the process from the previous paragraph, finding $u_2'' \sim u_2$. This $u_2''$ is earlier than $u_2'$, so $u_2'$ could not be the earliest copy of $u_2$.

An implementation of this algorithm follows.

```python
def compare(u1: Term, t1: Term, u2: Term, t2: Term, h21: dict[int, int]) -> bool:
    if (u2 is not t2) and (u2 is a pivot):
        if there is any term alpha-equivalent to u2 outside t2:
            return hash[u1] == hash[u2]
        if hash[u2] in h21:
            return h21[hash[u2]] == hash[u1]
        h21[hash[u2]] = hash[u1]

    match (u1, u2):
        case (Variable(x1), Variable(x2)):
            x1 = (x1 as de Bruijn index) if x1 defined within t1 else (x1 as name)
            x2 = (x2 as de Bruijn index) if x2 defined within t2 else (x2 as name)
            return x1 == x2
        case (Application(u11, u12), Application(u21, u22)):
            return compare(u11, t1, u21, t2, h21) and compare(u12, t1, u22, t2, h21)
        case (Abstraction(_, v1), Abstraction(_, v2)):
            return compare(v1, t1, v2, t2, h21)
        case _:
            return False

def verify_hashes():
    # Not implemented: validate that, within each class, all terms have the same size.
    # Not implemented: sort classes by increasing size of terms.
    for class_members in classes:
        t1 = class_members[0]
        for t2 in class_members[1:]:
            if not compare(t1, t1, t2, t2, {}):
                return False
    return True
```

It turns out that this algorithm takes linear time. We will now prove this.

The pair $(u_2, t_2)$ uniquely determines a particular invocation of `compare`. Split such invocations into two categories depending on whether the path $t_2 \leadsto u_2$ contains any pivots except $t_2$ and possibly $u_2$. For visited pairs without such pivots, $u_2$ determines $t_2$ almost uniquely: if $u_2$ is not a pivot, $t_2$ is the closest pivot ancestor; otherwise it's either such an ancestor or $u_2$ itself. This means that the number of visited pairs without pivots inbetween is $\le 2n = \mathcal{O}(n)$. We will now prove that the number of visited pairs with pivots is also linear with amortized analysis.

Consider any path $t_2 \leadsto u_2$ that does contain an additional pivot. Call the highest such pivot $p$, so that $t_2 \leadsto p$ is non-empty and pivot-free except for $t_2$ and $p$, and $p \leadsto u_2$ is non-empty. Since $p \leadsto u_2$ is non-empty, the pivot $p$ must have been recursed into, which only happens if $p$ has no copies outside $t_2$ and is the earliest copy within $t_2$. Call the immediately next copy in DFS order $p'$. Since $p' \sim p$, $p'$ and $p$ have the same tree structure and we can find $u_2'$ such that $(p' \leadsto u_2') = (p \leadsto u_2)$. We "pay" for entering the pair $(u_2, t_2)$ with $u_2'$ and will now demonstrate that all visited pairs pay with different terms, which implies linearity.

Suppose that there are two pairs that pay with the same $u'$: $(u_1, t_1)$ with highest pivot $p_1$ with next copy $p_1'$, and $(u_2, t_2)$ with highest pivot $p_2$ with next copy $p_2'$. $u'$ is a subterm of all of $t_1, t_2, p_1', p_2'$, so there is a linear order on these four terms. Without loss of generality, assume $t_1$ is an ancestor of $t_2$. There are three linear orders matching $t_1 \prec t_2$, $t_1 \prec p_1'$, $t_2 \prec p_2'$ (note that we aren't assuming that all terms in this order are distinct):

1. $t_1 \prec t_2 \prec p_1' \prec p_2'$. By proposition 9, $p_2$ is not a subterm of $p_1'$. By proposition 8, there is $q$ such that $p_2'/p_1' \sim q/p_1$. Since $p_2 \sim p_2'$ and $p_2$ is not a subterm of $p_1'$, by proposition 2 $q \sim p_2'$. By proposition 9, $p_1$ is not a subterm of $t_2$, thus $q$ is not a subterm of $t_2$. This means that $p_2$ could not be entered from $t_2$, since it has a copy $q$ outside $t_2$.

2. $t_1 \prec t_2 \prec p_2' \prec p_1'$. By proposition 9, $p_1$ is not a subterm of $t_2$ or $p_2'$. Since $p_1$ is earlier than $p_1'$ in DFS order, $p_1$ is also earlier than $t_2$. By proposition 8, there is $q$ such that $p_1'/p_2' \sim q/p_2$. Since $p_1 \sim p_1'$ and $p_1$ is not a subterm of $p_2'$, by proposition 2 $q \sim p_1'$. Since $q$ is in $t_2$, it is also later than $p_1$ in DFS order. Since $p_2$ is earlier than $p_2'$, $q$ is earlier than $p_1'$. Thus $q \sim p_1$ is between $p_1$ and $p_1'$ in DFS order, so $p_1'$ cannot be the immediately next copy of $p_1$.

3. $t_1 \prec p_1' \prec t_2 \prec p_2'$. By proposition 8, there are $q$ and $q'$ such that $p_2/p_1' \sim q/p_1$ and $p_2'/p_1' \sim q'/p_1$. By proposition 6, $q \sim q'$. Since $p_2$ is earlier than $p_2'$ in DFS order, $(p_1' \leadsto p_2) = (p_1 \leadsto q)$, and $(p_1' \leadsto p_2') = (p_1 \leadsto q')$, $q$ is earlier than $q'$ in DFS order. Together with $q \sim q'$, this implies $q'$ could not be entered from $t_1$. However, the path $p_1' \leadsto u'$ passes through $p_2'$, so the rerooted path $p_1 \leadsto u_1$ passes through $q'$, and thus $q'$ has to be entered for $u_1$ to be reached.

This proves that the mapping $(t_2, u_2) \mapsto u'$ is injective, and thus this part of the algorithm takes at most linear time, which proves the linear complexity of the entire algorithm.

Notes:

1. The algorithm is linear even under the presence of collisions. The mapping $(t_2, u_2) \mapsto u'$ will be defined over a smaller set of pairs than with perfect hashes, since the algorithm will abort at some point, but will stay injective.

2. The arguments $u_1, t_1$ to `compare` are not taken into consideration during the proof. `compare` can be transformed to `serialize`, which lists non-entered terms as either hash values or backrefs, followed by an assertion that the serialized strings of all terms within a group are equal. This still takes linear time because the total string length is linear. This algorithm can resolve hash collisions locally by splitting groups, but is more complex and requires more memory.

3. The only reason a `serialize`-based algorithm needs to be pre-fed with hashes is to determine which terms are pivots -- the exact hashes or even collisions between pivots are inconsequential. Pivots mostly matter because of the assumption that the path $t \leadsto p$ does not contain other pivots. Hashing is an overkill, but we are not aware of any algorithm for detecting pivots without it.


### Classes

To compute equivalence classes, we use the opposite approach of the one we used for hashing. We start with de Bruijn indices, compute the equivalence class of the root term using hash consing, and recurse, replacing de Bruijn indices with names as necessary.

```python
def rec(t: Term) -> int:
    classes[t] = calculate_class(t)
    match t:
        case Variable(_):
            pass
        case Application(t1, t2):
            rec(t1)
            rec(t2)
        case Abstraction(x, u):
            replace_mentions(x, u)
            rec(u)

# Not implemented: calculate the equivalence class of `t` with hash consing.
def calculate_class(t: Term) -> int: ...

# Not implemented: replace all de Bruijn indices corresponding to `x` within `t` with variable name
# of `x`.
def replace_mentions(x: Variable, t: Term): ...
```

The time complexity of `calculate_class` is $\mathcal{O} \left( \left\lvert t \right\rvert \right)$, where $\left\lvert \cdot \right\rvert$ denotes the number of subterms of $t$, resulting in quadratic time complexity in total.

To fix this, we adjust `rec` to compute not just the class of $t$, but also of some subterms $u$ of $t$, at no additional asymptotic cost, with a trick described below. In particular, we ensure that the classes of all subterms $u$ such that $\left\lvert u \right\rvert \ge \frac12 \left\lvert t \right\rvert$ are guaranteed to be computed. We then recurse into unhandled subterms, which are guaranteed to have size $\left\lvert u \right\rvert < \frac12 \left\lvert t \right\rvert$.

Since $\left\lvert t \right\rvert$ is at worst halved during each recursive invocation, there are at most $\log n$ levels of recursion. Since each subterm only contributes $\mathcal{O}(1)$ amortized to the time complexity per each recursion invocation it's part of, there can be at most $\log n$ such invocations. Hence the total time complexity is $\mathcal{O}(n \log n)$.

The rest of the section explains how to efficiently compute classes of all "large" subterms of $t$.

First, we adjust `calculate_class` to save the hash-consed classes of all subterms of $t$ in `aux_class`. Not all of those classes will be valid answers: for example, in $\lambda. \lambda. a \, \underline1$, the class of the first subterm will be computed as the class of string $\lambda. a \, \underline1$, which has a dangling de Bruijn index ($\lambda. a x$ would be correct).

However, these classes are guaranteed to be valid for all *locally closed* subterms, i.e. subterms without dangling indices. Such subterms can access both variables declared outside $t$ (by name) and variables declared inside themselves (by index), but not variables declared inbetween. To detect locally closed subterms, we calculate the topmost variable that each subterm accesses by the de Bruijn index (`max_index`). If, for a given subterm $u$, the top variable is within $u$, we know $u$ is locally closed.

Now that we've handled all locally closed subterms, it turns out that large non-locally-closed subterms are guaranteed to be globally unique, and thus we can assign an anonymous equivalence class to them without relying on hash consing. Indeed: each non-locally-closed subterm $u$ refers to variables defined within $t$, and can thus only be $\alpha$-equivalent to other subterms of $t$. But since $\left\lvert u \right\rvert \ge \frac12 \left\lvert t \right\rvert$, there isn't enough space within $t$ for another subterm of matching size.

The pseudo-code for the complete algorithm is shown below.

```python expansible
size: dict[Term, int] = {}
aux_class: dict[Term, int] = {}
max_index: dict[Term, int] = {}
out_index: dict[Term, int] = {}

def rec(t: Term) -> int:
    dfs1(t)
    dfs2(t, size[t])

def dfs1(t: Term):
    match t:
        case Variable(x):
            size[t] = 1
            aux_class[t] = hash_cons(x)
            if x is a de Bruijn index:
                max_index[t] = x
            else:  # x is a variable name
                max_index[t] = -1
        case Abstraction(x, u):
            dfs1(u)
            size[t] = 1 + size[u]
            aux_class[t] = hash_cons((aux_class[u],))
            max_index[t] = max_index[u] - 1
        case Application(t1, t2):
            dfs1(t1)
            dfs1(t2)
            size[t] = 1 + size[t1] + size[t2]
            aux_class[t] = hash_cons((aux_class[t1], aux_class[t2]))
            max_index[t] = max(max_index[t1], max_index[t2])

def dfs2(t: Term, root_size: int):
    if max_index[t] < 0:  # locally closed
        out_class[t] = aux_class[t]
    else:
        if 2 * size[t] < root_size:
            rec(t)
            return
        out_class[t] = anonymous_class()

    match t:
        case Variable(_):
            pass
        case Abstraction(x, u):
            replace_mentions(x, u)
            dfs2(u, root_size)
        case Application(t1, t2):
            dfs2(t1, root_size)
            dfs2(t2, root_size)

@functools.cache  # memoize
def hash_cons(arg) -> int:
    return anonymous_class()

last_class = 0
def anonymous_class() -> int:
    global last_class
    last_class += 1
    return last_class
```
