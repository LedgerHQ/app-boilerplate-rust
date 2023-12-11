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
use crate::utils::{read_bip32_path, MAX_ALLOWED_PATH_LEN};
use crate::AppSW;
use ledger_device_sdk::ecc::{Secp256k1, SeedDerive};
use ledger_device_sdk::io::Comm;
use ledger_secure_sdk_sys::{
    cx_hash_no_throw, cx_hash_t, cx_keccak_init_no_throw, cx_sha3_t, CX_LAST, CX_OK,
};

use serde::Deserialize;
use serde_json_core::from_slice;

const MAX_TRANSACTION_LEN: usize = 510;

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

pub struct TxContext {
    raw_tx: [u8; MAX_TRANSACTION_LEN], // raw transaction serialized
    raw_tx_len: usize,                 // length of raw transaction
    path: [u32; MAX_ALLOWED_PATH_LEN], // BIP32 path for key derivation
    path_len: usize,                   // length of BIP32 path
}

// Implement constructor for TxInfo with default values
impl TxContext {
    pub fn new() -> TxContext {
        TxContext {
            raw_tx: [0u8; MAX_TRANSACTION_LEN],
            raw_tx_len: 0,
            path: [0u32; MAX_ALLOWED_PATH_LEN],
            path_len: 0,
        }
    }
    // Implement reset for TxInfo
    fn reset(&mut self) {
        self.raw_tx = [0u8; MAX_TRANSACTION_LEN];
        self.raw_tx_len = 0;
        self.path = [0u32; MAX_ALLOWED_PATH_LEN];
        self.path_len = 0;
    }
}

pub fn handler_sign_tx(
    comm: &mut Comm,
    chunk: u8,
    more: bool,
    ctx: &mut TxContext,
) -> Result<(), AppSW> {
    // Try to get data from comm
    let data = comm.get_data().map_err(|_| AppSW::WrongApduLength)?;
    // First chunk, try to parse the path
    if chunk == 0 {
        // Reset transaction context
        ctx.reset();
        // This will propagate the error if the path is invalid
        ctx.path_len = read_bip32_path(data, &mut ctx.path)?;
        Ok(())
    // Next chunks, append data to raw_tx and return or parse
    // the transaction if it is the last chunk.
    } else {
        if ctx.raw_tx_len + data.len() > MAX_TRANSACTION_LEN {
            return Err(AppSW::TxWrongLength);
        }

        // Append data to raw_tx
        ctx.raw_tx[ctx.raw_tx_len..ctx.raw_tx_len + data.len()].copy_from_slice(data);
        ctx.raw_tx_len += data.len();

        // If we expect more chunks, return
        if more {
            Ok(())
        // Otherwise, try to parse the transaction
        } else {
            // Try to deserialize the transaction
            let (tx, _): (Tx, usize) =
                from_slice(&ctx.raw_tx[..ctx.raw_tx_len]).map_err(|_| AppSW::TxParsingFail)?;
            // Display transaction. If user approves
            // the transaction, sign it. Otherwise,
            // return a "deny" status word.
            if ui_display_tx(&tx)? {
                compute_signature_and_append(comm, ctx)
            } else {
                Err(AppSW::Deny)
            }
        }
    }
}

fn compute_signature_and_append(comm: &mut Comm, ctx: &mut TxContext) -> Result<(), AppSW> {
    let mut keccak256: cx_sha3_t = Default::default();
    let mut message_hash: [u8; 32] = [0u8; 32];

    unsafe {
        if cx_keccak_init_no_throw(&mut keccak256, 256) != CX_OK {
            return Err(AppSW::TxHashFail);
        }
        if cx_hash_no_throw(
            &mut keccak256.header as *mut cx_hash_t,
            CX_LAST,
            ctx.raw_tx.as_ptr(),
            ctx.raw_tx_len,
            message_hash.as_mut_ptr(),
            message_hash.len(),
        ) != CX_OK
        {
            return Err(AppSW::TxHashFail);
        }
    }

    let (sig, siglen, parity) = Secp256k1::derive_from_path(&ctx.path[..ctx.path_len])
        .deterministic_sign(&message_hash)
        .map_err(|_| AppSW::TxSignFail)?;
    comm.append(&[siglen as u8]);
    comm.append(&sig[..siglen as usize]);
    comm.append(&[parity as u8]);
    Ok(())
}
