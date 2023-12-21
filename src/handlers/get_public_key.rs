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

use crate::app_ui::address::ui_display_pk;
use crate::utils::{read_bip32_path, MAX_ALLOWED_PATH_LEN};
use crate::AppSW;
use ledger_device_sdk::ecc::BLSPrivateKey;
use ledger_device_sdk::io::Comm;
use ledger_device_sdk::testing;

pub fn handler_get_public_key(comm: &mut Comm, display: bool) -> Result<(), AppSW> {
    testing::debug_print("handling get_public_key command\n");

    let mut path = [0u32; MAX_ALLOWED_PATH_LEN];
    let data = match comm.get_data() {
        Ok(data) => data,
        Err(_) => return Err(AppSW::WrongDataLength),
    };

    let path_len = read_bip32_path(data, &mut path)?;

    let bls_sk = BLSPrivateKey::derive_from_path(&path[..path_len]);

    let pk = bls_sk.public_key().map_err(|_| AppSW::PubKeyDerivFail)?;
    testing::debug_print("BLS PK derived!");

    let mut addr_value_buf = [0u8; crate::utils::BUFFER_LEN_FOR_PK_BYTES_TO_DISPLAY];
    let addr_hex = crate::utils::addr_hex_for_ui(&pk.pubkey, &mut addr_value_buf)?;
    testing::debug_print(addr_hex);

    // Display address on device if requested
    if display {
        testing::debug_print("showing public key\n");
        if !ui_display_pk(&pk.pubkey)? {
            testing::debug_print("denied\n");
            return Err(AppSW::Deny);
        }
        testing::debug_print("shown and approved\n");
    }

    comm.append(&[pk.pubkey.len() as u8]);
    comm.append(&pk.pubkey);
    // Rust SDK key derivation API does not return chaincode yet
    // so we just append a dummy chaincode.
    const CHAINCODE_LEN: usize = 32;
    comm.append(&[CHAINCODE_LEN as u8]); // Dummy chaincode length
    comm.append(&[0u8; CHAINCODE_LEN]); // Dummy chaincode

    testing::debug_print("DONE!!!\n");

    Ok(())
}
