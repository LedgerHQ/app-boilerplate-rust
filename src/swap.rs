/*****************************************************************************
 *   Ledger App Boilerplate Rust - Swap Feature
 *   (c) 2023 Ledger SAS.
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 *****************************************************************************/

//! Swap Feature Implementation
//!
//! This module implements the "swap" feature, which allows the Ledger Exchange app
//! to call this coin app as a library to validate addresses and amounts before
//! signing swap transactions.
//!
//! ## Important Constraints
//!
//! When called as a library by the Exchange app (via `os_lib_call`), this code runs
//! under special constraints:
//!
//! - **No heap allocation** in `check_address` and `get_printable_amount`:
//!   The Exchange app shares BSS memory with the coin app. Writing to BSS (which
//!   includes heap allocations via `Vec`, `String`, etc.) will trigger integrity
//!   check failures and crash the device. Use stack-allocated types like arrays
//!   and `ArrayString` instead.
//!
//! - **BSS is reset** only before `sign_transaction`: The SDK calls `c_reset_bss()`
//!   before the signing phase, making heap allocation safe at that point.
//!
//! ## Swap Flow
//!
//! 1. **CHECK_ADDRESS**: Verify the destination address belongs to this device
//! 2. **GET_PRINTABLE_AMOUNT**: Format amounts for display (fees, totals)
//! 3. **SIGN_TRANSACTION**: Sign the transaction (UI bypass - already validated by Exchange)

use arrayvec::ArrayString;
use core::fmt::Write;
use ledger_device_sdk::{
    ecc::{Secp256k1, SeedDerive},
    libcall::{
        self,
        string::uint256_to_float,
        swap::{
            self, CheckAddressParams, CreateTxParams, PrintableAmountParams, SwapError,
            SwapErrorCommonCode, DEFAULT_COIN_CONFIG_BUF_SIZE,
        },
        SwapAppErrorCodeTrait,
    },
    log,
};

use crate::handlers::sign_tx::{Asset, Transfer, TxKind};
use crate::utils::{
    get_address_hash_from_pubkey, is_allowed_path, MAX_DECIMALS, MAX_PATH_LEN, NATIVE_DECIMALS,
    NATIVE_TICKER,
};
use alloc::{format, string::ToString};

/// Longest amount string the SDK forwards to the Exchange app (`MAX_PRINTABLE_AMOUNT_SIZE - 2`
/// in the C SDK).
pub const MAX_PRINTABLE_AMOUNT_LEN: usize = 48;
type PrintableAmount = ArrayString<MAX_PRINTABLE_AMOUNT_LEN>;

/// Application-specific swap error codes.
///
/// This enum defines the lower byte of the 2-byte error code used in swap error responses.
/// It provides additional context and refinement to the common error codes defined in
/// [`ledger_device_sdk::libcall::swap::SwapErrorCommonCode`].
///
/// # Error Code Structure
///
/// The complete 2-byte error code is structured as:
/// - **Upper byte**: Common error code from `SwapErrorCommonCode` (e.g., `ErrorWrongAmount`)
/// - **Lower byte**: Application-specific code from this enum (for additional context)
///
/// # Usage
///
/// When returning a swap error, combine a common code with an app-specific code. The handler
/// returns it in [`crate::AppError::Swap`], and `normal_main` sends it with
/// [`crate::AppSW::SwapFail`]:
///
/// ```rust,ignore
/// use ledger_device_sdk::libcall::swap::{SwapError, SwapErrorCommonCode};
///
/// // Example: Amount validation failed, no app-specific refinement needed
/// return Err(AppError::Swap(SwapError::with_message(
///     SwapErrorCommonCode::ErrorWrongAmount,
///     SwapAppErrorCode::Default,
///     "Amount mismatch: tx=1000, swap=2000".to_string(),
/// )));
/// ```
///
/// # Template Note
///
/// This is a template/placeholder enum. In a production application, you may want to add
/// specific error codes to provide more granular error information. For example:
///
/// ```rust,ignore
/// pub enum SwapAppErrorCode {
///     Default = 0x00,
///     AmountOverflow = 0x01,
///     AmountUnderflow = 0x02,
///     InvalidAddressChecksum = 0x03,
///     // ... add your application-specific codes
/// }
/// ```
///
/// Refer to the C SDK's `swap_error_code_helpers.h` for the specification.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwapAppErrorCode {
    /// Default application error code (no specific refinement).
    ///
    /// Use this when the common error code from `SwapErrorCommonCode` is sufficient
    /// and no additional application-specific context is needed.
    Default = 0x00,
    /// Other error codes, don't hesitate to add your own for more granularity.
    AmountCastFail = 0x01,
    DestinationDecodeFail = 0x02,
    CoinConfigFail = 0x03,
    ExtraIdMismatch = 0x04,
    /// The token of the transaction is unknown, or its ticker or decimals differ from those of
    /// the swap coin configuration.
    WrongTokenInfo = 0x05,
}

