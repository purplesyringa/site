---
title: The sentinel trick
time: August 13, 2024
discussion: https://t.me/alisa_rummages/148
intro: |
    The sentinel trick underlies a data structure with the following requirements:

    - Read element by index in $O(1)$,
    - Write element by index in $O(1)$,
    - Replace all elements with a given value in $O(1)$.

    It is not a novel technique by any means, but it doesn't seem on everyone's lips, so some of you might find it interesting.
---

The sentinel trick underlies a data structure with the following requirements:

- Read element by index in $O(1)$,
- Write element by index in $O(1)$,
- Replace all elements with a given value in $O(1)$.

It is not a novel technique by any means, but it doesn't seem on everyone's lips, so some of you might find it interesting.


### Why?

We could just use a hashmap and store a "default" value. Clearing a hashmap requires no more time than writes do, so the amortized time is $O(1)$ for all three operations.

But hashmaps are notoriously slow. If I want an array, I don't want random access all around my too-large-to-fit-into-cache data.

The sentinel trick provides `clear` for arrays.


### How?

The main idea is that in addition to the actual data, we store some per-element metadata and a *sentinel* that guards some of the data, switching it off conditionally. In this case, we store per-element "timestamps":

```rust
struct ArrayWithGlobalAssignment<T, const N: usize> {
    local: [(T, usize); N],
    global: T,
    sentinel: usize,
}
```

...and the writes to `local` are only "enabled" if the timestamp exactly matches the sentinel. So per-element writes store the current sentinel to `local`, and a *global* write increments the sentinel to disable the local writes.

```rust
impl<T: Default, const N: usize> ArrayWithGlobalAssignment<T, N> {
    fn new() -> Self {
        Self {
            local: core::array::from_fn(|_| Default::default()),
            global: T::default(),
            sentinel: 0,
        }
    }

    fn set(&mut self, index: usize, value: T) {
        self.local[index] = (value, self.sentinel);
    }

    fn set_global(&mut self, value: T) {
        self.global = value;
        self.sentinel += 1;
    }
}

impl<T, const N: usize> Index<usize> for ArrayWithGlobalAssignment<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        let (ref value, sentinel) = self.local[index];
        if sentinel == self.sentinel {
            value
        } else {
            &self.global
        }
    }
}
```

[Link to playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=45e793bd84b63f1d4e86b8b57840a55d).


### Persistency

Another variation of this trick enables the following operations:

- Read element by index in $O(1)$,
- Write element by index in $O(1)$,
- Get a token to the current version of the array in $O(1)$,
- Read element by index from the version specified by a token in $O(\log v)$.

In other words, it adds partial persistence to any data structure with $O(\log v)$ time overhead per non-recent access, where $v$ is the count of versions.

In this case, we store a per-element list of all the historical writes to the corresponding element. The sentinel is incremented at each write, and the value of the sentinel is the version token. A sentinel *switches off* the writes with timestamp above the sentinel.

```rust
struct PersistentArray<T, const N: usize> {
    data: [Vec<(T, usize)>; N],
    sentinel: usize,
}

impl<T: Default, const N: usize> PersistentArray<T, N> {
    fn new() -> Self {
        Self {
            data: core::array::from_fn(|_| vec![Default::default()]),
            sentinel: 0,
        }
    }

    fn set(&mut self, index: usize, value: T) {
        self.sentinel += 1;
        self.data[index].push((value, self.sentinel));
    }

    fn save(&self) -> usize {
        self.sentinel
    }

    fn get_at_version(&self, token: usize, index: usize) -> &T {
        let i = self.data[index].partition_point(|version| version.1 <= token);
        &self.data[index][i - 1].0
    }
}

impl<T, const N: usize> Index<usize> for PersistentArray<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        &self.data[index].last().unwrap().0
    }
}
```

[Link to playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=464d75f57967ba5cecec23142796de95).
