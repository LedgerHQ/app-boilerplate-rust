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
use crate::token::{self, DynamicToken, TokenInfo, TOKEN_ADDRESS_LEN};
use crate::utils::{Bip32Path, NATIVE_TICKER};
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

/// Kind of transaction a signing flow carries, set by the instruction that sends it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TxKind {
    /// Transfer of the native coin, sent with SIGN_TX.
    Native,
    /// Transfer of a token, sent with SIGN_TOKEN_TX.
    Token,
}

// The whole serialized transaction is signed, so parsing must reject anything the review does not
// cover: unknown fields are denied rather than silently ignored.

/// Native coin transaction, as serialized by the host.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeTx<'a> {
    #[allow(dead_code)]
    nonce: u64,
    coin: &'a str,
    value: u64,
    #[serde(with = "hex::serde")] // Allows JSON deserialization from hex string
    to: [u8; 20],
    memo: &'a str,
}

/// Token transaction, as serialized by the host. The token is named by its address only.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TokenTx<'a> {
    #[allow(dead_code)]
    nonce: u64,
    #[serde(with = "hex::serde")]
    token: [u8; TOKEN_ADDRESS_LEN],
    value: u64,
    #[serde(with = "hex::serde")]
    to: [u8; 20],
    memo: &'a str,
}

/// Transaction as parsed, before its asset is checked against those the app supports.
enum Tx<'a> {
    Native(NativeTx<'a>),
    Token(TokenTx<'a>),
}

/// Asset moved by a [`Transfer`].
pub enum Asset {
    Native,
    Token(TokenInfo),
}

/// Transfer requested by a transaction whose asset the app supports.
pub struct Transfer<'a> {
    pub asset: Asset,
    pub value: u64,
    pub to: [u8; 20],
    pub memo: &'a str,
}

impl<'a> Tx<'a> {
    /// Resolves the asset of the transaction, `None` if the app does not support it.
    ///
    /// A token is looked up in `dynamic`, the token of the last accepted descriptor, then in the
    /// hardcoded list.
    fn into_transfer(self, dynamic: Option<&DynamicToken>) -> Option<Transfer<'a>> {
        match self {
            Tx::Native(tx) => (tx.coin == NATIVE_TICKER).then_some(Transfer {
                asset: Asset::Native,
                value: tx.value,
                to: tx.to,
                memo: tx.memo,
            }),
            Tx::Token(tx) => Some(Transfer {
                asset: Asset::Token(token::lookup(&tx.token, dynamic)?),
                value: tx.value,
                to: tx.to,
                memo: tx.memo,
            }),
        }
    }
}

/// Transaction context holding state between APDU chunks.
pub struct TxContext<'a> {
    raw_tx: Vec<u8>,
    path: Bip32Path,
    /// Chunk number expected next, `None` when no flow was started by a path chunk.
    next_chunk: Option<u8>,
    /// Kind of the pending flow, `None` when no flow was started by a path chunk.
    flow_kind: Option<TxKind>,
    review_finished: bool,
    /// Token of the last accepted descriptor. It outlives signing flows, until the app exits or
    /// another PROVIDE_TOKEN_INFO is received.
    dynamic_token: Option<DynamicToken>,
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
            flow_kind: None,
            review_finished: false,
            dynamic_token: None,
            home: Default::default(),
            swap_params: None,
        }
    }

    pub fn new_with_swap(params: &'a CreateTxParams) -> TxContext<'a> {
        TxContext {
            raw_tx: Vec::new(),
            path: Default::default(),
            next_chunk: None,
            flow_kind: None,
            review_finished: false,
            dynamic_token: None,
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
    /// `review_finished` is kept: the main loop reads it after the handler returns. The token
    /// descriptor slot is kept too, as it outlives flows.
    pub fn clear_flow(&mut self) {
        self.raw_tx.clear();
        self.path = Default::default();
        self.next_chunk = None;
        self.flow_kind = None;
    }

    /// Replaces the token of the descriptor slot, `None` emptying it.
    pub fn set_dynamic_token(&mut self, token: Option<DynamicToken>) {
        self.dynamic_token = token;
    }
}

/// Handler for the SIGN_TX and SIGN_TOKEN_TX APDUs.
///
/// Receives transaction chunks, parses them, and signs the transaction.
///
/// A flow starts with the path chunk (`P1=0`) and continues with strictly consecutive chunk
/// numbers, all sent with the instruction of that path chunk. Any other sequence is rejected with
/// [`AppSW::BadState`], and the flow state is cleared after every final chunk and every error so
/// that no stale path or data can be signed.
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
    kind: TxKind,
    ctx: &mut TxContext,
) -> Result<CommandResponse<'a>, AppSW> {
    log::debug!("=> handler_sign_tx");
    if chunk == P1_SIGN_TX_START {
        return start_flow(command, kind, ctx);
    }

    if ctx.next_chunk != Some(chunk) || ctx.flow_kind != Some(kind) {
        log::error!("Unexpected chunk, or INS other than the one of chunk 0");
        ctx.clear_flow();
        return Err(AppSW::BadState);
    }

    let result = continue_flow(command, more, kind, ctx);
    if result.is_err() || !more {
        ctx.clear_flow();
    } else {
        ctx.next_chunk = chunk.checked_add(1);
    }
    result
}

