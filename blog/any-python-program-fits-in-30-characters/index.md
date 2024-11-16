---
title: Any Python program fits in 30 characters*
time: November 17, 2024
intro: |
    *\* If you don't take whitespace into account.*

    My friend challenged me to find the shortest solution to a certain Leetcode-style problem in Python. They were generous enough to let me use whitespace for free, so that the code stays readable. So that's exactly what we'll abuse to encode *any* Python program in $30$ bytes, ignoring whitespace.
---

*\* If you don't take whitespace into account.*

My friend challenged me to find the shortest solution to a certain Leetcode-style problem in Python. They were generous enough to let me use whitespace for free, so that the code stays readable. So that's exactly what we'll abuse to encode *any* Python program in $30$ bytes, ignoring whitespace.


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


### Alphabet

We can now encode any Python program that uses at most $10$ different characters.

This would be more than enough for JavaScript: [JSFuck](https://jsfuck.com/) can transform any JS program to an equivalent JS program that only uses characters from the 6-character set `[]()!+`. Does anything like this exist for Python?

Actually, it does! [PyFuck](https://github.com/kuangkzh/PyFuck) transforms any Python script to an equivalent script that uses only 8 characters: `exc('%0)`. This means that we can reduce our $10$-byte alphabet to just $8$ bytes, further reducing the code size:

```python
exec("[whitespace...]".translate("         e xc               ('%0)"))
```

That's $30$ characters (plus whitespace).


### Optimization

There's just one problem: the output of PyFuck is *exponential* in the count of non-`exc(0)` characters in the input code. So to encode realistic programs with just `exc('%0)`, we need to pass code through *a nested encoder* before passing it to PyFuck. The optimized nested code looks like this:

```python
exec(int("[bits of code]".replace("(","0").replace(")","1"),2).to_bytes([length of code]))
```

We store bits as `(` and `)`, so there's only a fixed cost due to PyFuck (about $400$ KiB). The bits of code take $8 \times$ more space than the original bytes, but that's nothing compared to the PyFuck overhead.


### The end

So that's how you print *Lorem Ipsum* in only $30$ characters and just $420$ KiB of whitespace (still smaller than Electron). [Check out the repo on GitHub.](https://github.com/purplesyringa/30-characters-of-python)

Hope you found this entertaining! If anyone knows how to bring this to $29$ characters or less, I'm all ears. :)
