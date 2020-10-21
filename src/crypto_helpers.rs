use nanos_sdk::bindings::*;
use nanos_sdk::ecc::{CurvesId, DEREncodedECDSASignature};

/// ASCII value of the character 0.
const ZERO: u8 = 48;
/// ASCII value of the character 9.
const NINE: u8 = 57;

/// Helper macro that creates an array from the ASCII values of a correctly formatted derivation path.
/// Format expected: `b"44'/coin_type'/account'/change/address"`.
///
/// # Panics
///
/// Panics if the parameter does not follow the correct format.
macro_rules! make_bip32_path {
    ($bytes:expr) => {{
        // The three first elements must start with `0x800000`.
        let mut path = [0x80000000, 0x80000000, 0x80000000, 0, 0];
        let mut i = 0;
        let mut j = 0;
        let mut acc = 0u32;

        // We are looking for 5 numbers, separated by `/`.
        // Those numbers are represented in ASCII bytes (e.g `[49, 48, 51]` represents the number `103`).
        // We are going to parse the string once, summing the bytes when we encounter them to create a number
        // and resetting our counter everytime we get to a separator (i.e. a byte that does not represent an ASCII number).
        while (j < path.len()) {
            // Check if this byte represents a number in ASCII.
            while (i < $bytes.len() && $bytes[i] >= ZERO && $bytes[i] <= NINE) {
                // It does: add it to the accumulator (taking care to substract the ASCII value of 0).
                acc = acc * 10 + $bytes[i] as u32 - ZERO as u32;
                i += 1;
            }
            // We've effectively parsed a number: add it to `path`.
            path[j] += acc;
            // Reset the accumulator.
            acc = 0;
            // Keep going until we either:
            // 1. Find a new number.
            // 2. Reach the end of the bytes.
            while (i < $bytes.len() && ($bytes[i] <= ZERO || $bytes[i] >= NINE)) {
                i += 1;
            }
            // Repeat that for the next element in `path`.
            j += 1;
        }
        path 
    }};
}

pub const BIP32_PATH: [u32; 5] = make_bip32_path!(b"44'/535348'/0'/0/0");

/// Helper function that derives the seed over secp256k1
pub fn bip32_derive_secp256k1(path: &[u32]) -> [u8; 32] {
    let mut raw_key = [0u8; 32];
    nanos_sdk::ecc::bip32_derive(CurvesId::Secp256k1, path, &mut raw_key);
    raw_key
} 

/// Helper function that signs with ECDSA in deterministic nonce,
/// using SHA256
pub fn detecdsa_sign(m: &[u8], ec_k: &cx_ecfp_private_key_t) -> Result<(DEREncodedECDSASignature, i32), ()> {
    nanos_sdk::ecc::ecdsa_sign(ec_k,
        (CX_RND_RFC6979 | CX_LAST) as i32,
        CX_SHA256,
        m)
}

/// Helper function that verifies a signature produced with `detecdsa_sign`
pub fn detecdsa_verify(m: &[u8], sig: &[u8], ec_pubk: &cx_ecfp_public_key_t) -> bool {
    nanos_sdk::ecc::ecdsa_verify(&ec_pubk, 
                sig, 
          (CX_RND_RFC6979 | CX_LAST) as i32,
       CX_SHA256,
            &m)
}

pub fn get_pubkey() -> nanos_sdk::bindings::cx_ecfp_public_key_t {
    let raw_key = bip32_derive_secp256k1(&BIP32_PATH);
    let mut ec_k = nanos_sdk::ecc::ec_init_key(CurvesId::Secp256k1, &raw_key);
    nanos_sdk::ecc::ec_get_pubkey(CurvesId::Secp256k1, &mut ec_k)
}

pub fn get_private_key() -> nanos_sdk::bindings::cx_ecfp_private_key_t {
    let raw_key = bip32_derive_secp256k1(&BIP32_PATH);
    nanos_sdk::ecc::ec_init_key(CurvesId::Secp256k1, &raw_key)
}
