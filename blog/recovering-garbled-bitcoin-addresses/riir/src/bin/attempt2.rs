use base58::FromBase58;
use itertools::Itertools;
use sha2::{digest::Update, Digest, Sha256};

fn main() {
    let address_lowercase = "18ryviovmwfyzhrzktjkqgycjkujoxh3k6";

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
