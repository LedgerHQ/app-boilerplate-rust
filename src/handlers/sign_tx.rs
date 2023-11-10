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
use crate::utils::{read_bip32_path, varint_read, slice_or_err, MAX_ALLOWED_PATH_LEN, to_hex_all_caps};
use crate::{SW_DENY, SW_TX_HASH_FAIL, SW_TX_PARSING_FAIL, SW_TX_SIGN_FAIL, SW_WRONG_TX_LENGTH};
use nanos_sdk::bindings::{
    cx_hash_no_throw, cx_hash_t, cx_keccak_init_no_throw, cx_sha3_t, CX_LAST, CX_OK,
};
use nanos_sdk::ecc::{Secp256k1, SeedDerive};
use nanos_sdk::io::{Comm, Reply};
use nanos_sdk::testing;

const MAX_TRANSACTION_LEN: usize = 510;

pub struct Tx {
    nonce: u64,
    pub value: u64,
    pub to: [u8; 20],
    pub memo: [u8; 255],
    pub memo_len: usize,
}

// Implement deserialize for Tx from a u8 array
impl TryFrom<&[u8]> for Tx {
    type Error = ();
    fn try_from(raw_tx: &[u8]) -> Result<Self, Self::Error> {
        if raw_tx.len() > MAX_TRANSACTION_LEN {
            return Err(());
        }

        // Try to parse the transaction fields :
        // Nonce
        let nonce = u64::from_be_bytes(slice_or_err(raw_tx, 0, 8)?.try_into().map_err(|_| ())?);
        // Destination address
        let to = slice_or_err(raw_tx, 8, 20)?.try_into().map_err(|_| ())?;
        // Amount value
        let value = u64::from_be_bytes(slice_or_err(raw_tx, 28, 8)?.try_into().map_err(|_| ())?);
        // Memo length
        // Memo will be trimmed to 255 bytes if it is longer
        let (memo_len_u64, memo_len_size) = varint_read(&raw_tx[36..])?;
        let memo_len = if memo_len_u64 < 255 {
            memo_len_u64 as usize
        } else {
            255 as usize
        };

        // Memo
        let memo_slice = slice_or_err(raw_tx, 36 + memo_len_size, memo_len)?;
        let mut memo = [0u8; 255];

        memo[..memo_len].copy_from_slice(memo_slice);
        
        // Check memo ASCII encoding
        if !memo[..memo_len].iter().all(|&byte| byte.is_ascii()) {
            return Err(());
        }
        
        Ok(Tx {
            nonce,
            value,
            to,
            memo,
            memo_len,
        })
    }
}

// #[derive(Copy, Clone)]
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
) -> Result<(), Reply> {
    // Try to get data from comm. If there is no data,
    // the '?' operator will propagate the error.
    let data = comm.get_data()?;
    // First chunk, try to parse the path
    if chunk == 0 {
        // Reset transaction context
        ctx.reset();
        // This will propagate the error if the path is invalid
        ctx.path_len = read_bip32_path(data, &mut ctx.path)?;
    // Next chunks, append data to raw_tx and return or parse
    // the transaction if it is the last chunk.
    } else {
        if ctx.raw_tx_len + data.len() > MAX_TRANSACTION_LEN {
            return Err(Reply(SW_WRONG_TX_LENGTH));
        }

        // Append data to raw_tx
        ctx.raw_tx[ctx.raw_tx_len..ctx.raw_tx_len + data.len()].copy_from_slice(data);
        ctx.raw_tx_len += data.len();

        // If we expect more chunks, return
        if more {
            return Ok(());
        // Otherwise, try to parse the transaction
        } else {
            testing::debug_print("Last chunk : parse transaction\n");
            let tx = match Tx::try_from(&ctx.raw_tx[..ctx.raw_tx_len]) {
                Ok(tx) => tx,
                Err(_) => return Err(Reply(SW_TX_PARSING_FAIL)),
            };
            testing::debug_print("Transaction parsed\n");
            // Display transaction. If user approves
            // the transaction, sign it. Otherwise,
            // return an error.
            if ui_display_tx(&tx)? {
                return compute_signature_and_append(comm, ctx);
            } else {
                return Err(Reply(SW_DENY));
            }
        }
    }
    Ok(())
}

fn compute_signature_and_append(comm: &mut Comm, ctx: &mut TxContext) -> Result<(), Reply> {
    let mut keccak256: cx_sha3_t = Default::default();
    let mut message_hash: [u8; 32] = [0u8; 32];

    testing::debug_print("Signature is appended 1\n");
    unsafe {
        let res = cx_keccak_init_no_throw(&mut keccak256, 256);
        if res != CX_OK {
            // Print error
            let err_buf = to_hex_all_caps(&res.to_be_bytes()).unwrap();
            let err_str = core::str::from_utf8(&err_buf).unwrap();
            testing::debug_print("Hashing err : ");
            testing::debug_print(err_str);
            testing::debug_print("\n");
            
            return Err(Reply(SW_TX_HASH_FAIL));
        }
        testing::debug_print("Signature is appended 1.1\n");
        if cx_hash_no_throw(
            &mut keccak256.header as *mut cx_hash_t,
            CX_LAST,
            ctx.raw_tx.as_ptr(),
            ctx.raw_tx_len as u32,
            message_hash.as_mut_ptr(),
            message_hash.len() as u32,
        ) != CX_OK
        {
            testing::debug_print("Hashing failed 2\n");
            return Err(Reply(SW_TX_HASH_FAIL));
        }
        testing::debug_print("Signature is appended 1.2\n");
    }

    let (sig, siglen, parity) = Secp256k1::derive_from_path(&ctx.path[..ctx.path_len])
        .deterministic_sign(&message_hash)
        .map_err(|_| Reply(SW_TX_SIGN_FAIL))?;
    comm.append(&[siglen as u8]);
    comm.append(&sig[..siglen as usize]);
    comm.append(&[parity as u8]);

    testing::debug_print("Signature is appended 2\n");

    Ok(())
}
