#![feature(array_chunks, portable_simd)]

use arrayvec::ArrayVec;
use num_bigint::BigUint;
use num_traits::{One, Zero};
use std::simd::u32x8;

const BASE58_ALPHABET: &'static [u8] =
    b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

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
        if self.queue.len() == 8 {
            self.flush();
        }
        self.queue.push((string, callback));
    }

    fn finalize(mut self) {
        self.flush();
    }
}

fn iterate_through_possible_differences<'a>(
    partly_fixed_number: &BigUint,
    possible_differences_tail: &'a [(usize, BigUint)],
    suffix_sums_of_possible_differences_tail: &[BigUint],
    mut subset_bitmask: u64,
    base_address: &'a Vec<u8>,
    possible_differences: &'a [(usize, BigUint)],
    parallel_checksum: &mut ParallelChecksum<'a>,
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
            parallel_checksum,
        );

        // No
        iterate_through_possible_differences(
            partly_fixed_number,
            &possible_differences_tail[1..],
            &suffix_sums_of_possible_differences_tail[1..],
            subset_bitmask,
            base_address,
            possible_differences,
            parallel_checksum,
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

    let mut current_checksum =
        u32::from_be_bytes(*byte_string_25[21..].first_chunk::<4>().unwrap());

    parallel_checksum.compute_later(
        *byte_string_25.first_chunk::<21>().unwrap(),
        Box::new(move |correct_checksum| {
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
        }),
    );
}

fn main() {
    let garbled_address = "18ryviovmwfyzhrzktjkqgycjkujoxh3k6";

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

    let mut parallel_checksum = ParallelChecksum::new();

    iterate_through_possible_differences(
        &parsed_number,
        &possible_differences,
        &suffix_sums_of_possible_differences,
        0,
        &base_address,
        &possible_differences,
        &mut parallel_checksum,
    );

    parallel_checksum.finalize();
}
