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
use crate::cashnotes::{Hash, Input, NanoTokens, Output, Spend, Transaction};
use crate::utils::{concat_and_debug_print, read_bip32_path, MAX_ALLOWED_PATH_LEN};
use crate::AppSW;
use ledger_device_sdk::ecc::BLSPrivateKey;
use ledger_device_sdk::io::Comm;
use ledger_device_sdk::testing;

const MAX_TRANSACTION_LEN: usize = 500;

// Domain separation tag used by SAFE nodes.
const DOMAIN_SEPARATION_TAG: &[u8; 43] = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_";

pub struct TxContext {
    raw_spend: [u8; MAX_TRANSACTION_LEN], // raw transaction serialized
    raw_spend_len: usize,                 // length of raw transaction
    path: [u32; MAX_ALLOWED_PATH_LEN],    // EIP2333 path for key derivation
    path_len: usize,                      // length of BIP32 path
}

// Implement constructor for TxInfo with default values
impl TxContext {
    pub fn new() -> TxContext {
        TxContext {
            raw_spend: [0u8; MAX_TRANSACTION_LEN],
            raw_spend_len: 0,
            path: [0u32; MAX_ALLOWED_PATH_LEN],
            path_len: 0,
        }
    }
    // Implement reset for TxInfo
    fn reset(&mut self) {
        self.raw_spend = [0u8; MAX_TRANSACTION_LEN];
        self.raw_spend_len = 0;
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
    testing::debug_print("handling sign_tx command\n");
    // Try to get data from comm
    let data = match comm.get_data() {
        Ok(data) => data,
        Err(_) => return Err(AppSW::WrongDataLength),
    };

    // First chunk, try to parse the path
    if chunk == 0 {
        testing::debug_print("handling sign_tx command first chunk\n");
        // Reset transaction context
        ctx.reset();
        // This will propagate the error if the path is invalid
        ctx.path_len = read_bip32_path(data, &mut ctx.path)?;
    // Next chunks, append data to raw_spend and return or parse
    // the transaction if it is the last chunk.
    } else {
        concat_and_debug_print("sign_tx command chunk #", chunk as usize)?;
        if ctx.raw_spend_len + data.len() > MAX_TRANSACTION_LEN {
            return Err(AppSW::TxWrongLength);
        }

        // Append data to raw_spend
        ctx.raw_spend[ctx.raw_spend_len..ctx.raw_spend_len + data.len()].copy_from_slice(data);
        ctx.raw_spend_len += data.len();

        // If we expect more chunks, return
        if more {
            testing::debug_print("we expect more chunks\n");
            return Ok(());
        // Otherwise, try to parse the transaction
        } else {
            testing::debug_print("we DO NOT expect more chunks\n");

            // Try to deserialize the transaction
            let spend = deserialise_spend_from_slice(&ctx.raw_spend[..ctx.raw_spend_len])?;

            // Display transaction. If user approves
            // the transaction, sign it. Otherwise,
            // return a "deny" status word.
            if ui_display_tx(&spend)? {
                return compute_signature_and_append(comm, ctx);
            } else {
                return Err(AppSW::Deny);
            }
        }
    }
    Ok(())
}

fn compute_signature_and_append(comm: &mut Comm, ctx: &mut TxContext) -> Result<(), AppSW> {
    let mut bls_sk = BLSPrivateKey::derive_from_path(&ctx.path[..ctx.path_len]);
    testing::debug_print("BLS SK derived!");

    let hash = bls_sk
        .hash_to_field(&ctx.raw_spend, DOMAIN_SEPARATION_TAG)
        .map_err(|_| AppSW::TxHashFail)?;

    let (sig, siglen) = bls_sk.sign(&hash).map_err(|_| AppSW::TxSignFail)?;

    comm.append(&[siglen as u8]);
    comm.append(&sig[..siglen as usize]);

    Ok(())
}

fn deserialise_spend_from_slice(spend_bytes: &[u8]) -> Result<Spend, AppSW> {
    testing::debug_print("deserialising Spend...");

    let unique_pubkey = deserialize_pubkey_from_slice(&spend_bytes)?;

    let (spent_tx, len) = deserialise_transaction_from_slice(&spend_bytes[unique_pubkey.len()..])?;

    let offset = unique_pubkey.len() + len;
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&spend_bytes[offset..offset + 32]);

    let offset = offset + hash.len();
    let (token, len) = deserialise_nanotokens_from_slice(&spend_bytes[offset..])?;

    let offset = offset + len;
    let (parent_tx, _len) = deserialise_transaction_from_slice(&spend_bytes[offset..])?;

    let spend = Spend {
        unique_pubkey,
        spent_tx,
        reason: Hash(hash),
        token,
        parent_tx,
    };

    Ok(spend)
}

fn deserialize_pubkey_from_slice(pubkey_bytes: &[u8]) -> Result<[u8; 48], AppSW> {
    let mut unique_pubkey = [0u8; 48];
    unique_pubkey.copy_from_slice(&pubkey_bytes[..48]);
    Ok(unique_pubkey)
}

fn deserialise_transaction_from_slice(
    transaction_bytes: &[u8],
) -> Result<(Transaction, usize), AppSW> {
    let (input, input_len) = deserialise_input_from_slice(transaction_bytes)?;
    let (output, output_len) = deserialise_output_from_slice(&transaction_bytes[input_len..])?;
    let transaction = Transaction {
        inputs: [input],
        outputs: [output],
    };

    Ok((transaction, input_len + output_len))
}

fn deserialise_input_from_slice(input_bytes: &[u8]) -> Result<(Input, usize), AppSW> {
    let unique_pubkey = deserialize_pubkey_from_slice(input_bytes)?;
    let (amount, len) = deserialise_nanotokens_from_slice(&input_bytes[unique_pubkey.len()..])?;
    let input = Input {
        unique_pubkey,
        amount,
    };

    Ok((input, unique_pubkey.len() + len))
}

fn deserialise_output_from_slice(output_bytes: &[u8]) -> Result<(Output, usize), AppSW> {
    let unique_pubkey = deserialize_pubkey_from_slice(output_bytes)?;
    let (amount, len) = deserialise_nanotokens_from_slice(&output_bytes[unique_pubkey.len()..])?;
    let output = Output {
        unique_pubkey,
        amount,
    };

    Ok((output, unique_pubkey.len() + len))
}

fn deserialise_nanotokens_from_slice(
    nanotokens_bytes: &[u8],
) -> Result<(NanoTokens, usize), AppSW> {
    let mut nanos_bytes = [0u8; 8];
    nanos_bytes.copy_from_slice(&nanotokens_bytes[..8]);
    let nanos = u64::from_ne_bytes(nanos_bytes);

    Ok((NanoTokens::from(nanos), 8))
}
