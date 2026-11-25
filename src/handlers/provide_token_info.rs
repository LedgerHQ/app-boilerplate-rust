/*****************************************************************************
 *   Ledger App Boilerplate Rust.
 *   (c) 2026 Ledger SAS.
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

//! PROVIDE_TOKEN_INFO: receives a token descriptor signed by Ledger.
//!
//! ADVANCED FEATURE: the descriptors are signed by the Ledger CAL (Crypto Assets List), so a token
//! can only be described this way once Ledger knows it. Most apps only need the hardcoded token
//! list, see `doc/TOKENS.md`.

use crate::handlers::sign_tx::TxContext;
use crate::token::parse_descriptor;
use crate::AppSW;
use ledger_device_sdk::io::{Command, CommandResponse};
use ledger_device_sdk::log;

/// Handler for the PROVIDE_TOKEN_INFO APDU.
///
/// Stores the token of the descriptor in the slot of `ctx`, replacing the previous one. The slot is
/// emptied first, so a rejected descriptor also discards the previous token.
///
/// Any pending signing flow is ended: the token of a flow is resolved on its last chunk, and the
/// descriptor must not change after the flow started. A descriptor is therefore sent before the
/// path chunk of the transaction it describes.
///
/// Parsing allocates. This handler only runs from the APDU loop, in normal mode or in the signing
/// phase of a swap, after the BSS reset.
pub fn handler_provide_token_info<'a>(
    command: Command<'a>,
    ctx: &mut TxContext,
) -> Result<CommandResponse<'a>, AppSW> {
    log::debug!("=> handler_provide_token_info");
    ctx.clear_flow();
    ctx.set_dynamic_token(None);

    let token = parse_descriptor(command.get_data()).map_err(|error| {
        log::error!("Token descriptor rejected: {:?}", error);
        AppSW::InvalidDynamicToken
    })?;
    ctx.set_dynamic_token(Some(token));
    Ok(command.into_response())
}
