use num_bigint::BigUint;
use num_traits::{One, Zero};
use sha2::{digest::Update, Digest, Sha256};

const BASE58_ALPHABET: &'static [u8] =
    b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

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

    iterate_through_possible_differences(
        &parsed_number,
        &possible_differences,
        &suffix_sums_of_possible_differences,
        0,
        &base_address,
        &possible_differences,
    );
}
