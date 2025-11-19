---
title: ntoh*/hton* is a bad API
time: November 18, 2025
intro: |
    Say you want to send a list of consumer records to another microservice over network via JSON. There are three concepts at play in this process:

    1. A logical value, which is how we humans treat the data. In this example, this would be "a list of consumer records". This description does not specify how it's represented in the computer, or whether you're using a computer at all.

    2. A data type, e.g. `std::vector<ConsumerRecord>`. The purpose of a data type is two-fold: a) it denotes a specific runtime representation of the logical value in computer memory, b) it provides an abstraction so that you can work with the logical value without thinking about this implementation detail.

    3. A serialization format, here JSON. It denotes an alternative, byte sequence-oriented representation of the logical value, but does not provide an abstraction layer, since you cannot directly work with the records encoded in the JSON string.
---

Say you want to send a list of consumer records to another microservice over network via JSON. There are three concepts at play in this process:

1. A logical value, which is how we humans treat the data. In this example, this would be "a list of consumer records". This description does not specify how it's represented in the computer, or whether you're using a computer at all.

2. A data type, e.g. `std::vector<ConsumerRecord>`. The purpose of a data type is two-fold: a) it denotes a specific runtime representation of the logical value in computer memory, b) it provides an abstraction so that you can work with the logical value without thinking about this implementation detail.

3. A serialization format, here JSON. It denotes an alternative, byte sequence-oriented representation of the logical value, but does not provide an abstraction layer, since you cannot directly work with the records encoded in the JSON string.

These concepts are mostly orthogonal. You can switch from `vector` to a linked list, or even change the programming language, affecting only the data type. You can also talk to a different microservice via XML without changing the way your application handles data.

Typically, libraries provide (de)serialization functions to convert between data types and serialized data, with an API along the lines of:

```c
char* serialize(struct object obj); // returns malloc'ed buffer
struct object deserialize(const char* bytes);
```

---

Now, let's apply the same idea to integers transferred via a binary protocol. We have:

1. A logical value, "integer". This corresponds to the concept of a number like $13$ -- not necessarily linked to computing or even the decimal numeral system. 12-fingered aliens would be aware of the concept of $13$, even though they might communicate it differently, say with $aa$.

2. A data type used for manipulation, e.g. `int` or `uint32_t`. You can do arithmetic on integers without concerning yourself with how the CPU handles it. For all we know, the integer might as well be encrypted with AES -- as long as the abstraction agrees that `2 + 2 == 4`, any implementation is fine.

3. A serialization format that describes how the integer is encoded into a sequence of bytes. The most straightforward approach is to split the number into 8-bit parts and write them in sequence in some order. The most common orders are called "little-endian" and "big-endian".

Here's what an API for (de)serializing integers might look like:

```c
char[4] serialize(uint32_t num);
uint32_t deserialize(char bytes[4]);
```

---

But the C standard library does not provide such an API. No, it exposes something much more sinister:

```c
uint32_t htonl(uint32_t hostlong);
uint32_t ntohl(uint32_t netlong);
```

`htonl` tries to be `serialize`, and `ntohl` tries to be `deserialize` -- emphasis on "tries".

`uint32_t` is supposed to be an abstraction, a computer implementation of the concept of "integer". So why is the serialized data, logically a sequence of bytes, also considered an integer? This makes no sense, given that we're trying to reduce the complexity of data. And it's not even an "integer" in the sense of a data type, since operating on the values returned by `htonl` (say, by adding them together) produces gibberish.

If a socket can only handle byte streams, and we couldn't send `std::vector<ConsumerRecord>` until it was converted to a byte sequence, it doesn't make much sense to directly send `uint32_t` like `htonl` wants us to. It's a category error. Really, the only reason this works is that the runtime representation of an integer -- i.e. the byte layout of the `uint32_t` data type -- is *remotely similar* to the intended serialization format. And by "remotely similar", I mean "has identical length and valid bit patterns". `htonl` is an ugly hack: it patches the runtime representation such that its byte sequence matches the intended output, without consideration for the meaning of the new value represented by the data type.

Can you imagine doing anything like that with any other type? Reordering bytes of `std::vector` would be madness and lead to UB galore. A hypothetical function like `bool htonb(bool hostbool)`, where the "host bool" is represented by bytes `0` or `1`, and the "network bool" is represented by `0` or `0xFF`, could not be implemented with certain ABIs, let alone without causing UB. And in many cases, the runtime and the serialized representations aren't even guaranteed to have equal lengths.

---

Really, the only reason why `htonl` doesn't return `char[4]` and `ntohl` doesn't take `char[4]` is that C doesn't support arrays in this position. This is exclusively a language deficiency. A better language would never expose functions with such signatures, and indeed, Go, Python, and Java all get this right.

But my point is not to bash C.

My point is that this API quirk fundamentally changes how people think about endianness, making them commit mistakes they wouldn't make otherwise. Many people think [integers have an intrinsic endianness](https://stackoverflow.com/questions/105252/how-do-i-convert-between-big-endian-and-little-endian-values-in-c), or prefer textual formats to seemingly avoid dealing with endianness, and I have held all these convictions myself at some point.

It's not that people are stupid. It's borderline impossible to make the same mistakes if you treat endianness as a parameter of a serialization format. But C doesn't want you to think about it this way, and so you never realize you were lied to until you get a reason to think hard about it. `man` pages and tutorials on the internet concurring that `ntohl` "converts a number from network order to host byte order" only make this worse.

How do I know this is the case? Beyond personal experience, it turns out that Rust offers methods like [`u32::to_le`](https://doc.rust-lang.org/stable/std/primitive.u32.html#method.to_le) with this broken signature. Granted, it also supports the sane version [`u32::to_le_bytes`](https://doc.rust-lang.org/stable/std/primitive.u32.html#method.to_le_bytes), but it was added much later. `to_le` was never deprecated, and is still documented straightforwardly without a hint to prefer `to_le_bytes` instead. I can only interpret this as a historical mistake that is so ingrained in developers' brains that it's not immediately obvious.

My plea is to educators: please teach endianness differently. Introduce it as a parameter of a specific serialization format. Highlight native endianness as an implementation detail that doesn't matter as long as you're only using abstractions like `int`. Explain that numbers don't have endianness. Elaborate that, much like JSON can be parsed by an architecture-agnostic algorithm and then becomes irrelevant, both little-endian and big-endian (de)serialization can be performed without knowing the native endianness or exposing the endianness of the original data. Recommend type-safe APIs if possible. Highlight `ntoh*`/`hton*` as a misdesigned non-type-safe API and tell people they're going to be met with misinformation online.