/// Starts a new flow from the chunk carrying the BIP32 path, discarding any previous one.
fn start_flow<'a>(
    command: Command<'a>,
    kind: TxKind,
    ctx: &mut TxContext,
) -> Result<CommandResponse<'a>, AppSW> {
    log::debug!("Chunk 0: Path parsing");
    ctx.clear_flow();
    ctx.review_finished = false;
    ctx.path = command.get_data().try_into()?;
    ctx.next_chunk = Some(P1_SIGN_TX_START + 1);
    ctx.flow_kind = Some(kind);
    Ok(command.into_response())
}

/// Appends a transaction chunk and, on the last one, parses, reviews and signs the transaction.
fn continue_flow<'a>(
    command: Command<'a>,
    more: bool,
    kind: TxKind,
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
    let tx = parse_tx(&ctx.raw_tx, kind)?;
    log::debug!("Tx parsed successfully");

    if let Some(params) = ctx.swap_params {
        ctx.review_finished = true;
        // --8<-- [start:SwapError_usage]
        let checked = tx
            .into_transfer(ctx.dynamic_token.as_ref())
            .ok_or_else(crate::swap::unsupported_asset_error)
            .and_then(|transfer| crate::swap::check_swap_params(params, &transfer));
        if let Err(error) = checked {
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
        let Some(transfer) = tx.into_transfer(ctx.dynamic_token.as_ref()) else {
            log::error!("Unsupported asset");
            return Err(AppSW::TxParsingFail);
        };
        log::debug!("Normal mode, showing UI");
        let comm = command.into_comm();
        let approved = ui_display_tx(comm, &transfer, &ctx.path)?;
        ctx.review_finished = true;
        if approved {
            compute_signature_and_append(comm.begin_response(), ctx)
        } else {
            Err(AppSW::Deny)
        }
    }
    // --8<-- [end:ui_bypass]
}

/// Parses the accumulated transaction in the format of `kind`, requiring that every text field
/// can be displayed.
fn parse_tx(raw_tx: &[u8], kind: TxKind) -> Result<Tx<'_>, AppSW> {
    let tx = match kind {
        TxKind::Native => Tx::Native(deserialize_tx(raw_tx)?),
        TxKind::Token => Tx::Token(deserialize_tx(raw_tx)?),
    };
    let displayable = match &tx {
        Tx::Native(tx) => !tx.coin.is_empty() && is_displayable(tx.coin) && is_displayable(tx.memo),
        Tx::Token(tx) => is_displayable(tx.memo),
    };
    if !displayable {
        return Err(AppSW::TxParsingFail);
    }
    Ok(tx)
}

/// Deserializes a JSON transaction, requiring that every signed byte was consumed.
fn deserialize_tx<'a, T: Deserialize<'a>>(raw_tx: &'a [u8]) -> Result<T, AppSW> {
    let (tx, consumed) = from_slice(raw_tx).map_err(|_| AppSW::TxParsingFail)?;
    if consumed != raw_tx.len() {
        return Err(AppSW::TxParsingFail);
    }
    Ok(tx)
}

/// Text shown for review is restricted to printable ASCII: control characters (NUL included) and
/// non-ASCII characters cannot be rendered faithfully, and a NUL would make the SDK panic when it
/// builds the C strings of the review.
///
/// Backslashes are rejected as well: strings are borrowed from the signed JSON without unescaping,
/// so an escape sequence would be displayed literally while JSON consumers decode it, e.g.
/// `\u0041` shown as such but read as `A`.
fn is_displayable(text: &str) -> bool {
    text.bytes()
        .all(|b| b != b'\\' && (b == b' ' || b.is_ascii_graphic()))
}

fn compute_signature_and_append<'a>(
    mut response: CommandResponse<'a>,
    ctx: &mut TxContext,
) -> Result<CommandResponse<'a>, AppSW> {
    log::debug!("Signing transaction");
    let mut keccak256 = Keccak256::new();
    let mut message_hash: [u8; 32] = [0u8; 32];

    keccak256
        .hash(&ctx.raw_tx, &mut message_hash)
        .map_err(|_| AppSW::TxHashFail)?;

    let (sig, siglen, parity) = Secp256k1::derive_from_path(ctx.path.as_ref())
        .deterministic_sign(&message_hash)
        .map_err(|_| AppSW::TxSignFail)?;

    response
        .append(&[siglen as u8])?
        .append(&sig[..siglen as usize])?
        .append(&[parity as u8])?;
    Ok(response)
}