impl SwapAppErrorCodeTrait for SwapAppErrorCode {
    // Cast is safe as enum is #[repr(u8)]
    fn as_u8(self) -> u8 {
        self as u8
    }
}

//  --8<-- [start:check_swap_params]
/// This function performs a strict validation of the transaction to be signed
/// against the reference transaction parameters provided by the Exchange app.
/// The UI is bypassed in swap mode, so every field of the signed transaction must be checked here.
/// It checks that:
/// 1. The transaction asset matches the coin configuration of the swap.
/// 2. The transaction amount matches the swap amount exactly.
/// 3. The swap fees are zero, as transactions of this app carry no fee field.
/// 4. The destination address matches the swap destination address exactly.
/// 5. The memo matches the swap destination extra ID exactly.
///
/// The nonce is the only field left unchecked: it orders the account's transactions and does not
/// affect what is transferred.
///
/// # Errors
///
/// Returns error if:
/// - Coin configuration is malformed or unsupported (ErrorWrongMethod / CoinConfigFail)
/// - A native coin transfer is made for a token swap, or a token transfer for a native coin swap
///   (ErrorWrongMethod)
/// - Token ticker or decimals don't match the coin configuration (ErrorWrongAmount /
///   WrongTokenInfo)
/// - Amount does not fit in a u64 (AmountCastFail)
/// - Amount doesn't match between tx and swap params (ErrorWrongAmount)
/// - Fees are not zero (ErrorWrongFees)
/// - Destination address has invalid UTF-8 (DestinationDecodeFail)
/// - Destination address hex decode fails (DestinationDecodeFail)
/// - Destination address doesn't match (ErrorWrongDestination)
/// - Memo doesn't match the destination extra ID (ErrorWrongDestination / ExtraIdMismatch)
pub fn check_swap_params(
    params: &CreateTxParams,
    tx: &Transfer,
) -> Result<(), SwapError<SwapAppErrorCode>> {
    log::debug!("Swap mode detected\n");

    check_swap_asset(params, &tx.asset)?;

    // Validate amount
    // The amount is right-aligned big-endian in a 16-byte buffer. Transaction values are u64, so
    // any amount that does not fit in a u64 cannot match and is rejected rather than truncated.
    let swap_amount = u128::from_be_bytes(params.amount);
    let swap_amount = u64::try_from(swap_amount).map_err(|_| {
        SwapError::without_message(
            SwapErrorCommonCode::ErrorWrongAmount,
            SwapAppErrorCode::AmountCastFail,
        )
    })?;

    //  --8<-- [start:SwapError_example]
    if tx.value != swap_amount {
        log::debug!("Swap amount mismatch\n");
        debug_u64("Tx: ", tx.value);
        debug_u64("Swap: ", swap_amount);
        // Error detected, we return the error with detailed message in common SDK defined format
        return Err(SwapError::with_message(
            SwapErrorCommonCode::ErrorWrongAmount,
            SwapAppErrorCode::Default,
            format!("Amount tx {} != swap {}", tx.value, swap_amount),
        ));
    }
    //  --8<-- [end:SwapError_example]

    // Validate destination
    let dest_str =
        core::str::from_utf8(&params.dest_address[..params.dest_address_len]).map_err(|_| {
            SwapError::with_message(
                SwapErrorCommonCode::ErrorWrongDestination,
                SwapAppErrorCode::DestinationDecodeFail,
                "Failed to read destination hex".to_string(),
            )
        })?;
    let dest_hex = dest_str.strip_prefix("0x").unwrap_or(dest_str);

    let mut swap_dest = [0u8; 20];
    if hex::decode_to_slice(dest_hex, &mut swap_dest).is_err() {
        log::error!("Swap dest hex decode fail");
        return Err(SwapError::with_message(
            SwapErrorCommonCode::ErrorWrongDestination,
            SwapAppErrorCode::DestinationDecodeFail,
            format!("Failed to decode destination hex: {}", dest_hex),
        ));
    }

    if tx.to != swap_dest {
        log::error!("Swap destination mismatch");
        log::error!("Tx: {:x?}", &tx.to);
        log::error!("Swap: {:x?}", &swap_dest);
        // Only build hex strings for error message (not on happy path)
        let tx_hex = hex::encode(tx.to);
        let swap_hex = hex::encode(swap_dest);
        return Err(SwapError::with_message(
            SwapErrorCommonCode::ErrorWrongDestination,
            SwapAppErrorCode::Default,
            format!("Destination mismatch: tx {} != swap {}", tx_hex, swap_hex),
        ));
    }

    check_swap_fees(params)?;
    check_swap_extra_id(params, tx)?;

    log::debug!("Swap validation success, bypassing UI");
    Ok(())
}

