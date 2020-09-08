use nanos_sdk::bindings::*;
use nanos_sdk::ecc::{CurvesId, DEREncodedECDSASignature};

// TODO: make a macro that derives the path by itself
// ex: make_bip32_path!(b"44'/535348'/0'/0/0";
// =>
pub const BIP32_PATH: [u32; 5] = [
        0x8000002cu32,
        0x80082b34, 
        0x80000000, 
        0x00000000, 
        0x00000000];

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
