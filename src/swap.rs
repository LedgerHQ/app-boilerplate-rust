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
        swap::{self, CheckAddressParams, PrintableAmountParams, CreateTxParams},
    },
    testing::debug_print,
};

use crate::handlers::sign_tx::Tx;
use crate::AppSW;
use crate::utils::get_address_hash_from_pubkey;

//  --8<-- [start:check_swap_params]
/// This function performs a strict validation of the transaction to be signed
/// against the reference transaction parameters provided by the Exchange app.
/// It checks that:
/// 1. The transaction type matches the expected one (Only one type of Tx implemented so auto true).
/// 2. The transaction amount matches the swap amount exactly.
/// 3. The transaction fees matches the swap fees exactly. (fees are not implemented so auto true).
/// 4. The destination address matches the swap destination address exactly.
///
/// Returns `Ok(())` if validation passes, or `Err(AppSW::SwapFail)` if any mismatch is found.
pub fn check_swap_params(params: &CreateTxParams, tx: &Tx) -> Result<(), AppSW> {
    debug_print("Swap mode detected\n");

    // Validate amount
    // Parse amount (u64 from big-endian bytes, right aligned in 16-byte buffer)
    let start = params.amount.len() - 8;
    let amount_bytes: [u8; 8] = params.amount[start..].try_into().unwrap_or([0u8; 8]);
    let swap_amount = u64::from_be_bytes(amount_bytes);

    if tx.value != swap_amount {
        debug_print("Swap amount mismatch\n");
        debug_u64("Tx: ", tx.value);
        debug_u64("Swap: ", swap_amount);
        return Err(AppSW::SwapFail);
    }

    // Validate destination
    let dest_str = core::str::from_utf8(&params.dest_address[..params.dest_address_len])
        .map_err(|_| AppSW::SwapFail)?;
    let dest_hex = dest_str.strip_prefix("0x").unwrap_or(dest_str);

    let mut swap_dest = [0u8; 20];
    if hex::decode_to_slice(dest_hex, &mut swap_dest).is_err() {
        debug_print("Swap dest hex decode fail\n");
        return Err(AppSW::SwapFail);
    }

    if tx.to != swap_dest {
        debug_print("Swap destination mismatch\n");
        debug_hex("Tx: ", &tx.to);
        debug_hex("Swap: ", &swap_dest);
        return Err(AppSW::SwapFail);
    }

    debug_print("Swap validation success\n");
    Ok(())
}
//  --8<-- [end:check_swap_params]

/// Helper function to print u64 for debugging.
pub fn debug_u64(label: &str, val: u64) {
    let mut buf = ArrayString::<64>::new();
    let _ = write!(&mut buf, "{}{}\n", label, val);
    debug_print(buf.as_str());
}

/// Helper function to print hex-encoded bytes for debugging.
/// Uses stack-allocated buffer to avoid BSS writes.
pub fn debug_hex(label: &str, data: &[u8]) {
    debug_print(label);
    let mut buf = ArrayString::<140>::new();
    for b in data {
        let _ = write!(&mut buf, "{:02x}", b);
    }
    let _ = write!(&mut buf, "\n");
    debug_print(buf.as_str());
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
    debug_print("swap_main called\n");
    let cmd = libcall::get_command(arg0);

    match cmd {
        libcall::LibCallCommand::SwapCheckAddress => {
            debug_print("Received SwapCheckAddress command\n");
            let mut params = swap::get_check_address_params(arg0);
            let res = check_address(&params);
            // Return to Exchange, forwarding the result
            swap::swap_return(swap::SwapResult::CheckAddressResult(&mut params, res));
        }
        libcall::LibCallCommand::SwapGetPrintableAmount => {
            debug_print("Received SwapGetPrintableAmount command\n");
            let mut params = swap::get_printable_amount_params(arg0);
            let amount_str = get_printable_amount(&params);
            // Return to Exchange, forwarding the result
            swap::swap_return(swap::SwapResult::PrintableAmountResult(
                &mut params,
                amount_str.as_str(),
            ));
        }
        libcall::LibCallCommand::SwapSignTransaction => {
            debug_print("Received SwapSignTransaction command\n");
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
    let mut path: [u32; 10] = [0; 10]; // Max 10 derivation levels

    if params.dpath_len > 10 {
        debug_print("Path too long\n");
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
    let (k, _) = Secp256k1::derive_from(&path[..params.dpath_len]);
    let pubkey = match k.public_key() {
        Ok(pk) => pk.pubkey,
        Err(_) => {
            debug_print("Key derivation failed\n");
            return 0;
        }
    };

    // Compute address: Keccak256 hash of pubkey (excluding first byte 0x04)
    let address_hash = get_address_hash_from_pubkey(&pubkey);
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
        debug_print("Check address successful, derived and received addresses match\n");
        1 // Success
    } else {
        debug_print("Derived and received addresses do NOT match\n");
        debug_hex("Derived address: ", address);
        debug_print("Reference (hex): ");
        debug_print(ref_hex);
        debug_print("\n");
        0 // Failure
    }
}
// --8<-- [end:check_address]

// --8<-- [start:get_printable_amount]
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
/// # Arguments
///
/// * `params` - Contains:
///   - `amount`: Big-endian encoded amount bytes (right-aligned in 16-byte buffer)
///   - `amount_len`: Actual number of significant bytes
///   - `coin_config`: Coin configuration (unused - hardcoded to CRAB in this template)
///   - `is_fee`: Whether this is a fee amount
///
/// # Returns
///
/// Stack-allocated string formatted as "CRAB {value}" (e.g., "CRAB 1.5")
///
/// # Memory Safety
///
/// Uses `ArrayString` (stack-allocated) to avoid heap allocation, as this function
/// runs under BSS memory restrictions.
///
/// # Production Notes
///
/// For a production app, you should:
/// - Parse `coin_config` to extract ticker and decimals dynamically
/// - Handle different coin types
/// - Support both u64 and u128 amounts
fn get_printable_amount(params: &PrintableAmountParams) -> ArrayString<40> {
    // Convert amount from 16-byte buffer to 32-byte buffer (uint256 format)
    // The amount is right-aligned in params.amount, we need to copy it to a
    // 32-byte buffer that's also right-aligned (big-endian)
    let mut amount_u256: [u8; 32] = [0; 32];
    let src_start = params.amount.len() - params.amount_len;
    let dst_start = 32 - params.amount_len;
    amount_u256[dst_start..].copy_from_slice(&params.amount[src_start..]);

    debug_print("Amount bytes (u256): ");
    debug_hex("", &amount_u256);

    // CRAB uses 9 decimals (similar to SUI/ETH Wei conversions)
    // For production: parse decimals from params.coin_config
    const CRAB_DECIMALS: usize = 9;
    const CRAB_TICKER: &str = "CRAB";

    // Use SDK helper to format amount with decimals
    let amount_str = uint256_to_float(&amount_u256, CRAB_DECIMALS);

    // Format as "CRAB {value}" using stack-allocated ArrayString
    let mut printable = ArrayString::<40>::new();
    let _ = write!(
        &mut printable,
        "{} {}",
        CRAB_TICKER,
        amount_str.as_str()
    );

    debug_print("Formatted amount: ");
    debug_print(printable.as_str());
    debug_print("\n");

    printable
}
// --8<-- [end:get_printable_amount]