/// Error for a transaction of `kind` whose asset the app does not support, which cannot match any
/// swap: a native transaction with another coin than the native one, or a token transaction naming
/// a token the app does not know.
///
/// The error codes are those [`check_swap_asset`] gives for the same kind of mismatch.
pub fn unsupported_asset_error(
    params: &CreateTxParams,
    kind: TxKind,
) -> SwapError<SwapAppErrorCode> {
    match kind {
        TxKind::Native => {
            log::error!("Swap coin not supported");
            SwapError::with_message(
                SwapErrorCommonCode::ErrorWrongMethod,
                SwapAppErrorCode::Default,
                "Coin not supported".to_string(),
            )
        }
        TxKind::Token if is_native_coin_config(params) => token_in_native_swap_error(),
        TxKind::Token => {
            log::error!("Swap token unknown");
            SwapError::with_message(
                SwapErrorCommonCode::ErrorWrongAmount,
                SwapAppErrorCode::WrongTokenInfo,
                "Unknown token".to_string(),
            )
        }
    }
}

/// Error of a final transaction that cannot be parsed. As in the C boilerplate, it ends the swap
/// with a generic swap error instead of a plain status word.
pub fn unparsable_tx_error() -> SwapError<SwapAppErrorCode> {
    log::error!("Swap tx parsing failed");
    SwapError::without_message(SwapErrorCommonCode::ErrorGeneric, SwapAppErrorCode::Default)
}

/// Whether the swap coin configuration designates the native coin.
///
/// Only an empty configuration does. A non-empty one describes a token, even with the native ticker
/// and decimals: the CAL can hold a token with the native ticker.
fn is_native_coin_config(params: &CreateTxParams) -> bool {
    params.coin_config_len == 0
}

fn token_in_native_swap_error() -> SwapError<SwapAppErrorCode> {
    log::error!("Token transfer in a native coin swap");
    SwapError::with_message(
        SwapErrorCommonCode::ErrorWrongMethod,
        SwapAppErrorCode::Default,
        format!("Token tx != swap coin {}", NATIVE_TICKER),
    )
}

/// Checks that the transaction transfers the asset described by the swap coin configuration.
///
/// A native coin swap requires a native transaction (see [`is_native_coin_config`]). A token swap
/// requires a token transaction whose token has the ticker and the decimals of the configuration.
///
/// The configuration comes from the CAL, through the Exchange app, and describes the token by its
/// ticker and decimals only. The ticker is enough to identify the token: at most one address
/// resolves to a given ticker, as the tickers of the hardcoded list are unique, the app keeps one
/// descriptor at a time, and a descriptor cannot reuse the ticker of a listed token at another
/// address (see [`crate::token::parse_descriptor`]).
fn check_swap_asset(
    params: &CreateTxParams,
    asset: &Asset,
) -> Result<(), SwapError<SwapAppErrorCode>> {
    let config = &params.coin_config[..params.coin_config_len];
    let swap_asset = parse_coin_config(config).ok_or_else(|| {
        SwapError::without_message(
            SwapErrorCommonCode::ErrorWrongMethod,
            SwapAppErrorCode::CoinConfigFail,
        )
    })?;
    match (asset, is_native_coin_config(params)) {
        (Asset::Native, true) => Ok(()),
        (Asset::Native, false) => {
            log::error!("Native coin transfer in a token swap");
            Err(SwapError::with_message(
                SwapErrorCommonCode::ErrorWrongMethod,
                SwapAppErrorCode::Default,
                format!(
                    "Asset tx {} != swap token {}",
                    NATIVE_TICKER, swap_asset.ticker
                ),
            ))
        }
        (Asset::Token(_), true) => Err(token_in_native_swap_error()),
        (Asset::Token(info), false)
            if info.ticker() == swap_asset.ticker.as_str()
                && info.decimals() == swap_asset.decimals =>
        {
            Ok(())
        }
        (Asset::Token(info), false) => {
            log::error!("Swap token mismatch");
            Err(SwapError::with_message(
                SwapErrorCommonCode::ErrorWrongAmount,
                SwapAppErrorCode::WrongTokenInfo,
                format!(
                    "Token tx {} ({} decimals) != swap {} ({} decimals)",
                    info.ticker(),
                    info.decimals(),
                    swap_asset.ticker,
                    swap_asset.decimals
                ),
            ))
        }
    }
}

