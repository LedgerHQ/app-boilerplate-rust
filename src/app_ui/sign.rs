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
use crate::cashnotes::Spend;
use crate::utils::{addr_hex_for_ui, concatenate, BUFFER_LEN_FOR_PK_BYTES_TO_DISPLAY};
use crate::AppSW;
use ledger_device_sdk::ui::bitmaps::{CROSSMARK, EYE, VALIDATE_14};
use ledger_device_sdk::ui::gadgets::{Field, MultiFieldReview};
use numtoa::NumToA;

const MAX_COIN_LENGTH: usize = 10;

/// Displays a transaction and returns true if user approved it.
///
/// This method can return [`AppSW::TxDisplayFail`] error if the coin name length is too long.
///
/// # Arguments
///
/// * `spend` - Spend/transaction to be displayed for validation
pub fn ui_display_tx(spend: &Spend) -> Result<bool, AppSW> {
    // Generate string for amount
    let mut numtoa_buf = [0u8; 20];
    let mut value_buf = [0u8; 20 + MAX_COIN_LENGTH + 1];

    let value_str = concatenate(
        &[
            "SNT ",
            spend.token.as_nano().numtoa_str(10, &mut numtoa_buf),
        ],
        &mut value_buf,
    )
    .map_err(|_| AppSW::TxDisplayFail)?; // Fails if value_buf is too small

    let spend_dest = spend.spent_tx.outputs[0].unique_pubkey;
    let mut addr_value_buf = [0u8; BUFFER_LEN_FOR_PK_BYTES_TO_DISPLAY];
    let addr_str = addr_hex_for_ui(&spend_dest, &mut addr_value_buf)?;

    // Define transaction review fields
    let my_fields = [
        Field {
            name: "Amount",
            value: value_str,
        },
        Field {
            name: "Destination",
            value: addr_str,
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
