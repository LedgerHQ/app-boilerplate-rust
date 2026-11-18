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
use crate::{AppSW, P1_SIGN_TX_START};
use alloc::vec::Vec;
use ledger_device_sdk::ecc::{Secp256k1, SeedDerive};
use ledger_device_sdk::hash::{sha3::Keccak256, HashInit};
use ledger_device_sdk::io::{Command, CommandResponse};
use ledger_device_sdk::log;
use ledger_device_sdk::nbgl::NbglHomeAndSettings;

use serde::Deserialize;
use serde_json_core::from_slice;

const MAX_TRANSACTION_LEN: usize = 510;

use ledger_device_sdk::libcall::swap::CreateTxParams;

/// Transaction to sign.
///
/// The whole serialized transaction is signed, so parsing must reject anything the review does
/// not cover: unknown fields are denied rather than silently ignored.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
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
    /// Chunk number expected next, `None` when no flow was started by a path chunk.
    next_chunk: Option<u8>,
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
            next_chunk: None,
            review_finished: false,
            home: Default::default(),
            swap_params: None,
        }
    }

    pub fn new_with_swap(params: &'a CreateTxParams) -> TxContext<'a> {
        TxContext {
            raw_tx: Vec::new(),
            path: Default::default(),
            next_chunk: None,
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
    /// Drops the transaction flow state so that no later chunk can reuse it.
    ///
    /// `review_finished` is kept: the main loop reads it after the handler returns.
    fn clear_flow(&mut self) {
        self.raw_tx.clear();
        self.path = Default::default();
        self.next_chunk = None;
    }
}

/// Handler for the Sign Transaction APDU.
///
/// Receives transaction chunks, parses them, and signs the transaction.
///
/// A flow starts with the path chunk (`P1=0`) and continues with strictly consecutive chunk
/// numbers. Any other sequence is rejected with [`AppSW::BadState`], and the flow state is
/// cleared after every final chunk and every error so that no stale path or data can be signed.
///
/// # Swap Mode
///
/// If `ctx.swap_params` is present:
/// 1. Validates the parsed transaction against the swap parameters (amount, destination).
/// 2. If valid, bypasses the UI and signs immediately.
/// 3. If invalid, returns an error.
pub fn handler_sign_tx<'a>(
    command: Command<'a>,
    chunk: u8,
    more: bool,
    ctx: &mut TxContext,
) -> Result<CommandResponse<'a>, AppSW> {
    log::debug!("=> handler_sign_tx");
    if chunk == P1_SIGN_TX_START {
        return start_flow(command, ctx);
    }

    if ctx.next_chunk != Some(chunk) {
        log::error!("Unexpected SIGN_TX chunk");
        ctx.clear_flow();
        return Err(AppSW::BadState);
    }

    let result = continue_flow(command, more, ctx);
    if result.is_err() || !more {
        ctx.clear_flow();
    } else {
        ctx.next_chunk = chunk.checked_add(1);
    }
    result
}

/// Starts a new flow from the chunk carrying the BIP32 path, discarding any previous one.
fn start_flow<'a>(command: Command<'a>, ctx: &mut TxContext) -> Result<CommandResponse<'a>, AppSW> {
    log::debug!("Chunk 0: Path parsing");
    ctx.clear_flow();
    ctx.review_finished = false;
    ctx.path = command.get_data().try_into()?;
    ctx.next_chunk = Some(P1_SIGN_TX_START + 1);
    Ok(command.into_response())
}

/// Appends a transaction chunk and, on the last one, parses, reviews and signs the transaction.
fn continue_flow<'a>(
    command: Command<'a>,
    more: bool,
    ctx: &mut TxContext,
) -> Result<CommandResponse<'a>, AppSW> {
    let data = command.get_data();
    if ctx.raw_tx.len() + data.len() > MAX_TRANSACTION_LEN {
        return Err(AppSW::TxWrongLength);
    }
    ctx.raw_tx.extend(data);

    if more {
        return Ok(command.into_response());
    }

    // --8<-- [start:ui_bypass]
    log::debug!("Last chunk received, parsing tx");
    let tx = parse_tx(&ctx.raw_tx)?;
    log::debug!("Tx parsed successfully");

    if let Some(params) = ctx.swap_params {
        ctx.review_finished = true;
        // --8<-- [start:SwapError_usage]
        if let Err(error) = crate::swap::check_swap_params(params, &tx) {
            // The swap validation failed and returned us the common format error defined by the SDK
            // Use SDK method to append error code and message in standard format
            let mut response = command.into_response();
            error.append_to_response(&mut response)?;
            Err(AppSW::SwapFail)
        } else {
            log::debug!("Swap validation success, bypassing UI");
            compute_signature_and_append(command.into_response(), ctx)
        }
        // --8<-- [end:SwapError_usage]
    } else {
        log::debug!("Normal mode, showing UI");
        let comm = command.into_comm();
        let approved = ui_display_tx(comm, &tx, &ctx.path)?;
        ctx.review_finished = true;
        if approved {
            compute_signature_and_append(comm.begin_response(), ctx)
        } else {
            Err(AppSW::Deny)
        }
    }
    // --8<-- [end:ui_bypass]
}

/// Parses the accumulated transaction, requiring that every signed byte was consumed and that
/// every text field can be displayed.
fn parse_tx(raw_tx: &[u8]) -> Result<Tx<'_>, AppSW> {
    let (tx, consumed): (Tx, usize) = from_slice(raw_tx).map_err(|_| AppSW::TxParsingFail)?;
    if consumed != raw_tx.len() {
        return Err(AppSW::TxParsingFail);
    }
    if tx.coin.is_empty() || !is_displayable(tx.coin) || !is_displayable(tx.memo) {
        return Err(AppSW::TxParsingFail);
    }
    Ok(tx)
}

/// Text shown for review is restricted to printable ASCII: control characters (NUL included) and
/// non-ASCII characters cannot be rendered faithfully, and a NUL would make the SDK panic when it
/// builds the C strings of the review.
fn is_displayable(text: &str) -> bool {
    text.bytes().all(|b| b == b' ' || b.is_ascii_graphic())
}

fn compute_signature_and_append<'a>(
    mut response: CommandResponse<'a>,
    ctx: &mut TxContext,
) -> Result<CommandResponse<'a>, AppSW> {
    log::debug!("Signing transaction");
    let mut keccak256 = Keccak256::new();
    let mut message_hash: [u8; 32] = [0u8; 32];

    let _ = keccak256.hash(&ctx.raw_tx, &mut message_hash);

    let (sig, siglen, parity) = Secp256k1::derive_from_path(ctx.path.as_ref())
        .deterministic_sign(&message_hash)
        .map_err(|_| AppSW::TxSignFail)?;

    response
        .append(&[siglen as u8])?
        .append(&sig[..siglen as usize])?
        .append(&[parity as u8])?;
    Ok(response)
}
