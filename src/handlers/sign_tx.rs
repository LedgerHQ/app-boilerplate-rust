/*****************************************************************************
 *   Ledger App Boilerplate Rust.
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
use crate::app_ui::sign::ui_display_tx;
use crate::utils::Bip32Path;
use crate::AppSW;
use alloc::vec::Vec;
use ledger_device_sdk::ecc::{Secp256k1, SeedDerive};
use ledger_device_sdk::hash::{sha3::Keccak256, HashInit};
use ledger_device_sdk::io::Comm;
use ledger_device_sdk::nbgl::NbglHomeAndSettings;
use ledger_device_sdk::testing::debug_print;

use serde::Deserialize;
use serde_json_core::from_slice;

const MAX_TRANSACTION_LEN: usize = 510;

use ledger_device_sdk::libcall::swap::CreateTxParams;

#[derive(Deserialize)]
pub struct Tx<'a> {
    #[allow(dead_code)]
    nonce: u64,
    pub coin: &'a str,
    pub value: u64,
    #[serde(with = "hex::serde")] // Allows JSON deserialization from hex string
    pub to: [u8; 20],
    pub memo: &'a str,
}

/// Transaction context holding state between APDU chunks.
pub struct TxContext<'a> {
    raw_tx: Vec<u8>,
    path: Bip32Path,
    review_finished: bool,
    pub home: NbglHomeAndSettings,
    /// Swap parameters if running in swap mode.
    /// Used to validate the transaction against the Exchange's request.
    pub swap_params: Option<&'a CreateTxParams>,
}

// Implement constructor for TxInfo with default values
impl<'a> TxContext<'a> {
    // Constructor
    pub fn new() -> TxContext<'a> {
        TxContext {
            raw_tx: Vec::new(),
            path: Default::default(),
            review_finished: false,
            home: Default::default(),
            swap_params: None,
        }
    }

    pub fn new_with_swap(params: &'a CreateTxParams) -> TxContext<'a> {
        TxContext {
            raw_tx: Vec::new(),
            path: Default::default(),
            review_finished: false,
            home: Default::default(),
            swap_params: Some(params),
        }
    }

    // Get review status
    #[allow(dead_code)]
    pub fn finished(&self) -> bool {
        self.review_finished
    }
    // Implement reset for TxInfo
    fn reset(&mut self) {
        self.raw_tx.clear();
        self.path = Default::default();
        self.review_finished = false;
    }
}

/// Handler for the Sign Transaction APDU.
///
/// Receives transaction chunks, parses them, and signs the transaction.
///
/// # Swap Mode
///
/// If `ctx.swap_params` is present:
/// 1. Validates the parsed transaction against the swap parameters (amount, destination).
/// 2. If valid, bypasses the UI and signs immediately.
/// 3. If invalid, returns an error.
pub fn handler_sign_tx(
    comm: &mut Comm,
    chunk: u8,
    more: bool,
    ctx: &mut TxContext,
) -> Result<(), AppSW> {
    debug_print("=> handler_sign_tx\n");
    // Try to get data from comm
    let data = comm.get_data().map_err(|_| AppSW::WrongApduLength)?;
    // First chunk, try to parse the path
    if chunk == 0 {
        debug_print("Chunk 0: Path parsing\n");
        // Reset transaction context
        ctx.reset();
        // This will propagate the error if the path is invalid
        ctx.path = data.try_into()?;
        Ok(())
    // Next chunks, append data to raw_tx and return or parse
    // the transaction if it is the last chunk.
    } else {
        if ctx.raw_tx.len() + data.len() > MAX_TRANSACTION_LEN {
            return Err(AppSW::TxWrongLength);
        }

        // Append data to raw_tx
        ctx.raw_tx.extend(data);

        // If we expect more chunks, return
        if more {
            ctx.review_finished = false;
            Ok(())
        // Otherwise, try to parse the transaction
        } else {
            // --8<-- [start:ui_bypass]
            debug_print("Last chunk received, parsing tx\n");
            // Try to deserialize the transaction
            let (tx, _): (Tx, usize) = from_slice(&ctx.raw_tx).map_err(|_| AppSW::TxParsingFail)?;
            debug_print("Tx parsed successfully\n");

            // Check if in swap mode
            if let Some(params) = ctx.swap_params {
                ctx.review_finished = true;
                // --8<-- [start:SwapError_usage]
                if let Err(error) = crate::swap::check_swap_params(params, &tx) {
                    // The swap validation failed and returned us the common format error defined by the SDK
                    // Use SDK method to append error code and message in standard format
                    error.append_to_comm(comm);
                    return Err(AppSW::SwapFail);
                } else {
                    debug_print("Swap validation success, bypassing UI\n");
                    compute_signature_and_append(comm, ctx)
                }
                // --8<-- [end:SwapError_usage]
            } else {
                debug_print("Normal mode, showing UI\n");
                // Display transaction. If user approves
                // the transaction, sign it. Otherwise,
                // return a "deny" status word.
                if ui_display_tx(&tx)? {
                    ctx.review_finished = true;
                    compute_signature_and_append(comm, ctx)
                } else {
                    ctx.review_finished = true;
                    Err(AppSW::Deny)
                }
            }
            // --8<-- [end:ui_bypass]
        }
    }
}

fn compute_signature_and_append(comm: &mut Comm, ctx: &mut TxContext) -> Result<(), AppSW> {
    debug_print("Signing transaction\n");
    let mut keccak256 = Keccak256::new();
    let mut message_hash: [u8; 32] = [0u8; 32];

    let _ = keccak256.hash(&ctx.raw_tx, &mut message_hash);

    let (sig, siglen, parity) = Secp256k1::derive_from_path(ctx.path.as_ref())
        .deterministic_sign(&message_hash)
        .map_err(|_| AppSW::TxSignFail)?;
    comm.append(&[siglen as u8]);
    comm.append(&sig[..siglen as usize]);
    comm.append(&[parity as u8]);
    Ok(())
}
