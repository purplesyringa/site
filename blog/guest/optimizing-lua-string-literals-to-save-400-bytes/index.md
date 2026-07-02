---
title: Optimizing Lua string literals to save 400 bytes
time: July 2, 2026
intro: |
  This is a guest post by [Yuki](https://codeberg.org/sylfn/) about some tricks we use for Lua code compression in [our shared ComputerCraft pet project](https://github.com/purplesyringa/computercraft-programs/). I've written about [how we adapted bzip2 for this purpose](/blog/an-ode-to-bzip/) earlier; this story is an installment that takes place in the same context.

  If you want to read more stuff from her, [here's an Atom feed](https://codeberg.org/sylfn/weblog/raw/branch/main/atom.xml) and [the blog](https://codeberg.org/sylfn/weblog).
upstream: https://codeberg.org/sylfn/weblog/src/branch/main/posts/2026-07-02.md
author: sylfn@disroot.org (Yuki Sireneva)
---

This is a guest post by [Yuki](https://codeberg.org/sylfn/) about some tricks we use for Lua code compression in [our shared ComputerCraft pet project](https://github.com/purplesyringa/computercraft-programs/). I've written about [how we adapted bzip2 for this purpose](/blog/an-ode-to-bzip/) earlier; this story is an installment that takes place in the same context.

If you want to read more stuff from her, [here's an Atom feed](https://codeberg.org/sylfn/weblog/raw/branch/main/atom.xml) and [the blog](https://codeberg.org/sylfn/weblog).

### Context

<aside-start-here />

Me and my girlfriend have created [a bunch of cool programs for CC: Tweaked][cc-p].
Programs for CC:T are written in Lua, and in-game computers have limited disk space.
The unadjusted limits are 1 MB of disk space per computer (or turtle)
and 125 KB of disk space on a diskette.

:::aside
I use SI prefixes for bytes. You should too!
:::

Our programs exceed that. By a lot. At the time of writing,
a concatenation of all the lua code in the repo alongside its documentation
is slightly larger than 440 KB, which does not fit in a single floppy and is simply too large.

[cc-p]: https://github.com/purplesyringa/computercraft-programs

We have a solution to the size problem, however, and it's compression.
Our data is Lua code and text, mostly code, though,
and there are [compression algorithms][bzip] that are simply the Best
(in terms of output size and decompression speed).

[bzip]: https://purplesyringa.moe/blog/an-ode-to-bzip/

One of our goals was to create a self-decompressing archive in the form of a single file,
so that the installation is as simple as typing

```shell
wget https://cc.purplesyringa.moe/initrd.lua startup.lua
```

into the shell.

### Substituting?

This feat requires merging the data with the decompressor.
The stupidest way to do this is to serialize data as Lua literals, but it has some issues.
It's trivially simple to serialize `nil`, `true`, `false`, numbers, and lists.
The hard part is tables and strings.

Tables have two main key representations, identifier-based and expression-based.
These tables are equivalent:

```lua
{ key = 10 }
{ ["key"] = 10 }
```

The first one uses an identifier-based key,
which can only be a valid identifier and is then converted to a string,
and the second one is a catch-all variant.
I have not benchmarked if using expression-based keys everywhere is worth it,
but I feel like it's not.

The final boss of substitution is strings.

How hard can it be?

### Strings are hard

There is a meme about dozens of ways to create a string in YAML
and how it's confusing to everyone not in the know.
This situation is a bit like this.

Lua strings (particularly the ones in Cobalt, a Lua runtime used in CC) are byte strings,
like `b"meow"` in Python but worse, or `b"meow"` in Rust but better.
There are three main ways to write a string:

<aside-start-here />

1. Single-quoted, on a single line: `'meow'`. Backslash-escapes work
2. Double-quoted, on a single line: `"meow"`. Backslash-escapes work
3. Raw strings, can be multiline: `[[meow]]` and `[=[this [[is]] a string]=]`. Backslashes are treated as normal characters

:::aside
As a sidenote, we don't bother with escaping characters like converting literal LFs to `\n`.
This probably has only negative effects in the long run and creates less-readable output.
:::

Quoted strings are cool.
If the string does not contain any line breaks or special characters,
then the quoted representation is simply the best.
It is probably how you'd write most of your normal-looking strings.
Raw strings, however...

### Raw strings

Raw strings
are started by `[=[` (any number of `=`s)
and terminated by `]=]` (same number of `=`s)
and must contain neither their starter nor their terminator.
This makes `[=[meow [=[ mrrp]=]` an invalid string, but `[[meow [=[ mrrp]]` is valid.

One might think "yeah let's find the longest matches of `\[=+\[|\]=+\]` regex and one-up them,
but that breaks on `[=[==[`.

One might think "yeah let's filter matches of `[\[\]]=*` by next charactex and then one-up them,
and that would work... mostly.

Except that there's nuance.

### Nuance???

Raw strings inside table keys are special and need a space, like C++ templates prior to C++11:

```lua
{ [ [[meow]]] = 10 }
```

This is the easiest of nuances to deal with, but there's more.

What is a newline character? Some might even say it's a sequence.
Cobalt folds newline sequences (CRLF, LFCR, single CR, single LF) into single LF in raw strings,
which is like running `dos2unix` on your PNG and expecting it not to be corrupted afterwards.

And Cobalt does one more thing.
If the string starts with a newline, that newline is dropped.
Only the first newline is affected, so both of these are true:

```lua
print("meow" == [[
meow]])
print("\nmeow" == [[

meow]])
```

### Dealing with CR

So we are gonna need escaping after all.
But raw strings are called raw because they don't interpret backslash in any special way!
How are we gonna escape?

We have a tool for that and it's called [`gsub`].

If there is a character that is not used in the string
(something along the lines of `\0` for text files),
then we can replace all CRs with this character in the code
and use gsub afterwards, like so:

```lua
print("\r\r\r" == ([[RRR]]):gsub('R','\r'))
```

[`gsub`]: https://www.lua.org/manual/5.2/manual.html#pdf-string.gsub

Note that because we use that character in normal strings and as an argument to `gsub`,
some characters are banned from being an "escape" character, namely
- CR itself
- `\`, normal escape character
- any of `^$()%.[]*+-?`, which are special pattern characters
- single quote (we use it as a string boundary for more readable code on the Rust side)

The simple cases are running out, and we now have a byte sequence that has all kinds of bytes in it,
even the CR, and all of these need to be preserved.

One way of dealing with it is by replacing both the CR and our escape character (e.g. `E`)
with something else, like:
- `Er` for CR
- `E]` for escape character itself

This solution works, but if the string has a nearly-uniform byte distribution,
the size of the string is increased by about 1%.

There is another way.
We can store "is it actually a CR" information out-of-band and then apply it with `gsub`:

```lua
([[string]]):gsub('E', (function(i)
    return function()
        i = i + 1
        return bit32.btest(
            ('V'):byte(math.floor(i/7)),
            2^(i%7)
        ) and '\r'
    end
end)(6))
```

where `E` is escape character and `V` is a bitstring.

These hundred bytes (unminified for readability) are packed with tricks:
- Only lower 7 bits of `V` are accessed
  so that we can always set the highest bit
  and avoid bumping into special characters.
- `and '\r'` is used to replace escapes with CRs,
  because if `gsub` callback returns `nil` or `false`,
  `gsub` does not perform a substitution.
- `function(i)` creates a local variable that gets initialized to `6` at the call site.
- `i = 6` is the starting state to compensate for 1-based indexes in Lua.

This almost halves the CR-overhead,
as CRs/escapes are now encoded with 9 bits and change instead of 16 bits.

There is probably a way to simplify this even further, tell me if you know.

### Bonus: Rust POV

There is one trick left:

```rust
core::iter::repeat(0..7).flatten()
```

creates an iterator that yields `0, 1, ..., 7, 0, 1, ..., 7, 0, 1, ...`,
the indices of the bits we need to populate. This feels cleaner than using `% 7` and `.enumerate()`.

Full string serialization code can be accessed [here][cc-p/ser/iter].

[cc-p/ser/iter]: https://github.com/purplesyringa/computercraft-programs/blob/df3551a486e239f7761312ba8989560229ddd241/initrd-ng/initrd-core/src/ser.rs#L102
