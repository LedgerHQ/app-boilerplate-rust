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

#[cfg(not(any(target_os = "stax", target_os = "flex")))]
use ledger_device_sdk::ui::{
    bitmaps::{CROSSMARK, EYE, VALIDATE_14},
    gadgets::{Field, MultiFieldReview},
};

#[cfg(any(target_os = "stax", target_os = "flex"))]
use crate::settings::Settings;
#[cfg(any(target_os = "stax", target_os = "flex"))]
use include_gif::include_gif;
#[cfg(any(target_os = "stax", target_os = "flex"))]
use ledger_device_sdk::nbgl::{
    convert_content_array, convert_fields, CField, CenteredInfo, CenteredInfoStyle, Field,
    InfoButton, InfoLongPress, NbglGenericReview, NbglGlyph, NbglPageContent, TagValueConfirm,
    TagValueList, TuneIndex,
};

use alloc::ffi::CString;
use alloc::format;

/// Displays a transaction and returns true if user approved it.
///
/// This method can return [`AppSW::TxDisplayFail`] error if the coin name length is too long.
///
/// # Arguments
///
/// * `tx` - Transaction to be displayed for validation
pub fn ui_display_tx(tx: &Tx) -> Result<bool, AppSW> {
    let value_str = format!("{} {}", tx.coin, tx.value);
    let to_str = format!("0x{}", hex::encode(tx.to).to_uppercase());
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
    ];

    // Create transaction review
    #[cfg(not(any(target_os = "stax", target_os = "flex")))]
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

    #[cfg(any(target_os = "stax", target_os = "flex"))]
    {
        // Load glyph from 64x64 4bpp gif file with include_gif macro. Creates an NBGL compatible glyph.
        const FERRIS: NbglGlyph = NbglGlyph::from_include(include_gif!("crab_64x64.gif", NBGL));
        let ferris_icon = (&FERRIS).into();

        let centered_info = CenteredInfo {
            text1: &CString::new("Please ").unwrap(),
            text2: &CString::new("Some stuff").unwrap(),
            text3: &CString::new("Some more stuff").unwrap(),
            icon: Some(&ferris_icon),
            on_top: true,
            style: CenteredInfoStyle::LargeCaseBoldInfo,
            offset_y: 0,
        };

        // let info_button = InfoButton {
        //     text: &CString::new("Validate info : abc").unwrap(),
        //     icon: Some(&ferris_icon),
        //     button_text: &CString::new("Approve").unwrap(),
        //     tune_id: TuneIndex::Success,
        // };

        // let info_long_press = InfoLongPress {
        //     text: &CString::new("Hold to validate transaction").unwrap(),
        //     icon: Some(&ferris_icon),
        //     long_press_text: &CString::new("Hold to validate").unwrap(),
        //     tune_id: TuneIndex::Success,
        // };

        let my_c_fields = [CField {
            name: &CString::new("Hash").unwrap(),
            value: &CString::new("0x6dfb7c0422c534f0b2bada1ac42fbafe").unwrap(),
        }];

        let tag_values_list = TagValueList {
            pairs: &convert_fields(my_c_fields),
            nb_max_lines_for_value: 5,
            small_case_for_value: false,
            wrapping: false,
        };

        let tag_value_confirm = TagValueConfirm {
            tag_value_list: tag_values_list,
            tune_id: TuneIndex::Success,
            confirmation_text: &CString::new("Confirm hash").unwrap(),
            cancel_text: &CString::new("Reject hash").unwrap(),
        };

        let content_array = [
            NbglPageContent::CenteredInfo(centered_info),
            NbglPageContent::TagValueConfirm(tag_value_confirm),
            // NbglPageContent::TagValueList(tag_values_list),
            // NbglPageContent::InfoButton(info_button),
            // NbglPageContent::InfoLongPress(info_long_press),
        ];

        let mut review: NbglGenericReview = NbglGenericReview::new();

        // let mut review: NbglGenericReview = NbglGenericReview::new()
        // .add_content(NbglPageContent::CenteredInfo(centered_info))
        // .add_content(NbglPageContent::TagValueList(tag_values_list));
        // .add_content(NbglPageContent::InfoButton(info_button));
        // .add_content(NbglPageContent::InfoLongPress(info_long_press));
        // .add_content(NbglPageContent::TagValueConfirm(tag_value_confirm));

        Ok(review.show_content_array(
            &(convert_content_array(content_array)),
            &CString::new("Reject Transaction").unwrap(),
        ))
        // Ok(review.show(&CString::new("Reject Transaction").unwrap()))
    }
}
