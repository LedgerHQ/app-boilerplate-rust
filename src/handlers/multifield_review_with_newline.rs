use crate::AppSW;
use ledger_device_sdk::{
    io,
    ui::{
        bitmaps::{CROSSMARK, EYE, VALIDATE_14},
        gadgets::{Field, MultiFieldReview},
    },
};

pub fn handler_multifield_newline(_comm: &mut io::Comm) -> Result<(), AppSW> {
    let my_field = [Field {
        name: "Field title",
        value: "value\nhidden part of value 1 2 3 4 5 6 7 8 9",
    }];

    let my_review = MultiFieldReview::new(
        &my_field,
        &["Example with newline"],
        Some(&EYE),
        "Approve",
        Some(&VALIDATE_14),
        "Reject",
        Some(&CROSSMARK),
    );
    let _ = my_review.show();
    Ok(())
}
