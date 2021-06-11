use nanos_sdk::bindings::*;
use nanos_sdk::ecc::{CurvesId, DerEncodedEcdsaSignature};
use nanos_sdk::io::SyscallError;

pub const BIP32_PATH: [u32; 5] = nanos_sdk::ecc::make_bip32_path(b"m/44'/535348'/0'/0/0");

/// Helper function that derives the seed over secp256k1
pub fn bip32_derive_secp256k1(path: &[u32]) -> Result<[u8; 32], SyscallError> {
    let mut raw_key = [0u8; 32];
    nanos_sdk::ecc::bip32_derive(CurvesId::Secp256k1, path, &mut raw_key)?;
    Ok(raw_key)
}

/// Helper function that signs with ECDSA in deterministic nonce,
/// using SHA256
pub fn detecdsa_sign(
    m: &[u8],
    ec_k: &cx_ecfp_private_key_t,
) -> Option<(DerEncodedEcdsaSignature, u32)> {
    nanos_sdk::ecc::ecdsa_sign(ec_k, CX_RND_RFC6979 | CX_LAST, CX_SHA256, m)
}

pub fn get_pubkey() -> Result<nanos_sdk::bindings::cx_ecfp_public_key_t, SyscallError> {
    let raw_key = bip32_derive_secp256k1(&BIP32_PATH)?;
    let mut ec_k = nanos_sdk::ecc::ec_init_key(CurvesId::Secp256k1, &raw_key)?;
    nanos_sdk::ecc::ec_get_pubkey(CurvesId::Secp256k1, &mut ec_k)
}

pub fn get_private_key() -> Result<nanos_sdk::bindings::cx_ecfp_private_key_t, SyscallError> {
    let raw_key = bip32_derive_secp256k1(&BIP32_PATH)?;
    nanos_sdk::ecc::ec_init_key(CurvesId::Secp256k1, &raw_key)
}