/// Checks that the swap carries no fees.
///
/// Transactions of this app have no fee field, so any non-zero fee approved in the Exchange app
/// could not be matched against the signed transaction. An app whose transactions carry fees must
/// compare them with `params.fee_amount` instead.
fn check_swap_fees(params: &CreateTxParams) -> Result<(), SwapError<SwapAppErrorCode>> {
    if params.fee_amount.iter().any(|&b| b != 0) {
        log::error!("Swap fees not supported");
        return Err(SwapError::with_message(
            SwapErrorCommonCode::ErrorWrongFees,
            SwapAppErrorCode::Default,
            "Fees are not supported".to_string(),
        ));
    }
    Ok(())
}

/// Checks that the transaction memo is exactly the destination extra ID of the swap.
fn check_swap_extra_id(
    params: &CreateTxParams,
    tx: &Transfer,
) -> Result<(), SwapError<SwapAppErrorCode>> {
    let extra_id = &params.dest_address_extra_id[..params.dest_address_extra_id_len];
    if tx.memo.as_bytes() != extra_id {
        log::error!("Swap memo mismatch");
        return Err(SwapError::with_message(
            SwapErrorCommonCode::ErrorWrongDestination,
            SwapAppErrorCode::ExtraIdMismatch,
            format!("Memo tx {} != swap extra ID", tx.memo),
        ));
    }
    Ok(())
}
//  --8<-- [end:check_swap_params]

/// Helper function to print u64 for debugging.
pub fn debug_u64(label: &str, val: u64) {
    let mut buf = ArrayString::<64>::new();
    let _ = writeln!(&mut buf, "{}{}", label, val);
    log::debug!("{}", buf.as_str());
}

/// Helper function to print hex-encoded bytes for debugging.
/// Uses stack-allocated buffer to avoid BSS writes.
pub fn debug_hex(label: &str, data: &[u8]) {
    log::debug!("{}", label);
    let mut buf = ArrayString::<140>::new();
    for b in data {
        let _ = write!(&mut buf, "{:02x}", b);
    }
    let _ = writeln!(&mut buf);
    log::debug!("{}", buf.as_str());
}

// --8<-- [start:swap_main]
/// Main entry point when app is called as a library by the Exchange app.
///
/// # Arguments
///
/// * `arg0` - Parameter passed by `os_lib_call` containing command and data pointers
///
/// The Exchange app calls this function with different commands during a swap:
/// - `SwapCheckAddress`: Validate that an address belongs to this device
/// - `SwapGetPrintableAmount`: Format amounts/fees for display
/// - `SwapSignTransaction`: Sign the final transaction
pub fn swap_main(arg0: u32) {
    log::debug!("swap_main called\n");
    let cmd = libcall::get_command(arg0);

    match cmd {
        libcall::LibCallCommand::SwapCheckAddress => {
            log::debug!("Received SwapCheckAddress command\n");
            let mut params = swap::get_check_address_params(arg0);
            let res = check_address(&params);
            // Return to Exchange, forwarding the result
            swap::swap_return(swap::SwapResult::CheckAddressResult(&mut params, res));
        }
        libcall::LibCallCommand::SwapGetPrintableAmount => {
            log::debug!("Received SwapGetPrintableAmount command\n");
            let mut params = swap::get_printable_amount_params(arg0);
            let amount_str = get_printable_amount(&params);
            // Return to Exchange, forwarding the result. An empty string reports the failure.
            swap::swap_return(swap::SwapResult::PrintableAmountResult(
                &mut params,
                amount_str.as_ref().map_or("", PrintableAmount::as_str),
            ));
        }
        libcall::LibCallCommand::SwapSignTransaction => {
            log::debug!("Received SwapSignTransaction command\n");
            let mut params = swap::sign_tx_params(arg0);
            // Call normal_main with Swap parameter set to enter the special Swap flow
            let success = crate::normal_main(Some(&params));
            // Return to Exchange, forwarding the result
            if success {
                swap::swap_return(swap::SwapResult::CreateTxResult(&mut params, 1));
            } else {
                swap::swap_return(swap::SwapResult::CreateTxResult(&mut params, 0));
            }
        }
    }
}
// --8<-- [end:swap_main]

