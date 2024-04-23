use num_bigint::BigUint;
use num_traits::{One, Zero};
use sha2::{digest::Update, Digest, Sha256};

const BASE58_ALPHABET: &'static [u8] =
    b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

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

    iterate_through_possible_differences(
        &parsed_number,
        &possible_differences,
        0,
        &base_address,
        &possible_differences,
    );
}
