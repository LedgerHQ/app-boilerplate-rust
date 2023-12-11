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
use ledger_device_sdk::ecc::{Secp256k1, SeedDerive};
use ledger_device_sdk::io::Comm;
use ledger_device_sdk::testing;
use ledger_secure_sdk_sys::{
    cx_hash_no_throw, cx_hash_t, cx_keccak_init_no_throw, cx_sha3_t, CX_LAST, CX_OK,
};

pub fn handler_get_public_key(comm: &mut Comm, display: bool) -> Result<(), AppSW> {
    let mut path = [0u32; MAX_ALLOWED_PATH_LEN];
    let data = comm.get_data().map_err(|_| AppSW::WrongApduLength)?;
    let path_len = read_bip32_path(data, &mut path)?;

    let pk = Secp256k1::derive_from_path(&path[..path_len])
        .public_key()
        .map_err(|_| AppSW::KeyDeriveFail)?;

    // Display address on device if requested
    if display {
        let mut keccak256: cx_sha3_t = Default::default();
        let mut address: [u8; 32] = [0u8; 32];

        unsafe {
            if cx_keccak_init_no_throw(&mut keccak256, 256) != CX_OK {
                return Err(AppSW::AddrDisplayFail);
            }

            let mut pk_mut = pk.pubkey;
            let pk_ptr = pk_mut.as_mut_ptr().offset(1);
            if cx_hash_no_throw(
                &mut keccak256.header as *mut cx_hash_t,
                CX_LAST,
                pk_ptr,
                64_usize,
                address.as_mut_ptr(),
                address.len(),
            ) != CX_OK
            {
                return Err(AppSW::AddrDisplayFail);
            }
        }

        testing::debug_print("showing public key\n");
        if !ui_display_pk(&address)? {
            testing::debug_print("denied\n");
            return Err(AppSW::Deny);
        }
    }

    comm.append(&[pk.pubkey.len() as u8]);
    comm.append(&pk.pubkey);
    // Rust SDK key derivation API does not return chaincode yet
    // so we just append a dummy chaincode.
    const CHAINCODE_LEN: usize = 32;
    comm.append(&[CHAINCODE_LEN as u8]); // Dummy chaincode length
    comm.append(&[0u8; CHAINCODE_LEN]); // Dummy chaincode

    Ok(())
}
