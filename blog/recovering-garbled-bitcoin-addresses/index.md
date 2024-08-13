---
title: Recovering garbled Bitcoin addresses
time: April 23, 2024
intro: |
    [ZeroNet](https://zeronet.io/) is a decentralized network that enables dynamic sites, such as blogs and forums, unlike popular content-addressed storage networks that came later. Sites aren't addressed by immutable hashes; instead, site updates are signed by Bitcoin addresses.

    A moot point is that Bitcoin addresses are case-sensitive, and people are used to addresses being case-insensitive. Mistakes happen, and sometimes the only trail you have is a lower-cased address, like `1lbcfr7sahtd9cgdqo3htmtkv8lk4znx71`.

    Losing valuable information is a bad thing when you're an archivist. Have we *really* lost access to the site if we only know the lower-cased address? Can we recover the original address somehow?
tikzThemes:
  light: |
    \colorlet{DeepBlue}{blue!30}
    \colorlet{ShallowBlue}{blue!15}
    \colorlet{DeepRed}{red!30}
    \colorlet{ShallowRed}{red!15}
    \colorlet{DeepLightRed}{red!20}
    \colorlet{ShallowLightRed}{red!10}
    \colorlet{DeepGold}{Gold}
    \colorlet{ShallowGold}{Gold!15}
    \colorlet{DeepMagenta}{magenta!30}
    \colorlet{ShallowMagenta}{magenta!15}
  dark: |
    \definecolor{DeepBlue}{rgb}{.2,.3,1}
    \definecolor{ShallowBlue}{rgb}{.1,.1,.5}
    \definecolor{DeepRed}{rgb}{1,.2,.2}
    \definecolor{ShallowRed}{rgb}{.5,.1,.1}
    \definecolor{DeepLightRed}{rgb}{.5,.1,.1}
    \definecolor{ShallowLightRed}{rgb}{.3,.1,.12}
    \definecolor{DeepGold}{rgb}{.6,.6,.1}
    \definecolor{ShallowGold}{rgb}{.4,.4,.1}
    \definecolor{DeepMagenta}{rgb}{.8,.2,.8}
    \definecolor{ShallowMagenta}{rgb}{.4,.1,.4}
---

There are many code snippets in this blog post. You can [download them from a repo](https://github.com/purplesyringa/site/tree/recovering-garbled-bitcoin-addresses/blog/recovering-garbled-bitcoin-addresses). The finished project is available [here](https://github.com/purplesyringa/btccaserestore/).

---

Once upon a time, there was a decentralized network called [ZeroNet](https://zeronet.io/). Unlike popular content-addressed storage networks that came later (such as IPFS), ZeroNet enabled dynamic sites that could be updated by their owners in real-time, such as blogs and forums. As a consequence, sites could not be addressed by immutable hashes. The lead developer didn't want to invent any new cryptography, though, so he made perhaps the smartest decision: sites were addressed by Bitcoin addresses, and their contents and updates were signed by that address.

:::aside
Bitcoin now supports [bech32 addresses](https://en.bitcoin.it/wiki/Bech32), which are case-insensitive, but those came later.
:::

If you haven't used Bitcoin, here's what its addresses look like: `1Lbcfr7sAHTD9CgdQo3HTMTkV8LK4ZnX71`. In comparison, a typical domain name on the web looks like this: `purplesyringa.moe`. The main difference is that Bitcoin addresses are case-sensitive, and people are used to addresses being case-insensitive. This has led to hacks like using `http://zero/1Lbcfr7sAHTD9CgdQo3HTMTkV8LK4ZnX71` instead of `http://1Lbcfr7sAHTD9CgdQo3HTMTkV8LK4ZnX71.zero`, but mistakes still happened, and sometimes the only trail you had is a lower-cased address, like `1lbcfr7sahtd9cgdqo3htmtkv8lk4znx71`. And that's how valuable information is lost.

<aside-inline-here />

I was working on archiving ZeroNet back then, so any sort of information loss due to human error was a nuisance worth fixing. Have we *really* lost access to the site if we only know the lower-cased address? Can we recover the original address somehow?


### What is an address?

:::aside
If you are interested in how to verify a signature without knowing a public key, [this](https://crypto.stackexchange.com/questions/18105/how-does-recovering-the-public-key-from-an-ecdsa-signature-work) should answer our question. Also, some people argue applying a hash here increases Bitcoin's resistance to quantum attacks.
:::

A common misconception is that a Bitcoin address is an encoding of a public key. This is not the case. Instead, an address contains an encoding of a *hash* of the public key. It is a UX decision: the hash is shorter than the public key and thus easier to use, but it is still long enough so that security is not affected.

<aside-inline-here />

It turns out that Satoshi made other UX considerations, too. A Bitcoin address encodes not only a hash of the public key but also a checksum. If someone makes a typo in the target address while transferring currency, the Bitcoin client will notice that and cancel the transaction. The protection is much more reliable than the one IBAN uses: the checksum is the first four bytes of `SHA256(SHA256(key_hash))`.

The last important part is that the encoding is not your favorite base64 but base58. The difference is that base58 excludes the characters `0`, `O`, `I`, and `l` from the encoding because they are easily confused, and `+` and `/` because they are not URI-safe (and potentially confusing), and `=` because padding is useless anyway.

Here's a neat illustration:

```tikz
\draw[step=1,DeepBlue,thick,fill=ShallowBlue] (0,0) grid (32,-1) rectangle (0,0);
\node[scale=2] at(16,-0.5) {Private key (random 32 bytes)};
\draw[-{>[length=0.25cm]},thick] (16,-1) -- node[right=0.5cm,scale=2] {ECDSA magic} (16,-4);
\draw[step=1,shift={(-0.5,0)},DeepBlue,thick,fill=ShallowBlue] (0,-4) grid (33,-5) rectangle (0,-4);
\node[scale=2] at(16,-4.5) {Public key (33 bytes)};
\draw[-{>[length=0.25cm]},thick] (16,-5) -- node[right=0.5cm,scale=2] {SHA-256} (16,-8);
\draw[step=1,DeepRed,thick,fill=ShallowRed] (0,-8) grid (32,-9) rectangle (0,-8);
\node[scale=2] at(16,-8.5) {Hash (32 bytes)};
\draw[-{>[length=0.25cm]},thick] (16,-9) -- node[right=0.5cm,scale=2] {RIPEMD-160} (16,-12);
\draw[step=1,DeepRed,thick,fill=ShallowRed] (6,-12) grid (26,-13) rectangle (6,-12);
\node[scale=2] at(16,-12.5) {Hash (20 bytes)};
\draw[DeepBlue,thick,fill=ShallowBlue] (4,-12) rectangle (5,-13);
\node[scale=2] at(4.5,-11) {Address type (1 byte)};
\node[scale=2] at(4.5,-12.5) {00};
\node[scale=2] at(5.5,-12.5) {+};
\draw[decorate,thick,decoration={brace,amplitude=1cm,mirror}] (4,-13.5) -- (26,-13.5);
\draw[DeepGold,shift={(-0.5,0)},thick,fill=ShallowGold] (5,-15) grid (26,-16) rectangle (5,-15);
\node[scale=2] at(16,-15.5) {Payload (21 bytes)};
\draw[-{>[length=0.25cm]},thick] (25.5,-15.5) -- node[above=0.5cm,scale=2] {SHA-256 $\times$ 2} (32,-15.5);
\draw[step=1,DeepRed,thick,fill=ShallowRed] (32,-15) grid (36,-16) rectangle (32,-15);
\draw[DeepLightRed,thin,fill=ShallowLightRed] (36,-15) rectangle (42,-16);
\node[scale=2] at(34,-15.5) {Checksum};
\node[scale=2] at(39,-15.5) {$\dots$};
\draw[decorate,thick,decoration={brace,amplitude=1cm,mirror}] (4.5,-16.5) -- (36,-16.5);
\draw[DeepGold,shift={(-0.5,0)},thick,fill=ShallowGold] (8,-18) grid (33,-19) rectangle (8,-18);
\node[scale=2] at(20.5,-18.5) {Decoded address (25 bytes)};
\draw[-{>[length=0.25cm]},thick] (20.5,-19) -- node[right=0.5cm,scale=2] {base58} (20.5,-22);
\draw[step=1,DeepMagenta,thick,fill=ShallowMagenta] (3,-22) grid (37,-23) rectangle (3,-22);
\node[scale=2] at(20,-22.5) {Encoded address ($\sim$ 34 characters)};
```


### First attempt

Can we brute-force our way through all combinations of lowercase/uppercase and check if any is valid? Let's try just that.

```python
import base58
import itertools


address_lowercase = "1lbcfr7sahtd9cgdqo3htmtkv8lk4znx71"


def try_both_cases(c):
    yield c
    if c.upper() != c:
        yield c.upper()

for address in itertools.product(*map(try_both_cases, address_lowercase)):
    address = "".join(address)
    try:
        base58.b58decode_check(address)
    except ValueError:
        pass
    else:
        print("Found valid address:", address)
```

```shell
$ time python3 attempt1.py
^CTraceback (most recent call last):
  File "/home/purplesyringa/btccaserestore/attempt1.py", line 16, in <module>
    base58.b58decode_check(address)
  File "/home/purplesyringa/.local/lib/python3.11/site-packages/base58/__init__.py", line 152, in b58decode_check
    result = b58decode(v, alphabet=alphabet, autofix=autofix)
             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  File "/home/purplesyringa/.local/lib/python3.11/site-packages/base58/__init__.py", line 128, in b58decode
    acc, mod = divmod(acc, 256)
               ^^^^^^^^^^^^^^^^
KeyboardInterrupt


real    1m47,770s
user    1m47,301s
sys     0m0,027s
```


### Second attempt

Yeah, using Python was a mistake. It's probably not going to terminate before the heat death of the universe. Let's rewrite it in Rust:

```rust
use base58::FromBase58;
use itertools::Itertools;
use sha2::{digest::Update, Digest, Sha256};

fn main() {
    let address_lowercase = "1lbcfr7sahtd9cgdqo3htmtkv8lk4znx71";

    let addresses = address_lowercase
        .bytes()
        .map(|byte| {
            if byte.to_ascii_uppercase() != byte {
                vec![byte, byte.to_ascii_uppercase()]
            } else {
                vec![byte]
            }
        })
        .multi_cartesian_product();

    for address in addresses {
        let address = String::from_utf8(address).unwrap();
        let Ok(decoded_address) = address.from_base58() else {
            continue;
        };
        if decoded_address.len() != 25 {
            continue;
        }
        let round1 = Sha256::new().chain(&decoded_address[..21]).finalize();
        let round2 = Sha256::new().chain(round1).finalize();
        if decoded_address[21..] == round2[..4] {
            eprintln!("Found valid address: {address}");
        }
    }
}
```

```shell
$ time cargo run --bin attempt2 --release
    Finished `release` profile [optimized] target(s) in 0.01s
     Running `target/release/attempt2`
Found valid address: 1Lbcfr7sAHTD9CgdQo3HTMTkV8LK4ZnX71

real    0m34,296s
user    0m34,275s
sys     0m0,013s
```

Yay! That's precisely the address that we started with.

Problem solved? Well, yeah, but in my case, address recovery was a part of an automated process. That means I'm not even sure if the input is total garbage.

Can `1hell0w0rldd9cgdqo3htmtkv8lk4znx71` be restored to a valid Bitcoin address? Maybe! Does anyone actually use that address? I don't know! They could have gotten lucky with Vanitygen.

Should I spend my CPU time trying to recover `1aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa`? Probably not, but how should I check that automatically while avoiding false negatives?

And that's how the journey towards the fastest recovering algorithm started.


### Third attempt

The first step in the process is decoding base58. Let's check out what the base58 encoding actually does to see if we can cut any corners.

1. We start with an arbitrary byte string, e.g. `00 d6 f6 4e e7 83 6a cf 6e 5a 93 7d 63 54 c3 a5 96 cd 24 2d fc 2f 78 fa 7c` (represented in hex for simplicity).
2. We interpret the byte string as a long number in big-endian: `5270856372487448678887896392566731007782045065082238990972`.
3. We then encode the number in radix-58: `19, 34, 35, 38, 49, 6, 50, 9, 16, 26, 12, 8, 11, 39, 36, 23, 46, 2, 16, 26, 20, 26, 43, 28, 7, 19, 18, 3, 32, 45, 30, 6, 0`.
4. This conversion process does not preserve the count of leading zero bytes (e.g. `00 ff` and `ff` map to one radix-58 sequence, namely `4, 23`), so we *add* all the leading zeroes from the byte string to the radix-58 representation. There was just one zero byte in the byte string, so we write `0, 19, 34, 35, 38, 49, 6, 50, 9, 16, 26, 12, 8, 11, 39, 36, 23, 46, 2, 16, 26, 20, 26, 43, 28, 7, 19, 18, 3, 32, 45, 30, 6, 0`.
5. Finally, we replace the numbers 0-57 with the corresponding characters from the sequence `123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz`. For instance, 0 maps to `1` and 57 maps to `z`: `1Lbcfr7sAHTD9CgdQo3HTMTkV8LK4ZnX71`.

Conversely, the decoding process is as follows:

1. Replace the alphabet characters with numbers 0-57.
2. Decode the long number from big-endian radix-58.
3. Encode the long number into big-endian radix-256.
4. Add however many leading zeroes there were in the radix-58 representation to the radix-256 representation.

So, how does replacing a single character in the encoded string affect the byte string? For example, let's replace `A` with `a` in the example address. On the first step, <code>..., 6, 50, **9**, 16, 26, ...</code> changes to <code>..., 6, 50, **33**, 16, 26, ...</code>. This is just adding 33 - 9 = 24 to a digit of a long number, which increases the long number from step 2 by $24 \cdot 58^{25}$. In radix-256, this is still just addition with carry. Step 4 is a bit of a mouthful, but we can sidestep the complexity by abusing that Bitcoin addresses with a checksum are always 25 bytes long.

So, how about we parse the address just once and then simulate flipping the case by adding or removing a constant (say, $24 \cdot 58^{25}$ in the example above) from the byte representation of the address? Let's do just that:

```rust expansible
use itertools::Itertools;
use num_bigint::BigUint;
use num_traits::{One, Zero};
use sha2::{digest::Update, Digest, Sha256};

const BASE58_ALPHABET: &'static [u8] =
    b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

fn main() {
    let garbled_address = "1lbcfr7sahtd9cgdqo3htmtkv8lk4znx71";

    let mut parsed_number: BigUint = Zero::zero();
    let mut power_58_i: BigUint = One::one();
    let mut possible_differences = Vec::new();
    let mut base_address = vec![0u8; garbled_address.len()];

    for (digit_index, byte) in garbled_address.bytes().enumerate().rev() {
        // Some letters, like L and o, are only valid base58 characters in one case; this
        // complicates the code a bit
        let digit1 = BASE58_ALPHABET
            .iter()
            .position(|&b| b == byte.to_ascii_uppercase());
        let digit2 = BASE58_ALPHABET
            .iter()
            .position(|&b| b == byte.to_ascii_lowercase());
        match (digit1, digit2) {
            (Some(digit1), Some(digit2)) if digit1 != digit2 => {
                // Two distinct variants are possible
                parsed_number += digit1 * &power_58_i;
                base_address[digit_index] = byte.to_ascii_uppercase();
                // digit1 is uppercase, digit2 is lowercase, lowercase comes after uppercase in the
                // alphabet, so the difference is positive
                possible_differences.push((digit_index, (digit2 - digit1) * &power_58_i));
            }
            (Some(digit), _) => {
                // Just the first variant is right
                parsed_number += digit * &power_58_i;
                base_address[digit_index] = byte.to_ascii_uppercase();
            }
            (_, Some(digit)) => {
                // Just the second variant is right
                parsed_number += digit * &power_58_i;
                base_address[digit_index] = byte.to_ascii_lowercase();
            }
            (None, None) => panic!("The address is invalid"),
        }
        power_58_i *= 58u32;
    }

    let subsets_of_possible_differences = possible_differences
        .iter()
        .map(|_| [false, true])
        .multi_cartesian_product();

    for subset in subsets_of_possible_differences {
        // Compute the original number plus fixups for this particular subset of characters that are
        // switched to uppercase
        let mut fixed_number = parsed_number.clone();
        for (&should_take, (_, addend)) in subset.iter().zip(&possible_differences) {
            if should_take {
                fixed_number += addend;
            }
        }

        // Convert the number to a byte string
        let byte_string = fixed_number.to_bytes_be();

        // Pad with zeroes at the beginning to exactly 25 bytes
        if byte_string.len() > 25 {
            // Got unlucky, this is certainly not a valid Bitcoin address
            continue;
        }
        let mut byte_string_25 = [0u8; 25];
        byte_string_25[25 - byte_string.len()..].copy_from_slice(&byte_string);

        let round1 = Sha256::new().chain(&byte_string_25[..21]).finalize();
        let round2 = Sha256::new().chain(round1).finalize();
        if byte_string_25[21..] != round2[..4] {
            // Checksum failed
            continue;
        }

        // This must be a valid address. Let's recover the string.
        let mut address = base_address.clone();
        for (&should_take, &(digit_index, _)) in subset.iter().zip(&possible_differences) {
            if should_take {
                // This character was converted to lowercase
                address[digit_index] = address[digit_index].to_ascii_lowercase();
            }
        }
        let address = String::from_utf8(address).unwrap();
        eprintln!("Found valid address: {address}");
    }
}
```

How did we do?

```shell
$ time cargo run --bin attempt3 --release
    Finished `release` profile [optimized] target(s) in 0.01s
     Running `target/release/attempt3`
Found valid address: 1Lbcfr7sAHTD9CgdQo3HTMTkV8LK4ZnX71

real    0m6,625s
user    0m6,615s
sys     0m0,009s
```

Hey, that's much better! Honestly, it's somewhat embarrassing that the slowest part of the last attempt was parsing, not hashing. That's why profilers are so important! Sometimes, one can't guess the bottleneck, even if it seems obvious.


### Fourth attempt

Before we try to apply clever optimizations to other parts of the program, let's see if we've maxed out the performance of byte string generation.

Suppose there are just three letters in the address, so `possible_differences` contains three fixup numbers: `A`, `B`, and `C`. We then iterate through eight subsets of `{A, B, C}` and compute the following numbers:

- `base`
- `base + A`
- `base + B`
- `base + A + B`
- `base + C`
- `base + A + C`
- `base + B + C`
- `base + A + B + C`

The weird part is *how* we compute them. To calculate `base + A + B + C`, we start with `base` and then iteratively add three numbers to it, even though we have computed `base + B + C` in the previous step, and just remembering that value and adding `A` to it would be faster.

So, can we optimize this computation? Of course, we can!

What we've been doing before was asking three questions in a row:

- Do we add `A` or not?
- Do we add `B` or not?
- Do we add `C` or not?

And iterating through all possible combinations. We can instead interpret this process as a *recursive* walk down a decision tree rather than a flat algorithm:

```tikz
\node[draw,diamond,thick,aspect=2] (Root) at(0,0) {Add A?};

\node[draw,diamond,thick,aspect=2] (Yes) at(-6,-2) {Add B?};
\node[draw,diamond,thick,aspect=2] (No) at(6,-2) {Add B?};
\draw[-{>[length=0.25cm]},thick] (Root) -- node[left=0.5cm,above=0.25cm] {Yes} (Yes);
\draw[-{>[length=0.25cm]},thick] (Root) -- node[right=0.5cm,above=0.25cm] {No} (No);

\node[draw,diamond,thick,aspect=2] (YesYes) at(-9,-4) {Add C?};
\node[draw,diamond,thick,aspect=2] (YesNo) at(-3,-4) {Add C?};
\node[draw,diamond,thick,aspect=2] (NoYes) at(3,-4) {Add C?};
\node[draw,diamond,thick,aspect=2] (NoNo) at(9,-4) {Add C?};
\draw[-{>[length=0.25cm]},thick] (Yes) -- node[left=0.4cm,above=0.1cm] {Yes} (YesYes);
\draw[-{>[length=0.25cm]},thick] (Yes) -- node[right=0.4cm,above=0.1cm] {No} (YesNo);
\draw[-{>[length=0.25cm]},thick] (No) -- node[left=0.4cm,above=0.1cm] {Yes} (NoYes);
\draw[-{>[length=0.25cm]},thick] (No) -- node[right=0.4cm,above=0.1cm] {No} (NoNo);

\node[draw,rectangle,thick,inner sep=0.2cm] (YesYesYes) at(-10.5,-6) {base+A+B+C};
\node[draw,rectangle,thick,inner sep=0.2cm] (YesYesNo) at(-7.5,-6) {base+A+B};
\node[draw,rectangle,thick,inner sep=0.2cm] (YesNoYes) at(-4.5,-6) {base+A+C};
\node[draw,rectangle,thick,inner sep=0.2cm] (YesNoNo) at(-1.5,-6) {base+A};
\node[draw,rectangle,thick,inner sep=0.2cm] (NoYesYes) at(1.5,-6) {base+B+C};
\node[draw,rectangle,thick,inner sep=0.2cm] (NoYesNo) at(4.5,-6) {base+B};
\node[draw,rectangle,thick,inner sep=0.2cm] (NoNoYes) at(7.5,-6) {base+C};
\node[draw,rectangle,thick,inner sep=0.2cm] (NoNoNo) at(10.5,-6) {base};
\draw[-{>[length=0.25cm]},thick] (YesYes) -- node[left=0.4cm,above=0cm] {Yes} (YesYesYes);
\draw[-{>[length=0.25cm]},thick] (YesYes) -- node[right=0.4cm,above=0cm] {No} (YesYesNo);
\draw[-{>[length=0.25cm]},thick] (YesNo) -- node[left=0.4cm,above=0cm] {Yes} (YesNoYes);
\draw[-{>[length=0.25cm]},thick] (YesNo) -- node[right=0.4cm,above=0cm] {No} (YesNoNo);
\draw[-{>[length=0.25cm]},thick] (NoYes) -- node[left=0.4cm,above=0cm] {Yes} (NoYesYes);
\draw[-{>[length=0.25cm]},thick] (NoYes) -- node[right=0.4cm,above=0cm] {No} (NoYesNo);
\draw[-{>[length=0.25cm]},thick] (NoNo) -- node[left=0.4cm,above=0cm] {Yes} (NoNoYes);
\draw[-{>[length=0.25cm]},thick] (NoNo) -- node[right=0.4cm,above=0cm] {No} (NoNoNo);
```

The recursion looks like this:

```rust expansible
fn iterate_through_possible_differences(
    partly_fixed_number: &BigUint,
    possible_differences_tail: &[(usize, BigUint)],
    subset_bitmask: u64,
    base_address: &Vec<u8>,
    possible_differences: &[(usize, BigUint)],
) {
    if let Some((digit_index, fixup_to_add)) = possible_differences_tail.get(0) {
        // Should we add this fixup?

        // Yes
        iterate_through_possible_differences(
            &(partly_fixed_number + fixup_to_add),
            &possible_differences_tail[1..],
            subset_bitmask | (1 << digit_index),
            base_address,
            possible_differences,
        );

        // No
        iterate_through_possible_differences(
            partly_fixed_number,
            &possible_differences_tail[1..],
            subset_bitmask,
            base_address,
            possible_differences,
        );

        return;
    }

    // No more decisions to make, yay!

    // Convert the number to a byte string
    let byte_string = partly_fixed_number.to_bytes_be();

    // Pad with zeroes at the beginning to exactly 25 bytes
    if byte_string.len() > 25 {
        // Got unlucky, this is certainly not a valid Bitcoin address
        return;
    }
    let mut byte_string_25 = [0u8; 25];
    byte_string_25[25 - byte_string.len()..].copy_from_slice(&byte_string);

    let round1 = Sha256::new().chain(&byte_string_25[..21]).finalize();
    let round2 = Sha256::new().chain(round1).finalize();
    if byte_string_25[21..] != round2[..4] {
        // Checksum failed
        return;
    }

    // This must be a valid address. Let's recover the string.
    let mut address = base_address.clone();
    for &(digit_index, _) in possible_differences {
        if (subset_bitmask >> digit_index) & 1 == 1 {
            // This character was converted to lowercase
            address[digit_index] = address[digit_index].to_ascii_lowercase();
        }
    }
    let address = String::from_utf8(address).unwrap();
    eprintln!("Found valid address: {address}");
}
```

And is initiated like this:

```rust
iterate_through_possible_differences(
    &parsed_number,
    &possible_differences,
    0,
    &base_address,
    &possible_differences,
);
```

This is a bit of a mouthful because we explicitly pass the parameters `base_address` and `possible_differences` down the recursion. In a better world, this would be a struct method, and the arguments would be passed implicitly, but that's just prototype code for now.

How did we do?

```shell
$ time cargo run --bin attempt4 --release
    Finished `release` profile [optimized] target(s) in 0.01s
     Running `target/release/attempt4`
Found valid address: 1Lbcfr7sAHTD9CgdQo3HTMTkV8LK4ZnX71

real    0m5,744s
user    0m5,729s
sys     0m0,014s
```

That's a slight improvement, perhaps not worth the hassle at first glance, but we'll see how it helps us down the road soon.


### Fifth attempt

Now that parsing is mostly out of the way, let's see if we can optimize hashing out. Well, duh, we can't do that without breaking cryptography, but can we maybe make do with computing fewer hashes?..

Let's try to watch what happens when we flip the case of a letter up close.

```tikz
\draw[DeepMagenta,thick,fill=ShallowMagenta] (-17,0) grid (17,-1) rectangle (-17,0);
\fill[DeepMagenta] (-9,0) rectangle (-8,-1);
\node[scale=2] at(-16.5,-0.5) {1};
\node[scale=2] at(-15.5,-0.5) {L};
\node[scale=2] at(-14.5,-0.5) {b};
\node[scale=2] at(-13.5,-0.5) {c};
\node[scale=2] at(-12.5,-0.5) {f};
\node[scale=2] at(-11.5,-0.5) {r};
\node[scale=2] at(-10.5,-0.5) {7};
\node[scale=2] at(-9.5,-0.5) {s};
\node[scale=2] at(-8.5,-0.5) {A};
\node[scale=2] at(-7.5,-0.5) {H};
\node[scale=2] at(-6.5,-0.5) {T};
\node[scale=2] at(-5.5,-0.5) {D};
\node[scale=2] at(-4.5,-0.5) {9};
\node[scale=2] at(-3.5,-0.5) {C};
\node[scale=2] at(-2.5,-0.5) {g};
\node[scale=2] at(-1.5,-0.5) {d};
\node[scale=2] at(-0.5,-0.5) {Q};
\node[scale=2] at(0.5,-0.5) {o};
\node[scale=2] at(1.5,-0.5) {3};
\node[scale=2] at(2.5,-0.5) {H};
\node[scale=2] at(3.5,-0.5) {T};
\node[scale=2] at(4.5,-0.5) {M};
\node[scale=2] at(5.5,-0.5) {T};
\node[scale=2] at(6.5,-0.5) {k};
\node[scale=2] at(7.5,-0.5) {V};
\node[scale=2] at(8.5,-0.5) {8};
\node[scale=2] at(9.5,-0.5) {L};
\node[scale=2] at(10.5,-0.5) {K};
\node[scale=2] at(11.5,-0.5) {4};
\node[scale=2] at(12.5,-0.5) {Z};
\node[scale=2] at(13.5,-0.5) {n};
\node[scale=2] at(14.5,-0.5) {X};
\node[scale=2] at(15.5,-0.5) {7};
\node[scale=2] at(16.5,-0.5) {1};

\draw[-{>[length=0.25cm]},thick] (0,-1) -- node[right=0.5cm,scale=2] {Decode base58} (0,-3);

\draw[DeepGold,thick,fill=ShallowGold,shift={(0.5,0)}] (-13,-3) grid (12,-4) rectangle (-13,-3);
\fill[DeepGold] (-6.5,-3) rectangle (9.5,-4);
\node[scale=2] at(-12,-3.5) {00};
\node[scale=2] at(-11,-3.5) {d6};
\node[scale=2] at(-10,-3.5) {f6};
\node[scale=2] at(-9,-3.5) {4e};
\node[scale=2] at(-8,-3.5) {e7};
\node[scale=2] at(-7,-3.5) {83};
\node[scale=2] at(-6,-3.5) {6a};
\node[scale=2] at(-5,-3.5) {cf};
\node[scale=2] at(-4,-3.5) {6e};
\node[scale=2] at(-3,-3.5) {5a};
\node[scale=2] at(-2,-3.5) {93};
\node[scale=2] at(-1,-3.5) {7d};
\node[scale=2] at(0,-3.5) {63};
\node[scale=2] at(1,-3.5) {54};
\node[scale=2] at(2,-3.5) {c3};
\node[scale=2] at(3,-3.5) {a5};
\node[scale=2] at(4,-3.5) {96};
\node[scale=2] at(5,-3.5) {cd};
\node[scale=2] at(6,-3.5) {24};
\node[scale=2] at(7,-3.5) {2d};
\node[scale=2] at(8,-3.5) {fc};
\node[scale=2] at(9,-3.5) {2f};
\node[scale=2] at(10,-3.5) {78};
\node[scale=2] at(11,-3.5) {fa};
\node[scale=2] at(12,-3.5) {7c};
```

```tikz
\draw[DeepMagenta,thick,fill=ShallowMagenta] (-17,0) grid (17,-1) rectangle (-17,0);
\fill[DeepMagenta] (-9,0) rectangle (-8,-1);
\node[scale=2] at(-16.5,-0.5) {1};
\node[scale=2] at(-15.5,-0.5) {L};
\node[scale=2] at(-14.5,-0.5) {b};
\node[scale=2] at(-13.5,-0.5) {c};
\node[scale=2] at(-12.5,-0.5) {f};
\node[scale=2] at(-11.5,-0.5) {r};
\node[scale=2] at(-10.5,-0.5) {7};
\node[scale=2] at(-9.5,-0.5) {s};
\node[scale=2] at(-8.5,-0.5) {a};
\node[scale=2] at(-7.5,-0.5) {H};
\node[scale=2] at(-6.5,-0.5) {T};
\node[scale=2] at(-5.5,-0.5) {D};
\node[scale=2] at(-4.5,-0.5) {9};
\node[scale=2] at(-3.5,-0.5) {C};
\node[scale=2] at(-2.5,-0.5) {g};
\node[scale=2] at(-1.5,-0.5) {d};
\node[scale=2] at(-0.5,-0.5) {Q};
\node[scale=2] at(0.5,-0.5) {o};
\node[scale=2] at(1.5,-0.5) {3};
\node[scale=2] at(2.5,-0.5) {H};
\node[scale=2] at(3.5,-0.5) {T};
\node[scale=2] at(4.5,-0.5) {M};
\node[scale=2] at(5.5,-0.5) {T};
\node[scale=2] at(6.5,-0.5) {k};
\node[scale=2] at(7.5,-0.5) {V};
\node[scale=2] at(8.5,-0.5) {8};
\node[scale=2] at(9.5,-0.5) {L};
\node[scale=2] at(10.5,-0.5) {K};
\node[scale=2] at(11.5,-0.5) {4};
\node[scale=2] at(12.5,-0.5) {Z};
\node[scale=2] at(13.5,-0.5) {n};
\node[scale=2] at(14.5,-0.5) {X};
\node[scale=2] at(15.5,-0.5) {7};
\node[scale=2] at(16.5,-0.5) {1};

\draw[-{>[length=0.25cm]},thick] (0,-1) -- node[right=0.5cm,scale=2] {Decode base58} (0,-3);

\draw[DeepGold,thick,fill=ShallowGold,shift={(0.5,0)}] (-13,-3) grid (12,-4) rectangle (-13,-3);
\fill[DeepGold] (-6.5,-3) rectangle (9.5,-4);
\node[scale=2] at(-12,-3.5) {00};
\node[scale=2] at(-11,-3.5) {d6};
\node[scale=2] at(-10,-3.5) {f6};
\node[scale=2] at(-9,-3.5) {4e};
\node[scale=2] at(-8,-3.5) {e7};
\node[scale=2] at(-7,-3.5) {83};
\node[scale=2] at(-6,-3.5) {ed};
\node[scale=2] at(-5,-3.5) {e8};
\node[scale=2] at(-4,-3.5) {2e};
\node[scale=2] at(-3,-3.5) {9b};
\node[scale=2] at(-2,-3.5) {70};
\node[scale=2] at(-1,-3.5) {9c};
\node[scale=2] at(0,-3.5) {15};
\node[scale=2] at(1,-3.5) {fa};
\node[scale=2] at(2,-3.5) {a2};
\node[scale=2] at(3,-3.5) {f2};
\node[scale=2] at(4,-3.5) {6f};
\node[scale=2] at(5,-3.5) {17};
\node[scale=2] at(6,-3.5) {13};
\node[scale=2] at(7,-3.5) {0e};
\node[scale=2] at(8,-3.5) {d3};
\node[scale=2] at(9,-3.5) {9f};
\node[scale=2] at(10,-3.5) {78};
\node[scale=2] at(11,-3.5) {fa};
\node[scale=2] at(12,-3.5) {7c};
```

:::aside
As no string can have two valid checksums, a typo in the last few characters of a Bitcoin address will definitely be noticed. On the other hand, making a typo in other characters might lead to a hash collision, meaning that the typo is unnoticed with probability $2^{-32}$. This means that Bitcoin addresses are ever so slightly more protected against a typo at the end of the address when the person writing the address by hand is more tired. This effect only exists because of two seemingly random choices: the checksum being appended to the data rather than prepended, and the use of big-endian instead of little-endian. Whether this was a conscious decision made by Satoshi or just a lucky coincidence is an interesting question to ponder.
:::

It turns out that flipping the case of a letter affects a *subinterval* of the underlying byte string. Indeed, if you add a small number to a longer one, it's quite reasonable that only its lower digits are changed (except in the unlikely cases when long carry propagation happens). Some trailing bits are unchanged because 58 is even, so, for instance, the lowest three bytes of $x \cdot 58^{25}$ are necessarily zero.

That's worth researching. See, if flipping the case of some letters affects just the checksum, we don't have to iterate through all the combinations of those letters and compute the hash of the immutable 21-byte prefix each time because we can straightforwardly compute the checksum of the unchanged 21-byte prefix once and infer the correct letter cases from that. Unfortunately, if some letters affect just the first 21 bytes, that does not really let us optimize anything: two strings can easily have identical checksums.

<aside-inline-here />

Here's the new algorithm: we shall recursively iterate our way through all combinations of cases, branching on the letters from left to right, and when we determine no further changes in case reflect on the leading 21 bytes, we stop guessing and compute the hash directly.

```rust expansible
// snip

fn iterate_through_possible_differences(
    partly_fixed_number: &BigUint,
    possible_differences_tail: &[(usize, BigUint)],
    suffix_sums_of_possible_differences_tail: &[BigUint],
    mut subset_bitmask: u64,
    base_address: &Vec<u8>,
    possible_differences: &[(usize, BigUint)],
) {
    let interval_length = &suffix_sums_of_possible_differences_tail[0];
    // Regardless of our further choices, the final fixed_number will be in range
    // [partly_fixed_number, partly_fixed_number + interval_length]. Do all the numbers in this
    // range have identical leading 21 bytes?

    if partly_fixed_number >> 32 != (partly_fixed_number + interval_length) >> 32 {
        // No, they don't. This means we have to guess whether to add the current fixup.
        let (digit_index, fixup_to_add) = &possible_differences_tail[0];

        // Yes
        iterate_through_possible_differences(
            &(partly_fixed_number + fixup_to_add),
            &possible_differences_tail[1..],
            &suffix_sums_of_possible_differences_tail[1..],
            subset_bitmask | (1 << digit_index),
            base_address,
            possible_differences,
        );

        // No
        iterate_through_possible_differences(
            partly_fixed_number,
            &possible_differences_tail[1..],
            &suffix_sums_of_possible_differences_tail[1..],
            subset_bitmask,
            base_address,
            possible_differences,
        );

        return;
    }

    // Yes, they are the same. Let's compute the expected checksum.

    // Convert the number to a byte string
    let byte_string = partly_fixed_number.to_bytes_be();

    // Pad with zeroes at the beginning to exactly 25 bytes
    if byte_string.len() > 25 {
        // Got unlucky, this is certainly not a valid Bitcoin address
        return;
    }
    let mut byte_string_25 = [0u8; 25];
    byte_string_25[25 - byte_string.len()..].copy_from_slice(&byte_string);

    let round1 = Sha256::new().chain(&byte_string_25[..21]).finalize();
    let round2 = Sha256::new().chain(round1).finalize();
    let correct_checksum = u32::from_be_bytes(*round2.first_chunk::<4>().unwrap());

    let mut current_checksum =
        u32::from_be_bytes(*byte_string_25[21..].first_chunk::<4>().unwrap());
    if current_checksum >= correct_checksum {
        // The minimal possible value is greater than what we need. Therefore, there is no solution.
        return;
    }

    // Determine which fixups to add to obtain current_checksum == correct_checksum
    for (digit_index, fixup_to_add) in possible_differences_tail {
        // fixup_to_add necessarily fits in u32
        let fixup_to_add = fixup_to_add.iter_u32_digits().next().unwrap();
        if current_checksum + fixup_to_add <= correct_checksum {
            subset_bitmask |= 1 << digit_index;
            current_checksum += fixup_to_add;
        }
    }

    if current_checksum != correct_checksum {
        // No solution
        return;
    }

    // This must be a valid address. Let's recover the string.
    let mut address = base_address.clone();
    for &(digit_index, _) in possible_differences {
        if (subset_bitmask >> digit_index) & 1 == 1 {
            // This character was converted to lowercase
            address[digit_index] = address[digit_index].to_ascii_lowercase();
        }
    }
    let address = String::from_utf8(address).unwrap();
    eprintln!("Found valid address: {address}");
}

fn main() {
    // snip

    // Recurse from left to right, i.e. from most significant to least significant
    possible_differences.reverse();

    // For differences A, B, C, computes [A + B + C, B + C, C, 0].
    let mut suffix_sums_of_possible_differences: Vec<BigUint> = possible_differences
        .iter()
        .rev()
        .scan(<BigUint as Zero>::zero(), |state, (_, fixup)| {
            *state += fixup;
            Some(state.clone())
        })
        .collect();
    suffix_sums_of_possible_differences.reverse();
    suffix_sums_of_possible_differences.push(Zero::zero());

    iterate_through_possible_differences(
        &parsed_number,
        &possible_differences,
        &suffix_sums_of_possible_differences,
        0,
        &base_address,
        &possible_differences,
    );
}
```

Theoretically, this should optimize $O(2^n)$ performance to $O(n)$ on a few terminal characters. Is that true in practice?

```shell
$ time cargo run --bin attempt5 --release
    Finished `release` profile [optimized] target(s) in 0.01s
     Running `target/release/attempt5`
Found valid address: 1Lbcfr7sAHTD9CgdQo3HTMTkV8LK4ZnX71

real    0m0,954s
user    0m0,941s
sys     0m0,013s
```

Yeah, sure it is!

We've already made our code *35 times* faster than the original Rust version. And the brute force taking under a second looks even quicker in comparison to the Python version, which would take God knows how much time. Can we use our well-earned rest? Let's try to restore another address just before doing that:

```diff
-let garbled_address = "1lbcfr7sahtd9cgdqo3htmtkv8lk4znx71";
+let garbled_address = "18ryviovmwfyzhrzktjkqgycjkujoxh3k6";
```

```shell
$ time cargo run --bin attempt5 --release
    Finished `release` profile [optimized] target(s) in 0.01s
     Running `target/release/attempt5`
Found valid address: 18ryVioVmwFYzhRZKTjKqGYCjkUjoxH3k6

real    0m14,347s
user    0m14,334s
sys     0m0,012s
```

Oh. That's no good. The reason it's so slow is clear in retrospect: there are much fewer digits in this address than in the one we've used for benchmarking before. And that's not just me being a dick, that's actually a real randomly generated address I've used on ZeroNet.


### Sixth attempt

We have probably applied all domain-specific optimizations, so let's start a profiler and see if there's any non-asymptotic low-hanging fruit!

```shell
$ perf record target/release/attempt5
WARNING: perf not found for kernel 6.8.1-060801

  You may need to install the following packages for this specific kernel:
    linux-tools-6.8.1-060801-generic
    linux-cloud-tools-6.8.1-060801-generic

  You may also want to install one of the following packages to keep up to date:
    linux-tools-generic
    linux-cloud-tools-generic
```

:::aside
My girlfriend uses Arch, by the way.
:::

Right, sorry, I'm using mainline on Ubuntu. Let me grab a `perf` and whatever it links to from another computer.

<aside-inline-here />

```shell
$ alias perf="LD_LIBRARY_PATH=. ./perf"

$ perf record target/release/attempt5
Found valid address: 18ryVioVmwFYzhRZKTjKqGYCjkUjoxH3k6
[ perf record: Woken up 9 times to write data ]
[ perf record: Captured and wrote 2,271 MB perf.data (59225 samples) ]

$ perf report
Samples: 59K of event 'cycles:P', Event count (approx.): 58317241835
Overhead  Command   Shared Object      Symbol
  66,33%  attempt5  attempt5           [.] sha2::sha256::soft::compress
   8,23%  attempt5  attempt5           [.] attempt5::iterate_through_possible_differences
   5,63%  attempt5  attempt5           [.] num_bigint::biguint::convert::to_bitwise_digits_le
   5,50%  attempt5  attempt5           [.] num_bigint::biguint::shift::biguint_shr2
   3,80%  attempt5  libc.so.6          [.] _int_free
   2,68%  attempt5  libc.so.6          [.] malloc
   2,17%  attempt5  libc.so.6          [.] cfree@GLIBC_2.2.5
   1,93%  attempt5  attempt5           [.] num_bigint::biguint::addition::<impl core::ops::arith::Add<&num_bigint::biguint::BigUint> for num_bigint::biguint::BigUint>::add
<snip>
```

That's hardly surprising: the slowest part is due to computing hashes. How does the `sha2` crate compute hashes, anyway?

[https://docs.rs/sha2](https://docs.rs/sha2) does not say anything about different implementations. Neither does the GitHub page. But, oh, wait, the GitHub repo shows that its [Cargo.toml](https://github.com/RustCrypto/hashes/blob/sha2-v0.10.8/sha2/Cargo.toml) has some intriguing features!

```toml
[features]
default = ["std"]
std = ["digest/std"]
oid = ["digest/oid"] # Enable OID support. WARNING: Bumps MSRV to 1.57
asm = ["sha2-asm"] # WARNING: this feature SHOULD NOT be enabled by library crates
# Use assembly backend for LoongArch64 targets
# WARNING: Bumps MSRV to 1.72. This feature SHOULD NOT be enabled by library crates
loongarch64_asm = []
compress = [] # Expose compress functions
force-soft = [] # Force software implementation
asm-aarch64 = ["asm"] # DEPRECATED: use `asm` instead
```

Should we enable the `asm` feature, maybe?..

```shell
$ cargo add sha2 -F asm
    Updating crates.io index
      Adding sha2 v0.10.8 to dependencies
             Features:
             + asm
             + sha2-asm
             + std
             - asm-aarch64
             - compress
             - force-soft
             - loongarch64_asm
             - oid
    Updating crates.io index
     Locking 2 packages
      Adding cc v1.0.94
      Adding sha2-asm v0.6.3

$ cargo build --bin attempt5 --release
<snip>

$ time cargo run --bin attempt5 --release
    Finished `release` profile [optimized] target(s) in 0.01s
     Running `target/release/attempt5`
Found valid address: 18ryVioVmwFYzhRZKTjKqGYCjkUjoxH3k6

real    0m13,143s
user    0m13,127s
sys     0m0,015s
```

The runtime decreased a bit, but not significantly. Sounds like `sha2` is a dead-end.

What is SHA-256, anyway? The Wikipedia page for [SHA-2](https://en.wikipedia.org/wiki/SHA-2) says:

> SHA-256 and SHA-512 are novel hash functions computed with eight 32-bit and 64-bit words, respectively.

This hints that the computation of a SHA-256 hash amounts to several operations on 32-bit integers. Indeed, the pseudocode does just that. Can we, perhaps, use SIMD? Many x86 processors support the AVX and AVX2 extensions, which, among other things, enable parallel computations on eight 32-bit integers; older and non-x86 processors typically only support 128-bit SIMD, but that should still give us a fourfold increase in performance. Worth a try!

Let's start with some boilerplate:

```rust expansible
// snip

struct ParallelChecksum<'a> {
    todo: PhantomData<&'a ()>,
}

impl<'a> ParallelChecksum<'a> {
    fn new() -> Self {
        Self { todo: PhantomData }
    }

    fn compute_later(&mut self, string: [u8; 21], callback: Box<dyn FnOnce(u32) + 'a>) {
        // TODO: Parallelize
        let round1 = Sha256::new().chain(&string).finalize();
        let round2 = Sha256::new().chain(round1).finalize();
        callback(u32::from_be_bytes(*round2.first_chunk::<4>().unwrap()));
    }

    fn finalize(self) {
        // TODO
    }
}

// snip

fn iterate_through_possible_differences<'a>(
    // snip
    parallel_checksum: &mut ParallelChecksum<'a>,
) {
    // snip

    parallel_checksum.compute_later(
        *byte_string_25.first_chunk::<21>().unwrap(),
        Box::new(move |correct_checksum| {
            if current_checksum >= correct_checksum {
                // The minimal possible value is greater than what we need. Therefore, there is no
                // solution.
                return;
            }
            // snip
            eprintln!("Found valid address: {address}");
        }),
    );
}

fn main() {
    // snip

    let mut parallel_checksum = ParallelChecksum::new();

    iterate_through_possible_differences(
        // snip
        &mut parallel_checksum,
    );

    parallel_checksum.finalize();
}
```

Does this work?

```shell
$ time cargo run --bin attempt6 --release
    Finished `release` profile [optimized] target(s) in 0.01s
     Running `target/release/attempt6`
Found valid address: 18ryVioVmwFYzhRZKTjKqGYCjkUjoxH3k6

real    0m13,922s
user    0m13,864s
sys     0m0,023s
```

More or less fine, I guess. Time to write some SIMD! In C, we'd have to use `<immintrin.h>` or `<arm_neon.h>`, but nightly Rust has portable SIMD support, so we will use that.

Let's try a straight-forward reimplementation of SHA-256, straight from Wikipedia's pseudocode:

```rust expansible
fn sha256_parallel_8<const LENGTH: usize>(inputs: [[u8; LENGTH]; 8]) -> [[u8; 32]; 8] {
    // Initialization
    let mut h0 = u32x8::splat(0x6a09e667);
    let mut h1 = u32x8::splat(0xbb67ae85);
    let mut h2 = u32x8::splat(0x3c6ef372);
    let mut h3 = u32x8::splat(0xa54ff53a);
    let mut h4 = u32x8::splat(0x510e527f);
    let mut h5 = u32x8::splat(0x9b05688c);
    let mut h6 = u32x8::splat(0x1f83d9ab);
    let mut h7 = u32x8::splat(0x5be0cd19);

    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut w: [u32x8; 64] = [u32x8::splat(0); 64];

    // We assume the inputs fit in one 512-bit chunk
    assert!(LENGTH * 8 + 1 + 64 <= 512);

    // Copy the chunk into first 16 words w[0..15] of the message schedule array, padding it to 512
    // bits as follows:
    // - begin with the original message of length L bits
    // - append a single '1' bit
    // - append K '0' bits, where K is the minimum number >= 0 such that (L + 1 + K + 64) is a
    //   multiple of 512
    // - append L as a 64-bit big-endian integer, making the total post-processed length a multiple
    //   of 512 bits
    // - such that the bits in the message are: <original message of length L> 1 <K zeros>
    //   <L as 64 bit integer> , (the number of bits will be a multiple of 512)
    for input_index in 0..8 {
        let mut word_index = 0;
        let mut chunks = inputs[input_index].array_chunks::<4>();
        for word in &mut chunks {
            w[word_index][input_index] = u32::from_be_bytes(*word);
            word_index += 1;
        }
        let last_word_head = chunks.remainder();
        let mut last_word = [0u8; 4];
        last_word[..last_word_head.len()].copy_from_slice(last_word_head);
        last_word[last_word_head.len()] = 0x80;
        w[word_index][input_index] = u32::from_be_bytes(last_word);
    }
    w[15] = u32x8::splat((LENGTH * 8) as u32);

    // Extend the first 16 words into the remaining 48 words w[16..64] of the message schedule array
    let rotate_right = |word, count| (word >> count) | (word << (32 - count));
    for i in 16..64 {
        let s0 = rotate_right(w[i - 15], 7) ^ rotate_right(w[i - 15], 18) ^ (w[i - 15] >> 3);
        let s1 = rotate_right(w[i - 2], 17) ^ rotate_right(w[i - 2], 19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16] + s0 + w[i - 7] + s1;
    }

    // Initialize working variables to current hash value
    let mut a = h0;
    let mut b = h1;
    let mut c = h2;
    let mut d = h3;
    let mut e = h4;
    let mut f = h5;
    let mut g = h6;
    let mut h = h7;

    // Compression function main loop
    for (ki, wi) in K.into_iter().zip(w) {
        let s1 = rotate_right(e, 6) ^ rotate_right(e, 11) ^ rotate_right(e, 25);
        let ch = (e & f) ^ (!e & g);
        let temp1 = h + s1 + ch + u32x8::splat(ki) + wi;
        let s0 = rotate_right(a, 2) ^ rotate_right(a, 13) ^ rotate_right(a, 22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let temp2 = s0 + maj;

        h = g;
        g = f;
        f = e;
        e = d + temp1;
        d = c;
        c = b;
        b = a;
        a = temp1 + temp2;
    }

    // Add the compressed chunk to the current hash value
    h0 += a;
    h1 += b;
    h2 += c;
    h3 += d;
    h4 += e;
    h5 += f;
    h6 += g;
    h7 += h;

    // Produce the final hash value (big-endian)
    let mut outputs = [[0u8; 32]; 8];
    for output_index in 0..8 {
        for (word_index, word) in [h0, h1, h2, h3, h4, h5, h6, h7].iter().enumerate() {
            outputs[output_index][word_index * 4..][..4]
                .copy_from_slice(&word[output_index].to_be_bytes());
        }
    }
    outputs
}

struct ParallelChecksum<'a> {
    queue: ArrayVec<([u8; 21], Box<dyn FnOnce(u32) + 'a>), 8>,
}

impl<'a> ParallelChecksum<'a> {
    fn new() -> Self {
        Self {
            queue: ArrayVec::new(),
        }
    }

    fn flush(&mut self) {
        let mut inputs = [[0u8; 21]; 8];
        for (input, (string, _)) in inputs.iter_mut().zip(&mut self.queue) {
            *input = *string;
        }
        let round1 = sha256_parallel_8(inputs);
        let round2 = sha256_parallel_8(round1);
        for (output, (_, callback)) in round2.into_iter().zip(self.queue.drain(..)) {
            callback(u32::from_be_bytes(*output.first_chunk::<4>().unwrap()));
        }
    }

    fn compute_later(&mut self, string: [u8; 21], callback: Box<dyn FnOnce(u32) + 'a>) {
        if self.queue.len().is_full() {
            self.flush();
        }
        self.queue.push((string, callback));
    }

    fn finalize(mut self) {
        self.flush();
    }
}
```

```shell
$ time cargo run --bin attempt6 --release
    Finished `release` profile [optimized] target(s) in 0.01s
     Running `target/release/attempt6`
Found valid address: 18ryVioVmwFYzhRZKTjKqGYCjkUjoxH3k6

real    0m8,872s
user    0m8,858s
sys     0m0,012s
```

I'm pretty sure that's less than a 2x improvement. (Though an improvement nonetheless.) Perhaps we should apply our minds or something instead of thoughtlessly copying code.

For one thing, the compression function clearly, among other things, performs a rotation of the list `[a, b, c, d, e, f, g, h]`, and we can avoid doing it by renaming variables instead:

```rust
// Compression function main loop
let compress = |a, b, c, d: &mut _, e, f, g, h: &mut _, ki, wi| {
    let s1 = rotate_right(e, 6) ^ rotate_right(e, 11) ^ rotate_right(e, 25);
    let ch = (e & f) ^ (!e & g);
    let temp1 = *h + s1 + ch + u32x8::splat(ki) + wi;
    let s0 = rotate_right(a, 2) ^ rotate_right(a, 13) ^ rotate_right(a, 22);
    let maj = (a & b) ^ (a & c) ^ (b & c);
    let temp2 = s0 + maj;
    *d += temp1;
    *h = temp1 + temp2;
};
for i in (0..64).step_by(8) {
    compress(a, b, c, &mut d, e, f, g, &mut h, K[i], w[i]);
    compress(h, a, b, &mut c, d, e, f, &mut g, K[i + 1], w[i + 1]);
    compress(g, h, a, &mut b, c, d, e, &mut f, K[i + 2], w[i + 2]);
    compress(f, g, h, &mut a, b, c, d, &mut e, K[i + 3], w[i + 3]);
    compress(e, f, g, &mut h, a, b, c, &mut d, K[i + 4], w[i + 4]);
    compress(d, e, f, &mut g, h, a, b, &mut c, K[i + 5], w[i + 5]);
    compress(c, d, e, &mut f, g, h, a, &mut b, K[i + 6], w[i + 6]);
    compress(b, c, d, &mut e, f, g, h, &mut a, K[i + 7], w[i + 7]);
}
```

```shell
$ time cargo run --bin attempt6 --release
    Finished `release` profile [optimized] target(s) in 0.01s
     Running `target/release/attempt6`
Found valid address: 18ryVioVmwFYzhRZKTjKqGYCjkUjoxH3k6

real    0m8,411s
user    0m8,398s
sys     0m0,012s
```

That's a bit better. But it's still sluggish. What takes so long? Let's apply the profiler.

```shell
Samples: 36K of event 'cycles:P', 4000 Hz, Event count (approx.): 34898518133
attempt6::ParallelChecksum::flush  /home/purplesyringa/site/blog/recovering-garbled-bitcoin-addresses/riir/target/release/attempt6 [Percent: local period]
  0,52 │        psrld  $0xa,%xmm2
  0,24 │        pxor   %xmm5,%xmm2
  0,02 │        psrld  $0xa,%xmm3
  0,07 │        pxor   %xmm7,%xmm3
  0,51 │        movdqa %xmm1,%xmm4
  0,22 │        psrld  $0x7,%xmm4
  0,06 │        movdqa %xmm1,%xmm5
  0,06 │        pslld  $0x19,%xmm5
  0,48 │        por    %xmm4,%xmm5
  0,28 │        movdqa %xmm0,%xmm4
  0,03 │        psrld  $0x7,%xmm4
...
```

For one thing, there are lots of moves. That's because the compiler does not utilize the [VEX prefix](https://en.wikipedia.org/wiki/VEX_prefix), which enables non-destructive three-operand SIMD operations. VEX is enabled by the AVX extension ...wait. Is that `%xmm` instead of `%ymm`? I have a sinking feeling.

We've never told rustc it can use any CPU extensions *at all*. All this time, it's been making do with SSE2, which is guaranteed to be available on x86-64 processors, and emulated 256-bit operations with two 128-bit operations. Oh no.

```shell
$ time RUSTFLAGS="-C target-feature=+avx2" cargo run --bin attempt6 --release
    Finished `release` profile [optimized] target(s) in 0.01s
     Running `target/release/attempt6`
Found valid address: 18ryVioVmwFYzhRZKTjKqGYCjkUjoxH3k6

real    0m6,239s
user    0m6,223s
sys     0m0,015s
```

That's better. Not 2x better, though, for some reason.


### Seventh attempt

Let's check if there's anything interesting in the profiler again.

```
Samples: 25K of event 'cycles:P', Event count (approx.): 24819663114
Overhead  Command   Shared Object      Symbol
  29,23%  attempt6  attempt6           [.] attempt6::ParallelChecksum::flush
  12,68%  attempt6  attempt6           [.] attempt6::iterate_through_possible_differences
  12,36%  attempt6  attempt6           [.] num_bigint::biguint::convert::to_bitwise_digits_le
  11,30%  attempt6  attempt6           [.] num_bigint::biguint::shift::biguint_shr2
  10,07%  attempt6  libc.so.6          [.] _int_free
   7,50%  attempt6  libc.so.6          [.] malloc
   5,32%  attempt6  libc.so.6          [.] cfree@GLIBC_2.2.5
   4,09%  attempt6  attempt6           [.] num_bigint::biguint::addition::<impl core::ops::arith::Add<&num_bigint::biguint::BigUint> for num_bigint::biguint::BigUint>::add
   1,62%  attempt6  libc.so.6          [.] __memmove_avx_unaligned_erms
   1,28%  attempt6  libc.so.6          [.] __memcmp_avx2_movbe
   1,24%  attempt6  attempt6           [.] core::ops::function::FnOnce::call_once$u7b$$u7b$vtable.shim$u7d$$u7d$::h54ccd23766b804f6
   0,90%  attempt6  libc.so.6          [.] __memset_avx2_unaligned_erms
   0,64%  attempt6  attempt6           [.] __rust_dealloc
   0,64%  attempt6  libc.so.6          [.] _int_malloc
   0,56%  attempt6  attempt6           [.] __rust_alloc
   0,30%  attempt6  attempt6           [.] __rdl_dealloc
   0,22%  attempt6  attempt6           [.] __rdl_alloc
   0,01%  attempt6  [kernel.kallsyms]  [k] __perf_addr_filters_adjust
...
```

Hashing takes around one-third of the time, and around half is spent in bigint arithmetic and heap management. We aren't using heap in the hot loop save for a single `Box::new`, so that's due to bigint arithmetic too. Can we do anything about that?

We're using a generic bigint implementation, but we actually need just 25-byte bigints. That's just three 64-bit words plus a single byte. Let's implement long arithmetic ourselves. We don't need any fancy logic here, just the bare bones.

```rust expansible
#![feature(bigint_helper_methods)]

use std::ops::{Add, AddAssign, BitAnd, Mul, MulAssign, Not};

// 25 bytes, little-endian: 8 + 8 + 8 + 1
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq)]
struct u200(u64, u64, u64, u8);

impl u200 {
    fn to_be_bytes(self) -> [u8; 25] {
        let mut bytes = [0u8; 25];
        bytes[0] = self.3;
        bytes[1..9].copy_from_slice(&self.2.to_be_bytes());
        bytes[9..17].copy_from_slice(&self.1.to_be_bytes());
        bytes[17..].copy_from_slice(&self.0.to_be_bytes());
        bytes
    }
}

impl From<u64> for u200 {
    fn from(value: u64) -> u200 {
        u200(value, 0, 0, 0)
    }
}

impl Add for u200 {
    type Output = u200;
    fn add(self, other: u200) -> u200 {
        let (a, carry) = self.0.overflowing_add(other.0);
        let (b, carry) = self.1.carrying_add(other.1, carry);
        let (c, carry) = self.2.carrying_add(other.2, carry);
        let (d, _) = self.3.carrying_add(other.3, carry);
        u200(a, b, c, d)
    }
}

impl AddAssign for u200 {
    fn add_assign(&mut self, other: u200) {
        *self = *self + other;
    }
}

impl Mul<u64> for u200 {
    type Output = u200;
    fn mul(self, other: u64) -> u200 {
        let (a, carry) = self.0.widening_mul(other);
        let (b, carry) = self.1.carrying_mul(other, carry);
        let (c, carry) = self.2.carrying_mul(other, carry);
        let (d, _) = self.3.carrying_mul(other as u8, carry as u8);
        u200(a, b, c, d)
    }
}

impl MulAssign<u64> for u200 {
    fn mul_assign(&mut self, other: u64) {
        *self = *self * other;
    }
}

impl BitAnd for u200 {
    type Output = u200;
    fn bitand(self, other: u200) -> u200 {
        u200(
            self.0 & other.0,
            self.1 & other.1,
            self.2 & other.2,
            self.3 & other.3,
        )
    }
}

impl Not for u200 {
    type Output = u200;
    fn not(self) -> u200 {
        u200(!self.0, !self.1, !self.2, !self.3)
    }
}

// Replacing BigUint with u200 in the rest of the code is straightforward
```

```shell
$ time RUSTFLAGS="-C target-feature=+avx2" cargo run --bin attempt7 --release
    Finished `release` profile [optimized] target(s) in 0.01s
     Running `target/release/attempt7`
Found valid address: 18ryVioVmwFYzhRZKTjKqGYCjkUjoxH3k6

real    0m2,524s
user    0m2,512s
sys     0m0,012s
```

Wow! Not bad. perf says hashing now takes 70% of the time. Can we hash even faster?


### Eighth attempt

Well, sort of. These are quite a few scalar operations in our hashing code. We're loading inputs from memory to vector registers and then dumping outputs from vector registers, all without vectorization.

First, we're doing that even between the two rounds of SHA-256 used when computing the checksum. We dump the data after the first round and then load the very same data before the second round. We can keep using the same vector registers:

```rust
fn sha256_adapt_iterated(h: [u32x8; 8]) -> [u32x8; 16] {
    let mut w: [u32x8; 16] = [u32x8::splat(0); 16];
    w[..8].copy_from_slice(&h);
    w[8] = u32x8::splat(0x80000000);
    w[15] = u32x8::splat(256);
    w
}
```

After this optimization, we perform a complicated scalar store from h0..h7 to memory and only consume the leading four bytes stored in h0. This inefficiency was invisible before, but now we know we can optimize it out:

```rust
fn sha256_store_leading_four_bytes(h: [u32x8; 8]) -> [u32; 8] {
    h[0].to_array()
}
```

This still leaves us with one inefficient 21-byte load. We have:

```rust
inputs[0] = [a0, a1, a2, a3, a4, a5]
inputs[1] = [b0, b1, b2, b3, b4, b5]
inputs[2] = [c0, c1, c2, c3, c4, c5]
inputs[3] = [d0, d1, d2, d3, d4, d5]
inputs[4] = [e0, e1, e2, e3, e4, e5]
inputs[5] = [f0, f1, f2, f3, f4, f5]
inputs[6] = [g0, g1, g2, g3, g4, g5]
inputs[7] = [h0, h1, h2, h3, h4, h5]
```

We want:

```rust
w[0] = (a0, b0, c0, d0, e0, f0, g0, h0)
w[1] = (a1, b1, c1, d1, e1, f1, g1, h1)
w[2] = (a2, b2, c2, d2, e2, f2, g2, h2)
w[3] = (a3, b3, c3, d3, e3, f3, g3, h3)
w[4] = (a4, b4, c4, d4, e4, f4, g4, h4)
w[5] = (a5, b5, c5, d5, e5, f5, g5, h5)
```

That's just matrix transposition. We could perhaps emit six gather calls, but adapting a typical 8x8 transposition algorithm to our use case is probably going to be more efficient:

```rust expansible
fn sha256_load_six_words(vecs: [u8x32; 8], length_in_bits: u32) -> [u32x8; 16] {
    let mut vecs = vecs.map(|vec| u32x8::from_ne_bytes(vec.to_ne_bytes()));

    // vecs[0] = [a0, a1, a2, a3, a4, a5, ?, ?]
    // vecs[1] = [b0, b1, b2, b3, b4, b5, ?, ?]
    // vecs[2] = [c0, c1, c2, c3, c4, c5, ?, ?]
    // vecs[3] = [d0, d1, d2, d3, d4, d5, ?, ?]
    // vecs[4] = [e0, e1, e2, e3, e4, e5, ?, ?]
    // vecs[5] = [f0, f1, f2, f3, f4, f5, ?, ?]
    // vecs[6] = [g0, g1, g2, g3, g4, g5, ?, ?]
    // vecs[7] = [h0, h1, h2, h3, h4, h5, ?, ?]

    // Transpose 2x2 matrix of 4x4 elements
    for i in 0..4 {
        (vecs[i], vecs[i + 4]) = (
            simd_swizzle!(vecs[i], vecs[i + 4], [0, 1, 2, 3, 8, 9, 10, 11]),
            simd_swizzle!(vecs[i], vecs[i + 4], [4, 5, 6, 7, 12, 13, 14, 15]),
        );
    }

    // vecs[0] = [a0, a1, a2, a3, e0, e1, e2, e3]
    // vecs[1] = [b0, b1, b2, b3, f0, f1, f2, f3]
    // vecs[2] = [c0, c1, c2, c3, g0, g1, g2, g3]
    // vecs[3] = [d0, d1, d2, d3, h0, h1, h2, h3]
    // vecs[4] = [a4, a5, ?,  ?,  e4, e5, ?,  ? ]
    // vecs[5] = [b4, b5, ?,  ?,  f4, f5, ?,  ? ]
    // vecs[6] = [c4, c5, ?,  ?,  g4, g5, ?,  ? ]
    // vecs[7] = [d4, d5, ?,  ?,  h4, h5, ?,  ? ]

    // Transpose each 4x4 submatrix as a 2x2 matrix of 2x2 elements
    for i in 0..2 {
        (vecs[i], vecs[i + 2]) = (
            simd_swizzle!(vecs[i], vecs[i + 2], [0, 1, 8, 9, 4, 5, 12, 13]),
            simd_swizzle!(vecs[i], vecs[i + 2], [2, 3, 10, 11, 6, 7, 14, 15]),
        );
    }
    for i in 4..6 {
        vecs[i] = simd_swizzle!(vecs[i], vecs[i + 2], [0, 1, 8, 9, 4, 5, 12, 13]);
    }

    // vecs[0] = [a0, a1, c0, c1, e0, e1, g0, g1]
    // vecs[1] = [b0, b1, d0, d1, f0, f1, h0, h1]
    // vecs[2] = [a2, a3, c2, c3, e2, e3, g2, g3]
    // vecs[3] = [b2, b3, d2, d3, f2, f3, h2, h3]
    // vecs[4] = [a4, a5, c4, c5, e4, e5, g4, g5]
    // vecs[5] = [b4, b5, d4, d5, f4, f5, h4, h5]

    // Transpose each 2x2 submatrix
    for i in (0..6).step_by(2) {
        (vecs[i], vecs[i + 1]) = (
            simd_swizzle!(vecs[i], vecs[i + 1], [0, 8, 2, 10, 4, 12, 6, 14]),
            simd_swizzle!(vecs[i], vecs[i + 1], [1, 9, 3, 11, 5, 13, 7, 15]),
        );
    }

    // vecs[0] = [a0, b0, c0, d0, e0, f0, g0, h0]
    // vecs[1] = [a1, b1, c1, d1, e1, f1, g1, h1]
    // vecs[2] = [a2, b2, c2, d2, e2, f2, g2, h2]
    // vecs[3] = [a3, b3, c3, d3, e3, f3, g3, h3]
    // vecs[4] = [a4, b4, c4, d4, e4, f4, g4, h4]
    // vecs[5] = [a5, b5, c5, d5, e5, f5, g5, h5]

    let mut w: [u32x8; 16] = [u32x8::splat(0); 16];
    for i in 0..6 {
        w[i] = u32x8::from_be_bytes(vecs[i].to_ne_bytes());
    }
    w[15] = u32x8::splat(length_in_bits);
    w
}
```

We can merge these snippets together, strip `sha256_parallel_8` to its core, and use the logical blocks like this:

```rust
// snip

#[inline(always)]
fn sha256_core(leading_w: [u32x8; 16]) -> [u32x8; 8] {
    // snip

    let mut w: [u32x8; 64] = [u32x8::splat(0); 64];
    w[..16].copy_from_slice(&leading_w);

    // snip

    [h0, h1, h2, h3, h4, h5, h6, h7]
}

// snip

    fn flush(&mut self) {
        let mut inputs = [u8x32::splat(0); 8];
        for (input, (string, _)) in inputs.iter_mut().zip(&mut self.queue) {
            input[..21].copy_from_slice(string);
            input[21] = 0x80;
        }
        let round1 = sha256_core(sha256_load_six_words(inputs, 21 * 8));
        let round2 = sha256_store_leading_four_bytes(sha256_core(sha256_adapt_iterated(round1)));
        for (output, (_, callback)) in round2.into_iter().zip(self.queue.drain(..)) {
            callback(output);
        }
    }

// snip
```

```shell
$ time RUSTFLAGS="-C target-feature=+avx2" cargo run --bin attempt8 --release
    Finished `release` profile [optimized] target(s) in 0.01s
     Running `target/release/attempt8`
Found valid address: 18ryVioVmwFYzhRZKTjKqGYCjkUjoxH3k6

real    0m2,444s
user    0m2,424s
sys     0m0,020s
```

Bummer. That's just barely faster than the inefficient load/store implementation. Perhaps the slowdown is simply due to the deliberately sequential nature of SHA-256, and we can't really make it any faster.

I guess this is it! Let's see, we started with an implementation that takes *522 seconds* to decode the address `18ryviovmwfyzhrzktjkqgycjkujoxh3k6`, and now it can do that in under 2.5 seconds. That's more than a 200x improvement!

:::aside
Intel has provided AVX2 since Haswell (2013), and AMD has provided it since Excavator (2015).
:::

This comparison is not *entirely* sound, though. The original code assumed nothing more than an x86-64-capable processor, and the last attempt needs AVX2, which is unavailable on older CPUs. I own devices that don't support AVX2, so supporting pure x86-64 is reasonable.

<aside-inline-here />

Here's how our latest code behaves with AVX2 switched off:

```shell
$ time cargo run --bin attempt8 --release
    Finished `release` profile [optimized] target(s) in 0.01s
     Running `target/release/attempt8`
Found valid address: 18ryVioVmwFYzhRZKTjKqGYCjkUjoxH3k6

real    0m4,617s
user    0m4,604s
sys     0m0,012s
```

That's a ~2x reduction in performance, but at least the code still works!

It makes me wonder. If we dropped the "works on x86-64-v1" restriction, would some other extensions help us increase performance? It turns out there are [Intel SHA extensions](https://en.wikipedia.org/wiki/Intel_SHA_extensions), which contain hardware implementations of SHA-256! Can we use them?

```shell
$ grep sha_ni /proc/cpuinfo
```

Not on this device. Here, let me ssh into an AMD Zen VPS.

```shell
> grep sha_ni /proc/cpuinfo
flags       : fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ht syscall nx mmxext fxsr_opt pdpe1gb rdtscp lm rep_good nopl cpuid extd_apicid tsc_known_freq pni pclmulqdq ssse3 fma cx16 sse4_1 sse4_2 x2apic movbe popcnt aes xsave avx f16c rdrand hypervisor lahf_lm cmp_legacy cr8_legacy abm sse4a misalignsse 3dnowprefetch osvw topoext perfctr_core ssbd ibrs ibpb stibp vmmcall fsgsbase bmi1 avx2 smep bmi2 rdseed adx smap clflushopt clwb sha_ni xsaveopt xsavec xgetbv1 clzero xsaveerptr wbnoinvd arat umip rdpid arch_capabilities
flags       : fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ht syscall nx mmxext fxsr_opt pdpe1gb rdtscp lm rep_good nopl cpuid extd_apicid tsc_known_freq pni pclmulqdq ssse3 fma cx16 sse4_1 sse4_2 x2apic movbe popcnt aes xsave avx f16c rdrand hypervisor lahf_lm cmp_legacy cr8_legacy abm sse4a misalignsse 3dnowprefetch osvw topoext perfctr_core ssbd ibrs ibpb stibp vmmcall fsgsbase bmi1 avx2 smep bmi2 rdseed adx smap clflushopt clwb sha_ni xsaveopt xsavec xgetbv1 clzero xsaveerptr wbnoinvd arat umip rdpid arch_capabilities
flags       : fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ht syscall nx mmxext fxsr_opt pdpe1gb rdtscp lm rep_good nopl cpuid extd_apicid tsc_known_freq pni pclmulqdq ssse3 fma cx16 sse4_1 sse4_2 x2apic movbe popcnt aes xsave avx f16c rdrand hypervisor lahf_lm cmp_legacy cr8_legacy abm sse4a misalignsse 3dnowprefetch osvw topoext perfctr_core ssbd ibrs ibpb stibp vmmcall fsgsbase bmi1 avx2 smep bmi2 rdseed adx smap clflushopt clwb sha_ni xsaveopt xsavec xgetbv1 clzero xsaveerptr wbnoinvd arat umip rdpid arch_capabilities
flags       : fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ht syscall nx mmxext fxsr_opt pdpe1gb rdtscp lm rep_good nopl cpuid extd_apicid tsc_known_freq pni pclmulqdq ssse3 fma cx16 sse4_1 sse4_2 x2apic movbe popcnt aes xsave avx f16c rdrand hypervisor lahf_lm cmp_legacy cr8_legacy abm sse4a misalignsse 3dnowprefetch osvw topoext perfctr_core ssbd ibrs ibpb stibp vmmcall fsgsbase bmi1 avx2 smep bmi2 rdseed adx smap clflushopt clwb sha_ni xsaveopt xsavec xgetbv1 clzero xsaveerptr wbnoinvd arat umip rdpid arch_capabilities
```


### Ninth attempt

SHA-NI provides three instructions:

- `sha256msg1`
- `sha256msg2`
- `sha256rnds2`

The first two instructions optimize this loop:

```rust
for i in 16..64 {
    let s0 = rotate_right(w[i - 15], 7) ^ rotate_right(w[i - 15], 18) ^ (w[i - 15] >> 3);
    let s1 = rotate_right(w[i - 2], 17) ^ rotate_right(w[i - 2], 19) ^ (w[i - 2] >> 10);
    w[i] = w[i - 16] + s0 + w[i - 7] + s1;
}
```

`sha256msg1` computes `w[i - 16] + s0` for four consecutive `i`s. `sha256msg2` *would* compute `w[i - 7] + s1` for four consecutive `i`s, if only `w[i]` didn't depend on `w[i - 2]`, so instead of computes `w[i]` given the result of `sha256msg1` and the previous values of `w`. Here is how they are meant to be used, roughly speaking:

```rust
for i in (16..64).step_by(4) {
    w[i..i + 4] = sha256msg2(sha256msg1(w[i - 16..i - 12], w[i - 12..i - 8]) + w[i - 7..i - 3], w[i - 4..i]);
}
```

`sha256rnds2` performs two rounds of the main loop of SHA-256, meaning that this:

```rust
for i in (0..64).step_by(8) {
    compress(a, b, c, &mut d, e, f, g, &mut h, K[i], w[i]);
    compress(h, a, b, &mut c, d, e, f, &mut g, K[i + 1], w[i + 1]);
    compress(g, h, a, &mut b, c, d, e, &mut f, K[i + 2], w[i + 2]);
    compress(f, g, h, &mut a, b, c, d, &mut e, K[i + 3], w[i + 3]);
    compress(e, f, g, &mut h, a, b, c, &mut d, K[i + 4], w[i + 4]);
    compress(d, e, f, &mut g, h, a, b, &mut c, K[i + 5], w[i + 5]);
    compress(c, d, e, &mut f, g, h, a, &mut b, K[i + 6], w[i + 6]);
    compress(b, c, d, &mut e, f, g, h, &mut a, K[i + 7], w[i + 7]);
}
```

Can be replaced by approximately this:

```rust
for i in (0..64).step_by(8) {
    sha256rnds2(a, b, &mut c, &mut d, e, f, &mut g, &mut h, K[i] + w[i], K[i + 1] + w[i + 1]);
    sha256rnds2(g, h, &mut a, &mut b, c, d, &mut e, &mut f, K[i + 2] + w[i + 2], K[i + 3] + w[i + 3]);
    sha256rnds2(e, f, &mut g, &mut h, a, b, &mut c, &mut d, K[i + 4] + w[i + 4], K[i + 5] + w[i + 5]);
    sha256rnds2(c, d, &mut e, &mut f, g, h, &mut a, &mut b, K[i + 6] + w[i + 6], K[i + 7] + w[i + 7]);
}
```

:::aside
Roughly speaking, an instruction can run on several ports. You ask a port to compute the instruction and then have to wait. Latency is how many cycles pass till the output is available. Throughput is how many cycles pass till you can send another instruction to this port. This is the reciprocal of what people *usually* call throughput, so it's sometimes (but not always) called rthroughput.
:::

This optimization looks like a no-brainer, but there is a pitfall: SHA-NI is used to optimize the hashing of a single message, so while the instructions use xmm registers, that is just a hardware way of saying, "I want lots of integers as inputs to this instruction". The only way to compute several hashes in parallel is to interleave the instructions that hash different strings, thanks to pipelined execution.

<aside-inline-here />

How many hashes should we compute in parallel? [uops.info](https://uops.info/table.html) says `sha256rnds2 (xmm, xmm)` has latency 4 and throughput 2 on Zen, so only two `sha256rnds2`s can be executed simultaneously. Other instructions, like `sha256msg1`, have different latencies and throughputs, but `sha256rnds2` is likely the bottleneck. This means that we should probably run only two SHA-256 chains.

Let's see how our original code performs on this machine so that we have some baseline for comparison:

```shell
> time RUSTFLAGS="-C target-feature=+avx2" cargo run --bin attempt8 --release
    Finished `release` profile [optimized] target(s) in 0.01s
     Running `target/release/attempt8`
Found valid address: 18ryVioVmwFYzhRZKTjKqGYCjkUjoxH3k6

________________________________________________________
Executed in    2.88 secs    fish           external
   usr time    2.85 secs  268.00 micros    2.85 secs
   sys time    0.02 secs  253.00 micros    0.02 secs
```

And here's the code that uses SHA-NI. It should be more or less straightforward. I hope you excuse me for a bit of unsafety here :-)

```rust expansible
fn sha256_load_six_words<const N: usize>(input: [u8; N]) -> [u32x4; 4] {
    let (chunks, trailing) = input.as_chunks::<4>();
    let mut w = [0u32; 16];
    for (wi, chunk) in w.iter_mut().zip(chunks) {
        *wi = u32::from_be_bytes(*chunk);
    }
    let mut trailing4 = [0u8; 4];
    trailing4[..trailing.len()].copy_from_slice(trailing);
    trailing4[trailing.len()] = 0x80;
    w[chunks.len()] = u32::from_be_bytes(trailing4);
    w[15] = (N * 8) as u32;
    unsafe { std::mem::transmute(w) }
}

fn sha256_adapt_iterated(h: (u32x4, u32x4)) -> [u32x4; 4] {
    let (h0145, h2367) = h;
    [
        simd_swizzle!(h0145, h2367, [3, 2, 7, 6]),
        simd_swizzle!(h0145, h2367, [1, 0, 5, 4]),
        u32x4::from_array([0x80000000, 0, 0, 0]),
        u32x4::from_array([0, 0, 0, 256]),
    ]
}

#[inline(always)]
#[unroll_for_loops]
fn sha256_core<const N: usize>(leading_w: [[u32x4; 4]; N]) -> [(u32x4, u32x4); N] {
    // Initialization
    const H0145: u32x4 = u32x4::from_array([0x9b05688c, 0x510e527f, 0xbb67ae85, 0x6a09e667]);
    const H2367: u32x4 = u32x4::from_array([0x5be0cd19, 0x1f83d9ab, 0xa54ff53a, 0x3c6ef372]);

    const K: [u32x4; 16] = [
        u32x4::from_array([0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5]),
        u32x4::from_array([0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5]),
        u32x4::from_array([0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3]),
        u32x4::from_array([0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174]),
        u32x4::from_array([0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc]),
        u32x4::from_array([0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da]),
        u32x4::from_array([0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7]),
        u32x4::from_array([0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967]),
        u32x4::from_array([0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13]),
        u32x4::from_array([0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85]),
        u32x4::from_array([0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3]),
        u32x4::from_array([0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070]),
        u32x4::from_array([0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5]),
        u32x4::from_array([0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3]),
        u32x4::from_array([0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208]),
        u32x4::from_array([0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2]),
    ];

    // Initialize working variables to current hash value
    let mut abef = [H0145; N];
    let mut cdgh = [H2367; N];

    let mut four_rounds = |i, j: usize, wij| {
        let kwij: u32x4 = K[i] + wij;
        unsafe {
            cdgh[j] = _mm_sha256rnds2_epu32(cdgh[j].into(), abef[j].into(), kwij.into()).into();
            abef[j] = _mm_sha256rnds2_epu32(
                abef[j].into(),
                cdgh[j].into(),
                simd_swizzle!(kwij, [2, 3, 2, 3]).into(),
            )
            .into();
        }
    };

    let mut w = [[u32x4::splat(0); N]; 16];
    for i in 0..4 {
        for j in 0..N {
            w[i][j] = leading_w[j][i];
            four_rounds(i, j, w[i][j]);
        }
    }

    // Extend the first 16 words into the remaining 48 words w[16..64] of the message schedule array
    // & Compression function main loop
    unsafe {
        unroll! {
            for i in 4..16 {
                for j in 0..N {
                    w[i][j] = _mm_sha256msg2_epu32(
                        (u32x4::from(_mm_sha256msg1_epu32(w[i - 4][j].into(), w[i - 3][j].into()))
                            + simd_swizzle!(w[i - 2][j], w[i - 1][j], [1, 2, 3, 4]))
                        .into(),
                        w[i - 1][j].into(),
                    )
                    .into();
                    four_rounds(i, j, w[i][j]);
                }
            }
        }
    }

    // Add the compressed chunk to the current hash value
    let mut output = [(u32x4::splat(0), u32x4::splat(0)); N];
    for i in 0..N {
        output[i] = (H0145 + abef[i], H2367 + cdgh[i]);
    }
    output
}

fn sha256_store_leading_four_bytes(h: (u32x4, u32x4)) -> u32 {
    h.0[3]
}

struct ParallelChecksum<'a> {
    queue: ArrayVec<([u8; 21], Box<dyn FnOnce(u32) + 'a>), { ParallelChecksum::PARALLELISM }>,
}

impl<'a> ParallelChecksum<'a> {
    const PARALLELISM: usize = 2;

    // snip

    fn flush(&mut self) {
        let mut inputs = [[u32x4::splat(0); 4]; Self::PARALLELISM];
        for (input, (string, _)) in inputs.iter_mut().zip(&mut self.queue) {
            *input = sha256_load_six_words(*string);
        }
        let round1 = sha256_core(inputs);
        let round2 =
            sha256_core(round1.map(sha256_adapt_iterated)).map(sha256_store_leading_four_bytes);
        for (output, (_, callback)) in round2.into_iter().zip(self.queue.drain(..)) {
            callback(output);
        }
    }

    // snip
}
```

Let's see if this was a good idea!

```shell
> time RUSTFLAGS="-C target-feature=+avx2,+sha" cargo run --bin attempt9 --release
    Finished `release` profile [optimized] target(s) in 0.02s
     Running `target/release/attempt9`
Found valid address: 18ryVioVmwFYzhRZKTjKqGYCjkUjoxH3k6

________________________________________________________
Executed in    1.60 secs    fish           external
   usr time    1.58 secs  341.00 micros    1.58 secs
   sys time    0.02 secs  273.00 micros    0.02 secs
```

:::aside
I have replicated the same speedup on Zen 4 later.
:::

This is a 1.8x speedup -- not quite exciting, but an improvement nonetheless. Also, `PARALLELISM == 2` turns out to indeed be the best value.

<aside-inline-here />

Some other improvements are possible, like replacing recursion with Gray code and abolishing vtables and heap allocation. But this post is already getting long, and we will not be able to get a greater than 1.5x improvement this way, so let's call it quits. It's faster than anything you will need anyway: the longest possible all-letters address, `1AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA`, can be recovered to `1AAAaAAaaAAaAaaaaAAAaAAAaAaaaAAAaa` in 27 seconds.


### Results

We have applied the following optimizations:

- Rewrite it in Rust
- Replace base58 parsing with simple arithmetic `5x`
- Replace a product loop with recursion `1.15x`
- Compute the right cases without guessing on lower recursion levels `6x`
- Vectorize SHA-256 computation `2.3x`
- Use a specialized long arithmetic implementation `2.5x`
- Use somewhat smarter vectorized algorithms `1.05x`
- Use SHA-NI if available `1.8x`

This is a ~300-400x speedup. Of these, the first four are theory-based optimizations, yielding an improvement of 34x, and the latter four utilize hardware better, further improving performance by 11x.

---

So, if you think you do not need to know computer science or math to write good software, this is bullshit.

Is parsing integers complicated math? Not really. Do you need *some* mathematical background to optimize base58 parsing the way we did? Probably.

:::aside
You got me, Gray code-based iteration does not require recursion and is even faster. My point stands.
:::

Do you need to know compsci to write a recursive algorithm? No. Do you need some intuition that compsci provides to realize recursion is sometimes more efficient than a loop? I think so.

<aside-inline-here />

Do you need to know math to notice the leading 21 bytes stop changing at some point? No. Do you need to know a trick or two to terminate recursion at that point instead of just caching the hash and going on? I think so.

It's not rocket science, yet people keep making the same mistakes. Rust's `async`s have $O(n)$ continuation complexity instead of $O(1)$, which forces `tokio` to workaround this inefficiency with various hacks. Developers keep choosing the most inefficient data structures or sticking to a terrible language (for their task, not generally) just because they are familiar with it. Maybe we should be a bit more mindful of these choices.

---

There is another lesson here. We often try to throw more workforce at the problem as if that is magically going to hide the inefficiency. But the hardware is often powerful enough, and the problem is that we've never learnt to make use of it.

I once heard an anecdote that stuck with me. A person noticed `free` showing little free memory and thought Linux was stupid and nasty for hogging all the memory. But Linux did not, in fact, use all the memory for permanent data. It kept a large disk cache, which could safely be dropped if some program needed more memory than what was available at the time. Linux said, "No one is using this memory at the moment anyway, so why don't I borrow it to increase I/O performance?"

The same applies to CPU use, at least on servers. If your services always use below 10% of CPU resources, you should consider switching to cheaper hardware or configuring the software to use the CPU more aggressively.

But what *is* CPU use? All mainstream operating systems interpret CPU use as the fraction of time a process executes an instruction on the CPU, ignoring the microarchitectural conditions. Both scalar and vectorized code use 100% of CPU but utilize different CPU ports with different efficiencies, so *should* we really be saying they utilize the same fraction of CPU resources?

I don't think so. Maybe we should invent a technical term for this closer-to-reality value. Ideas, anyone?
