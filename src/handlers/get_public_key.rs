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
use crate::utils::Bip32Path;
use crate::AppSW;
use ledger_device_sdk::ecc::{Secp256k1, SeedDerive};
use ledger_device_sdk::hash::{sha3::Keccak256, HashInit};
use ledger_device_sdk::io::{self, Command};

pub fn handler_get_public_key(
    command: Command,
    display: bool,
) -> Result<io::CommandResponse, AppSW> {
    let data = command.get_data();
    let path: Bip32Path = data.try_into()?;

    let (k, cc) = Secp256k1::derive_from(path.as_ref());
    let pk = k.public_key().map_err(|_| AppSW::KeyDeriveFail)?;

    // Display address on device if requested
    if display {
        let mut keccak256 = Keccak256::new();
        let mut address: [u8; 32] = [0u8; 32];

        let _ = keccak256.hash(&pk.pubkey[1..], &mut address);

        if !ui_display_pk(&address)? {
            return Err(AppSW::Deny);
        }
    }

    let mut response = command.into_response();
    response.append(&[pk.pubkey.len() as u8])?;
    response.append(&pk.pubkey)?;

    const CHAINCODE_LEN: u8 = 32;
    let code = cc.unwrap();
    response.append(&[CHAINCODE_LEN])?;
    response.append(&code.value)?;

    Ok(response)
}
