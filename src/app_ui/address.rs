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

use crate::{
    utils::{addr_hex_for_ui, BUFFER_LEN_FOR_PK_BYTES_TO_DISPLAY},
    AppSW,
};
use ledger_device_sdk::ui::bitmaps::{CROSSMARK, EYE, VALIDATE_14};
use ledger_device_sdk::ui::gadgets::{Field, MultiFieldReview};

pub fn ui_display_pk(addr: &[u8]) -> Result<bool, AppSW> {
    let mut addr_value_buf = [0u8; BUFFER_LEN_FOR_PK_BYTES_TO_DISPLAY];
    let addr_hex = addr_hex_for_ui(addr, &mut addr_value_buf)?;

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
