/*****************************************************************************
 *   Ledger App Conflux Rust.
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

use crate::cfx_addr::{cfx_addr_encode, Network};
use crate::consts::ADDRRESS_BYTES_LEN;
use crate::AppSW;

#[cfg(not(any(target_os = "stax", target_os = "flex")))]
use ledger_device_sdk::ui::{
    bitmaps::{CROSSMARK, EYE, VALIDATE_14},
    gadgets::{Field, MultiFieldReview},
};

#[cfg(any(target_os = "stax", target_os = "flex"))]
use ledger_device_sdk::nbgl::{NbglAddressReview, NbglGlyph};

#[cfg(any(target_os = "stax", target_os = "flex"))]
use include_gif::include_gif;

pub fn ui_display_pk(addr: &[u8]) -> Result<bool, AppSW> {
    let addr = &addr[addr.len() - ADDRRESS_BYTES_LEN..]; // last 20 bytes
    let network = Network::from_network_id(1029);
    let cfx_addr = cfx_addr_encode(addr, network).map_err(|_e| AppSW::AddrDisplayFail)?;

    #[cfg(not(any(target_os = "stax", target_os = "flex")))]
    {
        let my_field = [Field {
            name: "Address",
            value: cfx_addr.as_str(),
        }];

        let my_review = MultiFieldReview::new(
            &my_field,
            &["Confirm Address"],
            Some(&EYE),
            "Approve",
            Some(&VALIDATE_14),
            "Reject",
            Some(&CROSSMARK),
        );

        Ok(my_review.show())
    }

    #[cfg(any(target_os = "stax", target_os = "flex"))]
    {
        // Load glyph from 64x64 4bpp gif file with include_gif macro. Creates an NBGL compatible glyph.
        const FERRIS: NbglGlyph = NbglGlyph::from_include(include_gif!("icons/cfx_64.gif", NBGL));
        // Display the address confirmation screen.
        Ok(NbglAddressReview::new()
            .glyph(&FERRIS)
            .verify_str("Verify CFX address")
            .show(&cfx_addr))
    }
}
