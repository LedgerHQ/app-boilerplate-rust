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

use crate::handlers::sign_tx::Tx;
use crate::utils::{concatenate, to_hex_all_caps};
use crate::AppSW;
use core::str::from_utf8;
use ledger_device_ui_sdk::bitmaps::{CROSSMARK, EYE, VALIDATE_14};
use ledger_device_ui_sdk::ui::{Field, MultiFieldReview};
use numtoa::NumToA;

pub fn ui_display_tx(tx: &Tx) -> Result<bool, AppSW> {
    // Format amount value
    let mut amount_buf = [0u8; 20];
    let mut amount_with_denom_buf = [0u8; 25];
    concatenate(
        &["CRAB", " ", tx.value.numtoa_str(10, &mut amount_buf)],
        &mut amount_with_denom_buf,
    );
    let amount_str_with_denom = from_utf8(&amount_with_denom_buf)
        .map_err(|_| AppSW::TxDisplayFail)?
        .trim_matches(char::from(0));

    // Format destination address
    let hex_addr_buf = to_hex_all_caps(tx.to).map_err(|_| AppSW::TxDisplayFail)?;
    let hex_addr_str = from_utf8(&hex_addr_buf).map_err(|_| AppSW::TxDisplayFail)?;
    let mut addr_with_prefix_buf = [0u8; 42];
    concatenate(&["0x", hex_addr_str], &mut addr_with_prefix_buf);
    let hex_addr_str_with_prefix =
        from_utf8(&addr_with_prefix_buf).map_err(|_| AppSW::TxDisplayFail)?;

    // Format memo
    let memo_str = from_utf8(&tx.memo[..tx.memo_len]).map_err(|_| AppSW::TxDisplayFail)?;

    // Define transaction review fields
    let my_fields = [
        Field {
            name: "Amount",
            value: amount_str_with_denom,
        },
        Field {
            name: "Destination",
            value: hex_addr_str_with_prefix,
        },
        Field {
            name: "Memo",
            value: memo_str,
        },
    ];

    // Create transaction review
    let my_review = MultiFieldReview::new(
        &my_fields,
        &["Review ", "Transaction"],
        Some(&EYE),
        "Approve",
        Some(&VALIDATE_14),
        "Reject",
        Some(&CROSSMARK),
    );

    Ok(my_review.show())
}
