---
title: An ode to bzip
time: March 12, 2026
intro: |
    The story goes like this. [ComputerCraft](https://tweaked.cc) is a mod that adds programming to Minecraft. You write Lua code that gets executed by [a bespoke interpreter](https://github.com/cc-tweaked/Cobalt) with access to world APIs, and now you're writing code instead of having fun. Computers have limited disk space, and my [`/nix` folder](https://github.com/purplesyringa/computercraft-programs/) is growing out of control, so I need to compress code.

    The laziest option would be to use [LibDeflate](https://github.com/SafeteeWoW/LibDeflate/), but its decoder is larger than both the gains from compression and my personal boundary for copying code. So the question becomes: what's the shortest, simplest, most ratio-efficient compression algorithm?

    I initially thought this was a complex question full of tradeoffs, but it turns out it's very clear-cut. My answer is `bzip`, even though this algorithm [has been critiqued](https://changelog.complete.org/archives/910-how-to-think-about-compression) [multiple times](https://news.ycombinator.com/item?id=10721338) and has fallen into obscurity since xz and zstd became popular.
---

The story goes like this. [ComputerCraft](https://tweaked.cc) is a mod that adds programming to Minecraft. You write Lua code that gets executed by [a bespoke interpreter](https://github.com/cc-tweaked/Cobalt) with access to world APIs, and now you're writing code instead of having fun. Computers have limited disk space, and my [`/nix` folder](https://github.com/purplesyringa/computercraft-programs/) is growing out of control, so I need to compress code.

The laziest option would be to use [LibDeflate](https://github.com/SafeteeWoW/LibDeflate/), but its decoder is larger than both the gains from compression and my personal boundary for copying code. So the question becomes: what's the shortest, simplest, most ratio-efficient compression algorithm?

I initially thought this was a complex question full of tradeoffs, but it turns out it's very clear-cut. My answer is `bzip`, even though this algorithm [has been critiqued](https://changelog.complete.org/archives/910-how-to-think-about-compression) [multiple times](https://news.ycombinator.com/item?id=10721338) and has fallen into obscurity since xz and zstd became popular.


### First look

I'm compressing a 327 KB file that contains Lua code with occasional English text sprinkled in comments and documentation. This is important: bzip excels at text-like data rather than binary data. However, my results should be reproducible on other codebases, as the percentages seem to be mostly constant within that category.

Let's compare multiple well-known encoders on this data:

- uncompressed: 327005
- (gzip) `zopfli --i100`: 75882
- `zstd -22 --long --ultra`: 69018
- `xz -9`: 67940
- `brotli -Z`: 67859 (recompiled without a dictionary)
- `lzip -9`: 67651
- `bzip2 -9`: 63727
- `bzip3`: 61067

The `bzip` family is a clear winner by a large margin. It even beats `lzip`, whose docs say "'lzip -9' compresses most files more than bzip2" (I guess code is not "most files"). How does it achieve this? Well, it turns out that `bzip` is *not like the others*.


### Algorithms

You see, all other popular compression algorithms are actually the same thing at the core. They're all based on [LZ77](https://en.wikipedia.org/wiki/LZ77_and_LZ78#LZ77), a compression scheme that boils down to replacing repetitive text with short links to earlier occurrences.

The main difference is in how literal strings and backreferences are encoded as bit streams, and this is highly non-trivial. Since links can have wildly different offsets, lengths, and [frequencies](https://en.wikipedia.org/wiki/Letter_frequency) from location to location, a good algorithm needs to predict and succinctly encode these parameters.

But `bzip` does not use LZ77. `bzip` uses [BWT](https://en.wikipedia.org/wiki/Burrows%E2%80%93Wheeler_transform), which reorders characters in the text to group them by context -- so instead of predicting tokens based on similar earlier occurrences, you just need to look at the last few symbols. And, surprisingly, with the BWT order, you don't even need to store where each symbol came from!

For example, if the word `hello` is repeated in text multiple times, with LZ77 you'll need to find and insert new references at each occurrence. But with BWT, all continuations of `hell` are grouped together, so you'll likely just have a sequence of many `o`s in a row, and similarly with other characters, which simple [run-length encoding](https://en.wikipedia.org/wiki/Run-length_encoding) can deal with.

BWT comes with *some* downsides. For example, if you concatenate two texts in different English dialects, e.g. using `color` vs `colour`, BWT will mix the continuations of `colo` in an unpredictable order and you'll have to encode a weird sequence of `r`s and `u`s, whereas LZ77 would prioritize recent history. You can remedy this by separating input by formats, but for consistent data like code, it works just fine as is.

`bzip2` and `bzip3` are both based on BWT and differ mostly in how the BWT output is compressed. `bzip2` uses a variation on RLE, while `bzip3` tries to be more intelligent. I'll focus on `bzip2` for performance reasons, but most conclusions apply to `bzip3`, too.


### Heuristics

There is another interesting thing about BWT. You might have noticed that I'm invoking `bzip3` without passing any parameters like `-9`. That's because `bzip3` doesn't take them. In fact, even invoking `bzip2` with `-9` doesn't do much.

LZ77-based methods support different compression levels because searching for earlier occurrences is time-consuming, and sometimes it's preferable to use a literal string instead of a difficult-to-encode reference, so there is some brute-force. BWT, on the other hand, is entirely deterministic and free of heuristics.

Furthermore, there is no degree of freedom in determining how to efficiently encode the lengths and offsets of backreferences, since there are none. There are run lengths, but that's about it -- it's a single number, and it's smaller than typical offsets.

All of that is to say: if you know what the `bzip2` pipeline looks like, you can quickly achieve similar compression ratios without fine-tuning and worrying about edge cases. [My unoptimized ad-hoc `bzip2`-like encoder](https://github.com/purplesyringa/computercraft-programs/tree/master/initrd) compresses the same input to about 67 KB -- better than `lzip` and with clear avenues for improvement.


### Decoders

That covers the compression format, but what about the size of the decoder? Measuring ELFs is useless when targeting Lua, and Lua libraries like [LibDeflate](https://github.com/SafeteeWoW/LibDeflate) don't optimize code size for self-extracting archives, so at risk of alienating readers with fancy words and girl math, I'll have to eyeball this for everything but `bzip2`.

A self-extracting executable doesn't have to decode every archive -- just one. We can skip sanity checks, headers, inline metadata into code, and tune the format for easier decoding. As such, I will only look at the core decompression loops.

`gzip`, `zstd`, `xz`, `brotli`, and `lzip` all start by doing LZ77. Evaluating "copy" tokens is a simple loop that won't take much code. Where they differ is in how those tokens are encoded into bits:

:::aside
Here's an example of a Huffman code. Suppose there are 5 tokens with different frequencies: A (60%), B (20%), C (10%), D (5%), E (5%). Write `A = 0`, `B = 10`, `C = 110`, `D = 1110`, `E = 1111`. The more frequent a token is, the shorter its encoding. To decode a bit stream, pull bits one by one until you find an exact match.
:::

- `gzip` does some light pre-processing and then applies [Huffman coding](https://en.wikipedia.org/wiki/Huffman_coding), which assigns unambiguous bit sequences to tokens and then concatenates them, optimizing for total length based on the token frequency distribution. Huffman codes can be parsed in ~250 bytes, the bit [trie](https://en.wikipedia.org/wiki/Trie) might take ~700 bytes, and the glue should fit in ~500 bytes. Let's say 1.5 KB in total.

<aside-inline-here />

- `xz` encodes tokens bit-by-bit instead of treating them as atoms, which allows the coder to [adjust probabilities dynamically](https://en.wikipedia.org/wiki/Context-adaptive_binary_arithmetic_coding), yielding good ratios without encoding any tables at the cost of performance. Bit-by-bit parsing will take more space than usual, but avoiding tables is a huge win, so let's put at 1 KB.

- `lzip` is very similar to `xz`, only slightly changing token encodings, so let's put it at 1 KB as well.

:::aside
If you know what [arithmetic coding](https://en.wikipedia.org/wiki/Arithmetic_coding) is, FSE is like that, but for large alphabets.
:::

- `zstd` complicates the pre-processing step and uses [Finite State Entropy](http://fastcompression.blogspot.com/2013/12/finite-state-entropy-new-breed-of.html) instead of Huffman coding, which effectively allows tokens to be encoded with fractional bit lengths. FSE is simple, but requires large tables, so let's say ~2000 bytes for storing and parsing them. Adding glue, we should get about 3 KB.

<aside-inline-here />

:::aside
On the web, `brotli` often wins due to a large pre-shared dictionary. It raises the size of the decoder, so in our setup, it's a hindrance, and I'm not taking it into consideration.
:::

- `brotli` keeps Huffman coding, but switches between multiple static Huffman tables on the flight depending on context. I couldn't find the exact count, but I get 7 tables on my input. That's a lot of data that we can't just inline -- we'll need to encode it and parse it. Let's say ~500 bytes for parser and ~100 bytes per table. Together with the rest of the code, we should get something like 2.2 kB.

<aside-inline-here />

For `bzip` decoders, BWT can be handled in ~250 bytes. As for the unique parts,

- `bzip2` compresses the BWT output with [MTF](https://en.wikipedia.org/wiki/Move-to-front_transform) + [RLE](https://en.wikipedia.org/wiki/Run-length_encoding) + Huffman. With the default 6 Huffman tables, let's assign ~1.5 KB to all Huffman-related code and data and ~400 bytes for MTF, RLE, and glue.

- `bzip3` uses XZ-like [bit-by-bit coding](https://en.wikipedia.org/wiki/Context-adaptive_binary_arithmetic_coding) with [context mixing](https://en.wikipedia.org/wiki/Context_mixing) instead. Let's say 1 KB for the former and ~500 bytes for the latter.

Point is: by dropping compatibility with standard file formats, the decoder can become very small. I might be wrong on some of these figures, but it most likely won't switch things up significantly.

`bzip`-style methods are in the middle of the pack, but that's somewhat misleading. While `bzip2` typically uses 6 Huffman tables, I got good compression results with just one. With a single table, my `bzip`-style decoder fits in 1.5 KB, which is smaller than everything but `xz` and `lzip`, while being faster and more compact.


### Performance

It's common knowledge that `bzip` is slow, but there's nuance.

When compression is used to circumvent a hard limit, the difference between `bzip` and `gzip` is not booting slowly or quickly, but between booting and not booting at all. If unpacking [initrd](https://en.wikipedia.org/wiki/Initial_ramdisk) takes 0.5 seconds instead of 0.3 seconds, it's not a big deal compared to being unable to boot.

From this perspective, saying `bzip` is slower than `gzip` begs the question. `gzip` cannot easily produce an optimal output like `bzip` can, so it searches for earlier occurrences heuristically with a configurable time-ratio tradeoff. `gzip` is faster not because of different designs per se, but because `gzip` decided being fast is more important than a good output, even on `-9`. If you want a good ratio, you have to use [zopfli](https://github.com/google/zopfli), which tries to encode `gzip` more optimally and is a magnitude slower than `bzip` despite producing worse output.

On the decoding side, decoding `bzip` is slow because inverting BWT requires random access. This is less of an issue in high-level languages like Lua, where all operations are slow anyway. In this situation, most typical compression techniques are even more expensive. You can decode `gzip` faster than `bzip2`, but `zstd` and `brotli` are likely closer to it in speed.

I haven't tried using `bzip3` in practice, but I expect decoding `bzip3` to be significantly slower than `bzip2` due to having to parse tokens bit-by-bit. I'm assuming it's possible to get `bzip3`-like ratio without this, but I'll cross that bridge when I come to it. 


### Alternatives

So that's my answer: not a custom compression algorithm, just `bzip` with whistles.

If you know anything about me, not inventing anything new might sound surprising. Well, I tried to, with varying success.

Here's an exercise. The core idea behind text compression is that text is repetitive, so instead of repeating the word `repeat` every time it's written, we can encode (hopefully shorter) references to the earlier repetitions of `repeat`. For example, the string:

> [Rust](https://en.wikipedia.org/wiki/Rust_(video_game)) is an iron oxide, a usually reddish-[brown](https://www.youtube.com/watch?v=wh4aWZRtTwU) oxide formed by the reaction of iron and oxygen in the catalytic presence of water or air moisture.

...can be encoded as:

- Write `Rust is an iron oxide, a usually reddish-brow`
- Copy 7 bytes from 31 bytes ago (`n oxide`)
- Write ` formed by the`
- Copy 3 bytes from 24 bytes ago (` re`)
- Write `acti`
- Copy 4 bytes from 60 bytes ago (`on o`)
- Write `f`
- Copy 6 bytes from 68 bytes ago (<code> iron </code>) <!-- `` strips spaces on both sides -->
- Write `and`
- Copy 3 bytes from 41 bytes ago (` ox`)
- Write `yge`
- Copy 3 bytes from 84 bytes ago (`n i`)
- Write `n`
- Copy 5 bytes from 35 bytes ago (<code> the </code>)
- Write `catalytic presence`
- Copy 4 bytes from 45 bytes ago (<code> of </code>)
- write `water or air moisture.`

Since we're compressing text, you might expect the repeated parts to be words. But in this example, it's often syllables or random letters. Attempts to compress code based on code structure struggle with this, because they're only capable of compressing individual tokens.

And while we could pre-process the code and then apply `bzip` to reap the rest of the benefits, it would complicate the decoder without measurably improving the compression ratio. [Luz](https://github.com/MCJack123/Luz), for example, does this with `gzip` without any impact on the ratio, despite wasting many kilobytes on decoding.

This seems to be a common experience in data compression. Real-world data is structured in a way that humans can't easily guess, but computers can recognize on the fly. Improvements come not from complicating algorithms, but from restructuring data and applying more powerful general-purpose methods.

If this sounds suspiciously like machine learning, you're on point. The 2009 article [Rationale for a Large Text Compression Benchmark](https://mattmahoney.net/dc/rationale.html) describes this connection in more detail. It feels naive today, but it checks out: the top contender of the [Large Text Compression Benchmark](https://mattmahoney.net/dc/text.html) is [nncp](https://bellard.org/nncp/), and you can surely guess what "NN" stands for.


### Conclusion

`bzip` might be suboptimal as a general-purpose compression format, but it's great for text and code. One might even say the `b` in `bzip` stands for "best".

`bzip` encoders are less riddled with heuristics and word-of-mouth design decisions than LZ77-based encoders, leaving less room for subtle mistakes that greatly affect the compression ratio.

`bzip` decoding is quite fast when implemented in a high-level language.
