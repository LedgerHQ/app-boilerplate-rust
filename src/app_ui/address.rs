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

use crate::AppSW;
use alloc::format;

use include_gif::include_gif;
use ledger_device_sdk::nbgl::{NbglAddressReview, NbglGlyph};

// Display only the last 20 bytes of the address
const DISPLAY_ADDR_BYTES_LEN: usize = 20;

pub fn ui_display_pk(addr: &[u8]) -> Result<bool, AppSW> {
    let addr_hex = format!(
        "0x{}",
        hex::encode(&addr[addr.len() - DISPLAY_ADDR_BYTES_LEN..]).to_uppercase()
    );

    // Load glyph from 64x64 4bpp gif file with include_gif macro. Creates an NBGL compatible glyph.
    #[cfg(any(target_os = "stax", target_os = "flex"))]
    const FERRIS: NbglGlyph = NbglGlyph::from_include(include_gif!("crab_64x64.gif", NBGL));
    #[cfg(any(target_os = "nanosplus", target_os = "nanox"))]
    const FERRIS: NbglGlyph = NbglGlyph::from_include(include_gif!("crab_16x16.gif", NBGL));

    // Display the address confirmation screen.
    Ok(NbglAddressReview::new()
        .glyph(&FERRIS)
        .verify_str("Verify CRAB address")
        .show(&addr_hex))
}