// --8<-- [start:check_address]
/// Verify that a given address belongs to this device.
///
/// The Exchange app calls this to ensure the user owns the destination address
/// before proceeding with the swap. This prevents sending funds to wrong addresses.
///
/// # Flow
///
/// 1. Parse BIP32 derivation path from params
/// 2. Derive public key from the path
/// 3. Compute address from public key (Keccak256 hash)
/// 4. Compare with reference address from Exchange
///
/// # Important Notes
///
/// - **No heap allocation**: Uses stack arrays only (BSS memory is shared with Exchange)
/// - **Hex string comparison**: Exchange sends address as hex string via C API,
///   so we convert our computed address to hex for comparison
/// - **Address format**: This app uses Ethereum-style addresses (last 20 bytes of
///   Keccak256 hash of pubkey). Adapt this for your blockchain's address format.
///
/// # Arguments
///
/// * `params` - Contains BIP32 path and reference address from Exchange
///
/// # Returns
///
/// * `1` if addresses match (valid)
/// * `0` if addresses don't match or error occurred
fn check_address(params: &CheckAddressParams) -> i32 {
    // Parse BIP32 derivation path
    // Note: params.dpath_len is the NUMBER of u32 path components (e.g., 5 for m/44'/1'/0'/0/0),
    // not the byte length. Each component is 4 bytes (big-endian u32).
    let path_bytes = &params.dpath[..params.dpath_len * 4];

    // Use stack-allocated array (no heap!) to store parsed path
    let mut path = [0u32; MAX_PATH_LEN];

    if params.dpath_len > MAX_PATH_LEN {
        log::error!("Path too long\n");
        return 0;
    }

    // Convert big-endian bytes to u32 path components
    for i in 0..params.dpath_len {
        path[i] = u32::from_be_bytes([
            path_bytes[i * 4],
            path_bytes[i * 4 + 1],
            path_bytes[i * 4 + 2],
            path_bytes[i * 4 + 3],
        ]);
    }

    // Derive public key from path using the same logic as get_public_key handler
    let path = &path[..params.dpath_len];
    if !is_allowed_path(path) {
        log::error!("Path not allowed\n");
        return 0;
    }

    let (k, _) = Secp256k1::derive_from(path);
    let pubkey = match k.public_key() {
        Ok(pk) => pk.pubkey,
        Err(_) => {
            log::error!("Key derivation failed\n");
            return 0;
        }
    };

    // Compute address: Keccak256 hash of pubkey (excluding first byte 0x04)
    let Ok(address_hash) = get_address_hash_from_pubkey(&pubkey) else {
        log::error!("Address hash failed\n");
        return 0;
    };
    // Take last 20 bytes as address (Ethereum-style)
    let address = &address_hash[address_hash.len() - 20..];

    // Exchange sends address bytes, but SDK's read_c_string() interprets them as
    // a hex string. This is a quirk of the C API - the Exchange sends binary address
    // bytes, but they're read as ASCII characters.
    // Example: byte 0x04 becomes ASCII '0' (0x30) and '4' (0x34) = "04" in the string
    let ref_hex = match core::str::from_utf8(&params.ref_address[..params.ref_address_len]) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    // Convert our derived address to hex string for comparison
    // Using ArrayString (stack-allocated) to avoid heap allocation
    let mut our_hex = ArrayString::<40>::new(); // 20 bytes * 2 hex chars
    for b in address {
        let _ = write!(&mut our_hex, "{:02x}", b);
    }

    // Compare hex strings
    if our_hex.as_str() == ref_hex {
        log::debug!("Check address successful, derived and received addresses match\n");
        1 // Success
    } else {
        log::debug!("Derived and received addresses do NOT match\n");
        log::debug!("Derived address: {:x?}", address);
        log::debug!("Reference (hex): {}", ref_hex);
        0 // Failure
    }
}
// --8<-- [end:check_address]

