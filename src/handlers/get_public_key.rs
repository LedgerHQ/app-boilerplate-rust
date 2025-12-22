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
use crate::utils::{get_address_hash_from_pubkey, get_pubkey_from_path, Bip32Path};
use crate::AppSW;
use ledger_device_sdk::ecc::{Secp256k1, SeedDerive};
use ledger_device_sdk::io::Comm;

/// Handler for GET_PUBLIC_KEY APDU command.
///
/// Derives and returns the public key for a given BIP32 path, optionally
/// displaying the corresponding address on the device for user verification.
///
/// # Flow
///
/// 1. Parse BIP32 path from APDU data
/// 2. Derive public key using shared helper `get_pubkey_from_path()`
/// 3. If display requested, compute and show address on device
/// 4. Return public key and chaincode to client
///
/// # Note
///
/// This handler uses the same address derivation logic as `swap::check_address()`
/// via the shared `get_address_hash_from_pubkey()` helper, ensuring consistency.
pub fn handler_get_public_key(comm: &mut Comm, display: bool) -> Result<(), AppSW> {
    let data = comm.get_data().map_err(|_| AppSW::WrongApduLength)?;
    let path: Bip32Path = data.try_into()?;

    // Derive public key using shared helper (also used by swap)
    let pubkey = get_pubkey_from_path(&path)?;
    let (_, cc) = Secp256k1::derive_from(path.as_ref());

    // Display address on device if requested
    if display {
        // Compute address using shared helper (same as swap::check_address)
        let address_hash = get_address_hash_from_pubkey(&pubkey);

        if !ui_display_pk(&address_hash)? {
            return Err(AppSW::Deny);
        }
    }

    // Return public key to client (65 bytes uncompressed)
    comm.append(&[pubkey.len() as u8]);
    comm.append(&pubkey);

    // Return chaincode
    const CHAINCODE_LEN: u8 = 32;
    let code = cc.unwrap();
    comm.append(&[CHAINCODE_LEN]);
    comm.append(&code.value);

    Ok(())
}
