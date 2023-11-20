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
use crate::AppSW;
use ledger_device_ui_sdk::bitmaps::{CROSSMARK, EYE, VALIDATE_14};
use ledger_device_ui_sdk::ui::{Field, MultiFieldReview};

pub fn ui_display_tx(tx: &Tx) -> Result<bool, AppSW> {
    // Define transaction review fields
    let my_fields = [
        Field {
            name: "Amount",
            value: tx.value,
        },
        Field {
            name: "Destination",
            value: tx.to,
        },
        Field {
            name: "Memo",
            value: tx.memo,
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