// --8<-- [start:get_printable_amount]
/// Asset a swap operates on, as described by the Exchange coin configuration.
pub struct SwapAsset {
    /// Ticker displayed next to amounts and expected in the signed transaction.
    pub ticker: ArrayString<DEFAULT_COIN_CONFIG_BUF_SIZE>,
    /// Number of decimals of the asset's base unit.
    pub decimals: u8,
}

/// Parses the coin configuration that the Exchange app forwards from its trusted currency
/// configuration.
///
/// Only an empty configuration designates the native coin. Otherwise it describes a token, with the
/// layout `[ticker_len: u8][ticker][decimals: u8]`, which must fill the configuration exactly.
///
/// Returns `None` for any malformed or unsupported configuration: an asset that cannot be
/// identified must never be displayed or signed under assumed metadata.
///
/// Does not allocate, as it runs while the Exchange app shares BSS memory.
pub fn parse_coin_config(config: &[u8]) -> Option<SwapAsset> {
    if config.is_empty() {
        return native_asset();
    }

    let (&ticker_len, rest) = config.split_first()?;
    let (ticker, decimals) = rest.split_at_checked(usize::from(ticker_len))?;
    let &[decimals] = decimals else {
        return None;
    };
    if ticker.is_empty() || !ticker.iter().all(u8::is_ascii_graphic) || decimals > MAX_DECIMALS {
        return None;
    }

    Some(SwapAsset {
        ticker: ArrayString::from(core::str::from_utf8(ticker).ok()?).ok()?,
        decimals,
    })
}

/// The native coin, as a [`SwapAsset`].
fn native_asset() -> Option<SwapAsset> {
    Some(SwapAsset {
        ticker: ArrayString::from(NATIVE_TICKER).ok()?,
        decimals: NATIVE_DECIMALS,
    })
}

/// Format an amount for display in the Exchange app UI.
///
/// The Exchange app calls this to get human-readable strings for amounts and fees.
/// This is used during swap transactions to show the user what amounts they're
/// exchanging.
///
/// # Amount Format
///
/// The amount is provided as big-endian bytes in `params.amount`:
/// - Right-aligned in a 16-byte buffer (AMOUNT_BUF_SIZE)
/// - Actual length is in `params.amount_len`
/// - Padded to 32 bytes (uint256) for SDK formatting helpers
///
/// The ticker and decimals come from `params.coin_config` (see [`parse_coin_config`]), except for
/// fees: they are always paid in the native coin, even in a token swap.
///
/// # Returns
///
/// Stack-allocated string formatted as "{ticker} {value}" (e.g., "CRAB 1.5"), or `None` if the
/// coin configuration is not supported or the result does not fit the Exchange buffer. The
/// caller reports `None` to the Exchange app as a failure.
///
/// # Memory Safety
///
/// Uses `ArrayString` (stack-allocated) to avoid heap allocation, as this function
/// runs under BSS memory restrictions.
fn get_printable_amount(params: &PrintableAmountParams) -> Option<PrintableAmount> {
    let asset = if params.is_fee {
        native_asset()?
    } else {
        parse_coin_config(&params.coin_config[..params.coin_config_len])?
    };

    // Convert amount from 16-byte buffer to 32-byte buffer (uint256 format)
    // The amount is right-aligned in params.amount, we need to copy it to a
    // 32-byte buffer that's also right-aligned (big-endian)
    let mut amount_u256: [u8; 32] = [0; 32];
    let src_start = params.amount.len() - params.amount_len;
    let dst_start = 32 - params.amount_len;
    amount_u256[dst_start..].copy_from_slice(&params.amount[src_start..]);

    log::debug!("Amount bytes (u256): ");
    debug_hex("", &amount_u256);

    let amount_str = uint256_to_float(&amount_u256, usize::from(asset.decimals));

    let mut printable = PrintableAmount::new();
    write!(&mut printable, "{} {}", asset.ticker, amount_str.as_str()).ok()?;

    log::debug!("Formatted amount: {}", printable.as_str());

    Some(printable)
}
// --8<-- [end:get_printable_amount]
