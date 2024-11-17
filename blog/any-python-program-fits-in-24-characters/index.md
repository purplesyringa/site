---
title: Any Python program fits in 24 characters*
time: November 17, 2024
intro: |
    *\* If you don't take whitespace into account.*

    My friend challenged me to find the shortest solution to a certain Leetcode-style problem in Python. They were generous enough to let me use whitespace for free, so that the code stays readable. So that's exactly what we'll abuse to encode *any* Python program in $24$ bytes, ignoring whitespace.
---

*\* If you don't take whitespace into account.*

My friend challenged me to find the shortest solution to a certain Leetcode-style problem in Python. They were generous enough to let me use whitespace for free, so that the code stays readable. So that's exactly what we'll abuse to encode *any* Python program in $24$ bytes, ignoring whitespace.

> This post originally stated that $30$ characters are always enough. Since then, [commandz](https://github.com/commandblockguy) and another person from the codegolf Discord server have devised a better solution, reaching $24$ bytes. After a few minor modifications, it satisfies the requirements of this problem, so I publish it here too.


### Bits

We can encode arbitrary data in a string by only using whitespace. For example, we could encode `0` bits as spaces and `1` bits as tabs. Now you just have to decode this.

As you start implementing the decoder, it immediately becomes clear that this approach requires about 50 characters at minimum. You can use `c % 2 for c in b"..."` to extract individual bits, then you need to merge bits by using `str` and concatenating then with `"".join(...)`, then you to parse the bits with `int.to_bytes(...)`, and finally call `exec`. We need to find another solution.


### Characters

What if we didn't go from characters to bits and then back? What if instead, we mapped each whitespace character to its own non-whitespace character and then evaluated that?

```python
exec(
    "[whitespace...]"
        .replace(" ", "A")
        .replace("\t", "B")
        .replace("\v", "C")
        .replace("\f", "D")
        ...
)
```

Unicode has quite a lot of whitespace characters, so this should be possible, in theory. Unfortunately, this takes even more bytes in practice. Under 50 characters, we can fit just two `replace` calls:

```python
exec("[whitespace...]".replace(" ","A").replace("\t","B"))
```

But we don't have to use `replace`! The less-known `str.translate` method can perform multiple single-character replaces at once:

```python
>>> "Hello, world!".translate({ord("H"): "h", ord("!"): "."})
'hello, world.'
```

The following fits in 50 characters:

```python
exec("[whitespace...]".translate({9: "A", 11: "B", 12: "C", 28: "D"})
```

4 characters isn't much to work with, but here's some good news: `translate` takes anything indexable with integers (code points). We can thus replace the dict with a string:

```python
exec(
    "[whitespace...]".translate(
        "         A BC               DEFGH                                                                                                    I                          J"
    )
)
```

The characters `ABCDEFGHIJ` are located at indices $9, 11, 12, 28, 29, 30, 31, 32, 133, 160$ -- all whitespace code points below $256$ except CR and LF, which are invalid in a string. While this code is long, most of it is just whitespace, which we ignore. After removing whitespace, it's only $32$ characters:

```python
exec("".translate("ABCDEFGHIJ"))
```

We can now encode any Python program that uses at most $10$ different characters. We could now use [PyFuck](https://github.com/kuangkzh/PyFuck), which transforms any Python script to an equivalent script that uses only $8$ characters: `exc('%0)`. This reduces the code size to $30$ charaters (plus whitespace). A bit of postprocessing is necessary to get it working well, as PyFuck often has exponential output, but that's a minor issue.


### A better way


But it turns out there's another way to translate whitespace to non-whitespace.

> This solution was found by a reader of my blog -- thanks!

When `repr` is applied to Unicode strings, it replaces the Unicode codepoints with their `\uXXXX` representations. For example, `U+2001 Em Quad` is encoded as `'\u2001'`. All in all, Unicode whitespace gives us unlimited supply of `\`, `x`, and the whole hexadecimal alphabet (plus two instances of `'`).

Say we wanted to extract the least significant digits of characters from `U+2000` to `U+2007`. Here's how to do this:

```python
# Imagine these \uXXXX escapes are literal whitespace characters
>>> repr("\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007")[6::6]
'01234567'
```

To get `\`, `x`, and the rest of the hexadecimal alphabet, we need characters like `U+000B` and `U+001F`. We also need to align the strings exactly, so that one of the columns contains all the alphabet:

```python
         v
\: "     \t "
x: "    \x0b"
0: "\u2000  "
1: "\u2001  "
2: "\u2002  "
3: "\u2003  "
4: "\u2004  "
5: "\u2005  "
6: "\u2006  "
7: "\u2007  "
8: "\u2008  "
9: "\u2009  "
a: "\u200a  "
b: "  \x0b  "
c: "  \x0c  "
d: "  \x1d  "
e: "  \x1e  "
f: "  \x1f  "
         ^
```

This requires us to increase the step to $8$, but it works!

Now, if we have free access to `\`, `x`, and the hexadecimal alphabet, we can reduce any program to just $4$ characters outside this alphabet (we're lucky that `exec` is free):

```python
# print("Hello, world!")
exec('\x70\x72\x69\x6e\x74\x28\x22\x48\x65\x6c\x6c\x6f\x2c\x20\x77\x6f\x72\x6c\x64\x21\x22\x29')
```

Now we can encode this using the previous trick, leaving `('')` as-is, and run it:

```python
exec(repr("[encoding of exec]([padding]'[user code]'[padding])")[6::8])
```


### The end

So that's how you print *Lorem Ipsum* in only $24$ characters and just $10$ KiB of whitespace. [Check out the repo on GitHub.](https://github.com/purplesyringa/24-characters-of-python)

Hope you found this entertaining! If anyone knows how to bring this to $23$ characters or less, I'm all ears. :)
