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
use core::str::from_utf8_mut;
use ledger_device_sdk::ui::bitmaps::{CROSSMARK, EYE, VALIDATE_14};
use ledger_device_sdk::ui::gadgets::{Field, MultiFieldReview};

// Display only the last 20 bytes of the address
const DISPLAY_ADDR_BYTES_LEN: usize = 20;

pub fn ui_display_pk(addr: &[u8]) -> Result<bool, AppSW> {
    let mut addr_hex = [0u8; DISPLAY_ADDR_BYTES_LEN * 2 + 2];
    addr_hex[..2].copy_from_slice("0x".as_bytes());
    hex::encode_to_slice(
        &addr[addr.len() - DISPLAY_ADDR_BYTES_LEN..],
        &mut addr_hex[2..],
    )
    .unwrap();
    let addr_hex = from_utf8_mut(&mut addr_hex).unwrap();
    addr_hex[2..].make_ascii_uppercase();

    let my_field = [Field {
        name: "Address",
        value: addr_hex,
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
