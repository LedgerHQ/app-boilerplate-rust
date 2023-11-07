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

use crate::utils;
use core::str::from_utf8;
use nanos_sdk::io;
use nanos_ui::bitmaps::{CROSSMARK, EYE, VALIDATE_14};
use nanos_ui::ui::{Field, MultiFieldReview};

pub fn ui_display_pk(pk: &[u8]) -> Result<bool, io::Reply> {
    // Todo add error handling
    // ======================
    let hex = utils::to_hex(pk).unwrap();
    let m = from_utf8(&hex).unwrap();
    // ======================

    let my_field = [Field {
        name: "Public Key",
        value: m[..pk.len() * 2].as_ref(),
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
