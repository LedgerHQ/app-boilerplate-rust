use alloc::vec::Vec;

use crate::AppSW;
use ledger_device_sdk::ecc::{Secp256k1, SeedDerive};
use ledger_device_sdk::hash::{sha3::Keccak256, HashInit};

/// BIP32 derivation path stored as a vector of u32 components.
///
/// Each component represents one level in the path (e.g., m/44'/1'/0'/0/0 has 5 components).
/// Hardened derivation is indicated by setting the high bit (>= 0x80000000).
#[derive(Default)]
pub struct Bip32Path(Vec<u32>);

impl AsRef<[u32]> for Bip32Path {
    fn as_ref(&self) -> &[u32] {
        &self.0
    }
}

impl TryFrom<&[u8]> for Bip32Path {
    type Error = AppSW;

    /// Constructs a [`Bip32Path`] from APDU-encoded bytes.
    ///
    /// # Format
    ///
    /// - First byte: Number of path components (e.g., 5 for m/44'/1'/0'/0/0)
    /// - Remaining bytes: Big-endian u32 components (4 bytes each)
    ///
    /// # Example
    ///
    /// For path m/44'/1'/0'/0/0:
    /// ```text
    /// [0x05, 0x8000002C, 0x80000001, 0x80000000, 0x00000000, 0x00000000]
    /// ```
    ///
    /// # Note
    ///
    /// This uses `Vec` for dynamic allocation, which is fine for normal APDU handlers
    /// but CANNOT be used in swap's `check_address` or `get_printable_amount` due to
    /// BSS memory sharing with the Exchange app.
    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        // Check data length
        if data.is_empty() // At least the length byte is required
            || (data[0] as usize * 4 != data.len() - 1)
        {
            return Err(AppSW::WrongApduLength);
        }

        Ok(Bip32Path(
            data[1..]
                .chunks(4)
                .map(|chunk| u32::from_be_bytes(chunk.try_into().unwrap()))
                .collect(),
        ))
    }
}

/// Derive the raw public key from a BIP32 path.
///
/// Returns the uncompressed secp256k1 public key (65 bytes):
/// - First byte: 0x04 (uncompressed marker)
/// - Next 32 bytes: X coordinate
/// - Last 32 bytes: Y coordinate
///
/// # Used by
///
/// - `handler_get_public_key`: Returns this raw pubkey to the client
/// - Internally for address computation
///
/// # Arguments
///
/// * `path` - BIP32 derivation path
///
/// # Returns
///
/// 65-byte uncompressed public key or error
pub fn get_pubkey_from_path(path: &Bip32Path) -> Result<[u8; 65], AppSW> {
    let (k, _) = Secp256k1::derive_from(path.as_ref());
    let pk = k.public_key().map_err(|_| AppSW::KeyDeriveFail)?;
    Ok(pk.pubkey)
}

/// Compute Keccak256 hash of a public key for address derivation.
///
/// This is used for Ethereum-style address computation:
/// 1. Take uncompressed pubkey (65 bytes)
/// 2. Skip first byte (0x04 marker)
/// 3. Hash the remaining 64 bytes with Keccak256
/// 4. Take last 20 bytes as address
///
/// # Used by
///
/// - `handler_get_public_key`: For displaying address to user
/// - `swap::check_address`: For verifying address ownership
///
/// # Arguments
///
/// * `pubkey` - 65-byte uncompressed secp256k1 public key
///
/// # Returns
///
/// 32-byte Keccak256 hash (last 20 bytes are the Ethereum address)
pub fn get_address_hash_from_pubkey(pubkey: &[u8; 65]) -> [u8; 32] {
    let mut keccak256 = Keccak256::new();
    let mut address: [u8; 32] = [0u8; 32];
    // Hash pubkey excluding first byte (0x04 uncompressed marker)
    let _ = keccak256.hash(&pubkey[1..], &mut address);
    address
}
