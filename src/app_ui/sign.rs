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
use crate::utils::concatenate;
use crate::AppSW;

use ledger_secure_sdk_sys::*;

#[cfg(not(target_os = "stax"))]
use ledger_device_sdk::ui::bitmaps::{CROSSMARK, EYE, VALIDATE_14};
#[cfg(not(target_os = "stax"))]
use ledger_device_sdk::ui::gadgets::{Field, MultiFieldReview};

#[cfg(target_os = "stax")]
use include_gif::include_gif;
#[cfg(target_os = "stax")]
use ledger_device_sdk::nbgl::{Field, NbglGlyph, NbglReview};

use numtoa::NumToA;

const MAX_COIN_LENGTH: usize = 10;

/// Displays a transaction and returns true if user approved it.
///
/// This method can return [`AppSW::TxDisplayFail`] error if the coin name length is too long.
///
/// # Arguments
///
/// * `tx` - Transaction to be displayed for validation
pub fn ui_display_tx(tx: &Tx) -> Result<bool, AppSW> {
    // Generate string for amount
    let mut numtoa_buf = [0u8; 20];
    let mut value_buf = [0u8; 20 + MAX_COIN_LENGTH + 1];

    let value_str = concatenate(
        &[tx.coin, " ", tx.value.numtoa_str(10, &mut numtoa_buf)],
        &mut value_buf,
    )
    .map_err(|_| AppSW::TxDisplayFail)?; // Fails if value_buf is too small

    // Generate destination address string in hexadecimal format.
    let mut to_str = [0u8; 42];
    to_str[..2].copy_from_slice("0x".as_bytes());
    hex::encode_to_slice(tx.to, &mut to_str[2..]).unwrap();
    to_str[2..].make_ascii_uppercase();

    // Define transaction review fields
    let my_fields = [
        Field {
            name: "Amount",
            value: value_str,
        },
        Field {
            name: "Destination",
            value: core::str::from_utf8(&to_str).unwrap(),
        },
        Field {
            name: "Memo",
            value: tx.memo,
        },
    ];

    // Create transaction review
    #[cfg(not(target_os = "stax"))]
    {
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

    #[cfg(target_os = "stax")]
    {
        // This will copy the fields into the nbgl format, not ideal
        // memory-wise...
        let nbgl_fields: [nbgl_layoutTagValue_t; 3] = [
            my_fields[0].into(),
            my_fields[1].into(),
            my_fields[2].into(),
        ];

        const FERRIS: NbglGlyph = NbglGlyph::from_include(include_gif!("crab_64x64.gif", NBGL));
        Ok(NbglReview::new()
            .status_strings("TRANSACTION\nSIGNED\0", "Transaction\nRejected\0")
            .glyph(&FERRIS)
            .show(&nbgl_fields))
    }
}
