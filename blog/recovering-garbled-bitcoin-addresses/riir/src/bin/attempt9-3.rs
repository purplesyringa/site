#![feature(array_chunks, bigint_helper_methods, portable_simd, slice_as_chunks)]

use arrayvec::ArrayVec;
use core::arch::x86_64::{_mm_sha256msg1_epu32, _mm_sha256msg2_epu32, _mm_sha256rnds2_epu32};
use crunchy::unroll;
use std::ops::{Add, AddAssign, BitAnd, Mul, MulAssign, Not};
use std::simd::{simd_swizzle, u32x4};

const BASE58_ALPHABET: &'static [u8] =
    b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

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
    const PARALLELISM: usize = 3;

    fn new() -> Self {
        Self {
            queue: ArrayVec::new(),
        }
    }

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

    fn compute_later(&mut self, string: [u8; 21], callback: Box<dyn FnOnce(u32) + 'a>) {
        if self.queue.is_full() {
            self.flush();
        }
        self.queue.push((string, callback));
    }

    fn finalize(mut self) {
        self.flush();
    }
}

fn iterate_through_possible_differences<'a>(
    partly_fixed_number: u200,
    possible_differences_tail: &'a [(usize, u200)],
    suffix_sums_of_possible_differences_tail: &[u200],
    mut subset_bitmask: u64,
    base_address: &'a Vec<u8>,
    possible_differences: &'a [(usize, u200)],
    parallel_checksum: &mut ParallelChecksum<'a>,
) {
    let interval_length = suffix_sums_of_possible_differences_tail[0];
    // Regardless of our further choices, the final fixed_number will be in range
    // [partly_fixed_number, partly_fixed_number + interval_length]. Do all the numbers in this
    // range have identical leading 21 bytes?

    if partly_fixed_number & !u200::from((1 << 32) - 1)
        != (partly_fixed_number + interval_length) & !u200::from((1 << 32) - 1)
    {
        // No, they don't. This means we have to guess whether to add the current fixup.
        let (digit_index, fixup_to_add) = &possible_differences_tail[0];

        // Yes
        iterate_through_possible_differences(
            partly_fixed_number + *fixup_to_add,
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
    let byte_string_25 = partly_fixed_number.to_be_bytes();

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
                let fixup_to_add = fixup_to_add.0 as u32;
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

    let mut parsed_number: u200 = 0.into();
    let mut power_58_i: u200 = 1.into();
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
                parsed_number += power_58_i * digit1 as u64;
                base_address[digit_index] = byte.to_ascii_uppercase();
                // digit1 is uppercase, digit2 is lowercase, lowercase comes after uppercase in the
                // alphabet, so the difference is positive
                possible_differences.push((digit_index, power_58_i * (digit2 - digit1) as u64));
            }
            (Some(digit), _) => {
                // Just the first variant is right
                parsed_number += power_58_i * digit as u64;
                base_address[digit_index] = byte.to_ascii_uppercase();
            }
            (_, Some(digit)) => {
                // Just the second variant is right
                parsed_number += power_58_i * digit as u64;
                base_address[digit_index] = byte.to_ascii_lowercase();
            }
            (None, None) => panic!("The address is invalid"),
        }
        power_58_i *= 58;
    }

    // Recurse from left to right, i.e. from most significant to least significant
    possible_differences.reverse();

    // For differences A, B, C, computes [A + B + C, B + C, C, 0].
    let mut suffix_sums_of_possible_differences: Vec<u200> = possible_differences
        .iter()
        .rev()
        .scan(0.into(), |state, (_, fixup)| {
            *state += *fixup;
            Some(*state)
        })
        .collect();
    suffix_sums_of_possible_differences.reverse();
    suffix_sums_of_possible_differences.push(0.into());

    let mut parallel_checksum = ParallelChecksum::new();

    iterate_through_possible_differences(
        parsed_number,
        &possible_differences,
        &suffix_sums_of_possible_differences,
        0,
        &base_address,
        &possible_differences,
        &mut parallel_checksum,
    );

    parallel_checksum.finalize();
}
