---
title: Sorted rank coding
time: July 13, 2026
intro: |
    I need a place to describe this algorithm, since it seems to be undocumented on the 'net, so here it is.

    SRC is a replacement for [MTF](https://en.wikipedia.org/wiki/Move-to-front_transform) in [BWT](https://en.wikipedia.org/wiki/Burrows%E2%80%93Wheeler_transform)-based compressors. [According to OpenBWT](https://encode.su/threads/104-libBWT?p=8617&viewfull=1#post8617), it's the best known MTF replacement measured by compression ratio, if you don't want to use context modeling.
---

I need a place to describe this algorithm, since it seems to be undocumented on the 'net, so here it is.

SRC is a replacement for [MTF](https://en.wikipedia.org/wiki/Move-to-front_transform) in [BWT](https://en.wikipedia.org/wiki/Burrows%E2%80%93Wheeler_transform)-based compressors. [According to OpenBWT](https://encode.su/threads/104-libBWT?p=8617&viewfull=1#post8617), it's the best known MTF replacement measured by compression ratio, if you don't want to use context modeling.


### MTF

Let's start with MTF and then cover the differences introduced by SRC.

MTF maintains an LRU cache of symbols. To encode a symbol `c`, the encoder emits the index of `c` in the cache ("rank") and then moves it to front. To decode a symbol, the decoder reads the index, outputs the symbol at that index in the cache, and similarly moves the symbol to front. The encoder and the decoder agree on the original order in the cache implicitly.

Mathematically, the MTF rank is equal to the number of distinct symbols between the current symbol and the previous occurrence of that symbol.


### QLFC

Next we have [QLFC](https://ieeexplore.ieee.org/document/1402216). It's exactly like MTF, but the encoder additionally communicates the final state of the cache to the decoder. To decode the string, we iteratively invert the actions performed by the encoder: we scan backwards and, on each step, move the character from the front of the cache to position `rank` to cancel out the MTF action that the encoder performs.

We want this because during decoding, we know which symbol we are moving **before** decoding its rank (namely, it's the first symbol in cache). We can use that symbol as context when decoding the rank, and that produces better results than order-1 MTF, because distances between occurrences of a specific symbol are highly correlated.

In reality, we encode `qlfc(s) = rev(mtf(rev(s)))` instead of `mtf(s)`, so that the decoder iterates and emits symbols forwards. Mathematically, the QLFC rank is the number of distinct symbols between the current symbol and the **next** occurrence of that symbol.

QLFC requires additional metadata compared to MTF because we need to send the final state. But we no longer need to agree on the initial state, in fact we can encode side data with an arbitrary permutation that way. Maybe useful for metadata?


### SRC

QLFC requires tracking multiple contexts explicitly, which is not always possible (and you might not have memory for that, or maybe your adaptive entropy coder adjusts too slowly). SRC is to QLFC as BWT is to PPM -- it invertibly reorders ranks such that ranks with the same context are locationally close.

The SRC encoder runs QLFC, but maintains separate ranks lists for each symbol. The lists are then emitted in alphabetic order to a simple entropy coder (e.g. concatenated and paired with a histogram to communicate where each sublist starts).

The SRC decoder completely unpacks the rank lists with the entropy coder, and then runs the usual QLFC decoder, but instead of using the known symbol as a context, it just reads the rank from the corresponding list. In effect, we maintain $\lvert \Sigma \rvert$ pointers initialized to the sublists and parse ranks as `rank = *ps[symbol]++`. 

Since the ranks of each symbol are grouped together, the entropy coder should have a better time inferring local frequencies.

The SRC decoder can be merged with the BWT decoder. SRC iterates in order, and the address `ps[symbol]` at each moment is exactly the total number of characters less than `symbol`, plus the index of the current symbol among equal symbols. But that's exactly the formula for $LF$ in BWT, so you can just inline the first half of the BWT decoder into SRC. This makes BWT+SRC almost as fast as BWT+MTF to decode.


### Notes

I'm not sure about the origins of SRC. It seems to have [appeared in OpenBWT first](https://encode.su/threads/104-libBWT?p=2174&viewfull=1#post2174), so it's probably by Yuta Mori, but I haven't confirmed that.
