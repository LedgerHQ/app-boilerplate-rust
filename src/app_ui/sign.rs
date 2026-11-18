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
use crate::utils::Bip32Path;
use crate::AppSW;

use crate::settings::{Settings, SETTING_SHOW_PATH};
use ledger_device_sdk::include_gif;
use ledger_device_sdk::io::Comm;
use ledger_device_sdk::nbgl::{Field, NbglGlyph, NbglReview};

use alloc::{format, string::ToString};

/// Displays a transaction and returns true if user approved it.
///
/// This method can return [`AppSW::TxDisplayFail`] error if the coin name length is too long.
///
/// # Arguments
///
/// * `comm` - Reference to the communication object
/// * `tx` - Transaction to be displayed for validation
/// * `path` - Derivation path of the signing key, displayed when enabled in the settings
///
/// Every field of the signed transaction except the nonce is always displayed: no setting may
/// hide data that is signed.
pub fn ui_display_tx(comm: &mut Comm, tx: &Tx, path: &Bip32Path) -> Result<bool, AppSW> {
    let value_str = format!("{} {}", tx.coin, tx.value);
    let to_str = format!("0x{}", hex::encode(tx.to).to_uppercase());
    let path_str = path.to_string();

    // Define transaction review fields
    let my_fields = [
        Field {
            name: "Amount",
            value: value_str.as_str(),
        },
        Field {
            name: "Destination",
            value: to_str.as_str(),
        },
        Field {
            name: "Memo",
            value: tx.memo,
        },
        Field {
            name: "Path",
            value: path_str.as_str(),
        },
    ];
    let show_path = Settings.get_element(SETTING_SHOW_PATH) != 0;
    let displayed_fields = if show_path {
        &my_fields[..]
    } else {
        &my_fields[..my_fields.len() - 1]
    };

    // Create transaction review

    // Load glyph from file with include_gif macro. Creates an NBGL compatible glyph.
    #[cfg(target_os = "apex_p")]
    const FERRIS: NbglGlyph = NbglGlyph::from_include(include_gif!("glyphs/crab_48x48.png", NBGL));
    #[cfg(any(target_os = "stax", target_os = "flex"))]
    const FERRIS: NbglGlyph = NbglGlyph::from_include(include_gif!("glyphs/crab_64x64.gif", NBGL));
    #[cfg(any(target_os = "nanosplus", target_os = "nanox"))]
    const FERRIS: NbglGlyph = NbglGlyph::from_include(include_gif!("icons/crab_14x14.gif", NBGL));

    // Create NBGL review. Maximum number of fields and string buffer length can be customised
    // with constant generic parameters of NbglReview. Default values are 32 and 1024 respectively.
    let review: NbglReview = NbglReview::new()
        .titles(
            "Review transaction\nto send CRAB",
            "",
            "Sign transaction\nto send CRAB",
        )
        .glyph(&FERRIS);

    Ok(review.show(comm, displayed_fields))
}
