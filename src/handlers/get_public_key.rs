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
use crate::SW_DENY;
use nanos_sdk::ecc::{Secp256k1, SeedDerive};
use nanos_sdk::{io, testing};

const MAX_ALLOWED_PATH_LEN: usize = 10;

// const SW_DENY: u16 = 0x6985;

pub fn handler_get_public_key(comm: &mut io::Comm, display: bool) -> Result<(), io::Reply> {
    let mut path = [0u32; MAX_ALLOWED_PATH_LEN];
    let data = comm.get_data()?;

    let path_len = read_bip32_path(data, &mut path)?;

    let pk = Secp256k1::derive_from_path(&path[..path_len])
        .public_key()
        .map_err(|x| io::Reply(0x6eu16 | (x as u16 & 0xff)))?;

    // Display public key on device if requested
    if display {
        testing::debug_print("showing public key\n");
        if !ui_display_pk(&pk.pubkey)? {
            testing::debug_print("denied\n");
            return Err(io::Reply(SW_DENY));
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

fn read_bip32_path(data: &[u8], path: &mut [u32]) -> Result<usize, io::Reply> {
    // Check input length and path buffer capacity
    if data.len() < 1 || path.len() < data.len() / 4 {
        return Err(io::StatusWords::BadLen.into());
    }

    let path_len = data[0] as usize; // First byte is the length of the path
    let path_data = &data[1..];

    // Check path data length and alignment
    if path_data.len() != path_len * 4
        || path_data.len() > MAX_ALLOWED_PATH_LEN * 4
        || path_data.len() % 4 != 0
    {
        return Err(io::StatusWords::BadLen.into());
    }

    let mut idx = 0;
    for (i, chunk) in path_data.chunks(4).enumerate() {
        path[idx] = u32::from_be_bytes(chunk.try_into().unwrap());
        idx = i + 1;
    }

    Ok(idx)
}
