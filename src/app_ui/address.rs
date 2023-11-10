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

use crate::utils::{concatenate, to_hex_all_caps};
use crate::SW_DISPLAY_ADDRESS_FAIL;
use core::str::from_utf8;
use nanos_sdk::io::Reply;
use nanos_ui::bitmaps::{CROSSMARK, EYE, VALIDATE_14};
use nanos_ui::ui::{Field, MultiFieldReview};

// Display only the last 20 bytes of the address
const DISPLAY_ADDR_BYTES_LEN: usize = 20;

pub fn ui_display_pk(addr: &[u8]) -> Result<bool, Reply> {
    let addr_hex_str_buf =
        to_hex_all_caps(&addr[addr.len() - DISPLAY_ADDR_BYTES_LEN as usize..])
            .map_err(|_| Reply(SW_DISPLAY_ADDRESS_FAIL))?;
    let addr_hex_str = from_utf8(&addr_hex_str_buf[..DISPLAY_ADDR_BYTES_LEN * 2])
        .map_err(|_| Reply(SW_DISPLAY_ADDRESS_FAIL))?;

    let mut addr_hex_str_with_prefix_buf = [0u8; DISPLAY_ADDR_BYTES_LEN * 2 + 2];
    concatenate(
        &["0x", &addr_hex_str],
        &mut addr_hex_str_with_prefix_buf,
    );
    let addr_hex_str_with_prefix =
        from_utf8(&addr_hex_str_with_prefix_buf).map_err(|_| Reply(SW_DISPLAY_ADDRESS_FAIL))?;

    let my_field = [Field {
        name: "Address",
        value: addr_hex_str_with_prefix,
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
