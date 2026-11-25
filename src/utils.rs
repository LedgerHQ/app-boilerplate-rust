use alloc::vec::Vec;

use crate::AppSW;
use ledger_device_sdk::ecc::{Secp256k1, SeedDerive};
use ledger_device_sdk::hash::{sha3::Keccak256, HashInit};
use ledger_device_sdk::libcall::string::{uint256_to_float, CustomString};

/// Ticker of the coin handled natively by this app.
pub const NATIVE_TICKER: &str = "CRAB";
/// Decimals of the coin handled natively by this app.
pub const NATIVE_DECIMALS: u8 = 9;

/// Bound on asset decimals, well within what the 79-byte output of `uint256_to_float` allows.
pub const MAX_DECIMALS: u8 = 30;

/// Longest string [`format_amount`] returns: `u64::MAX` has 20 digits, and an amount below one
/// unit is written `0.` followed by all its decimals.
pub const MAX_AMOUNT_LEN: usize = {
    let integer_len = u64::MAX.ilog10() as usize + 1;
    let with_point = integer_len + 1;
    let below_one_unit = 2 + MAX_DECIMALS as usize;
    if with_point > below_one_unit {
        with_point
    } else {
        below_one_unit
    }
};

/// Formats an amount of base units in asset units, e.g. `0.000000777` for 777 with 9 decimals.
///
/// Returns `None` if `decimals` exceeds [`MAX_DECIMALS`].
pub fn format_amount(value: u64, decimals: u8) -> Option<CustomString<79>> {
    if decimals > MAX_DECIMALS {
        return None;
    }
    let mut value_u256 = [0u8; 32];
    value_u256[32 - core::mem::size_of::<u64>()..].copy_from_slice(&value.to_be_bytes());
    Some(uint256_to_float(&value_u256, usize::from(decimals)))
}

/// Application name that signed token descriptors must carry, which is the `name` of the
/// `[package.metadata.ledger]` section of `Cargo.toml` for the variant being built. A fork that
/// renames the app or its variants must update it.
#[cfg(not(any(feature = "variant_testnet", feature = "variant_betanet")))]
pub const DESCRIPTOR_APP_NAME: &str = "Rust Boilerplate";
#[cfg(feature = "variant_testnet")]
pub const DESCRIPTOR_APP_NAME: &str = "Rust Testnet BP";
#[cfg(feature = "variant_betanet")]
pub const DESCRIPTOR_APP_NAME: &str = "Rust Betanet BP";

/// SLIP-0044 coin type of this app, the testnet one shared by all coins.
pub const SLIP44_COIN_TYPE: u32 = 1;
/// Flag marking a hardened BIP32 path component.
pub const HARDENED: u32 = 0x8000_0000;
/// Prefix of every path this app derives keys under, `m/44'/1'`, matching the `path` declared in
/// the `[package.metadata.ledger]` section of `Cargo.toml`.
const ALLOWED_PATH_PREFIX: [u32; 2] = [44 | HARDENED, SLIP44_COIN_TYPE | HARDENED];
/// Deepest path the OS derives (`MAX_BIP32_PATH` in the C SDK).
pub const MAX_PATH_LEN: usize = 10;

/// Whether `path` lies within the derivation scope of this app.
///
/// The OS also refuses derivations outside the scope declared at installation, but by aborting
/// the app: checking beforehand turns such requests into a proper error status.
pub fn is_allowed_path(path: &[u32]) -> bool {
    path.len() <= MAX_PATH_LEN && path.starts_with(&ALLOWED_PATH_PREFIX)
}

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

/// Formats the path in the usual `m/44'/1'/0'/0/0` notation, hardened components marked with `'`.
impl core::fmt::Display for Bip32Path {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("m")?;
        for &component in &self.0 {
            if component & HARDENED != 0 {
                write!(f, "/{}'", component & !HARDENED)?;
            } else {
                write!(f, "/{}", component)?;
            }
        }
        Ok(())
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

        let path: Vec<u32> = data[1..]
            .chunks(4)
            .map(|chunk| u32::from_be_bytes(chunk.try_into().unwrap()))
            .collect();
        if !is_allowed_path(&path) {
            return Err(AppSW::InvalidPath);
        }
        Ok(Bip32Path(path))
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
/// 32-byte Keccak256 hash (last 20 bytes are the Ethereum address), or
/// [`AppSW::KeyDeriveFail`] if hashing fails: an unhashed buffer must never pass for an address.
pub fn get_address_hash_from_pubkey(pubkey: &[u8; 65]) -> Result<[u8; 32], AppSW> {
    let mut keccak256 = Keccak256::new();
    let mut address: [u8; 32] = [0u8; 32];
    // Hash pubkey excluding first byte (0x04 uncompressed marker)
    keccak256
        .hash(&pubkey[1..], &mut address)
        .map_err(|_| AppSW::KeyDeriveFail)?;
    Ok(address)
}
