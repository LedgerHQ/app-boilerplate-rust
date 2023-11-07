#![no_std]
#![no_main]

mod utils;
mod app_ui {
    pub mod address;
    pub mod menu;
}
mod handlers {
    pub mod get_public_key;
    pub mod get_version;
}

use core::str::from_utf8;
use nanos_sdk::buttons::ButtonEvent;
use nanos_sdk::ecc::{Secp256k1, SeedDerive};
use nanos_sdk::io;
use nanos_sdk::io::SyscallError;
use nanos_ui::ui;

use nanos_ui::bitmaps::{CROSSMARK, EYE, VALIDATE_14};

use app_ui::menu::ui_menu_main;
use handlers::{get_public_key::handler_get_public_key, get_version::handler_get_version};

nanos_sdk::set_panic!(nanos_sdk::exiting_panic);

pub const BIP32_PATH: [u32; 5] = nanos_sdk::ecc::make_bip32_path(b"m/44'/535348'/0'/0/0");

pub const SW_INS_NOT_SUPPORTED: u16 = 0x6D00;
pub const SW_DENY: u16 = 0x6985;
pub const SW_WRONG_P1P2: u16 = 0x6A86;
pub const SW_WRONG_DATA_LENGTH: u16 = 0x6A87;

/// This is the UI flow for signing, composed of a scroller
/// to read the incoming message, a panel that requests user
/// validation, and an exit message.
fn sign_ui(message: &[u8]) -> Result<Option<([u8; 72], u32, u32)>, SyscallError> {
    let hex = utils::to_hex(message).map_err(|_| SyscallError::Overflow)?;
    let m = from_utf8(&hex).map_err(|_| SyscallError::InvalidParameter)?;
    let my_field = [ui::Field {
        name: "Data",
        value: m,
    }];

    let my_review = ui::MultiFieldReview::new(
        &my_field,
        &["Review ", "Transaction"],
        Some(&EYE),
        "Approve",
        Some(&VALIDATE_14),
        "Reject",
        Some(&CROSSMARK),
    );

    if my_review.show() {
        let signature = Secp256k1::derive_from_path(&BIP32_PATH)
            .deterministic_sign(message)
            .map_err(|_| SyscallError::Unspecified)?;
        ui::popup("Done !");
        Ok(Some(signature))
    } else {
        ui::popup("Cancelled");
        Ok(None)
    }
}

#[no_mangle]
extern "C" fn sample_pending() {
    let mut comm = io::Comm::new();

    loop {
        ui::SingleMessage::new("Pending").show();
        match comm.next_event::<Ins>() {
            io::Event::Button(ButtonEvent::RightButtonRelease) => break,
            _ => (),
        }
    }
    loop {
        ui::SingleMessage::new("Ledger review").show();
        match comm.next_event::<Ins>() {
            io::Event::Button(ButtonEvent::BothButtonsRelease) => break,
            _ => (),
        }
    }
}

#[no_mangle]
extern "C" fn sample_main() {
    let mut comm = io::Comm::new();

    loop {
        // Wait for either a specific button push to exit the app
        // or an APDU command
        match ui_menu_main(&mut comm) {
            io::Event::Command(ins) => match handle_apdu(&mut comm, ins.into()) {
                Ok(()) => comm.reply_ok(),
                Err(sw) => comm.reply(sw),
            },
            _ => (),
        }
    }
}

#[repr(u8)]

enum Ins {
    GetVersion,
    GetAppName,
    GetPubkey,
    SignTx,
    UnknownIns,
}

const CLA: u8 = 0xe0;

impl From<io::ApduHeader> for Ins {
    fn from(header: io::ApduHeader) -> Ins {
        match header.ins {
            3 => Ins::GetVersion,
            4 => Ins::GetAppName,
            5 => Ins::GetPubkey,
            6 => Ins::SignTx,
            _ => Ins::UnknownIns,
        }
    }
}

use nanos_sdk::io::Reply;

fn handle_apdu(comm: &mut io::Comm, ins: Ins) -> Result<(), Reply> {
    if comm.rx == 0 {
        return Err(io::StatusWords::NothingReceived.into());
    }

    let apdu_metadata = comm.get_apdu_metadata();

    if apdu_metadata.cla != CLA {
        return Err(io::StatusWords::BadCla.into());
    }

    match ins {
        Ins::GetAppName => {
            if apdu_metadata.p1 != 0 || apdu_metadata.p2 != 0 {
                return Err(io::Reply(SW_WRONG_P1P2));
            }
            comm.append(env!("CARGO_PKG_NAME").as_bytes());
        }
        Ins::GetVersion => {
            if apdu_metadata.p1 != 0 || apdu_metadata.p2 != 0 {
                return Err(io::Reply(SW_WRONG_P1P2));
            }
            return handler_get_version(comm);
        }
        Ins::GetPubkey => {
            if apdu_metadata.p1 > 1 || apdu_metadata.p2 != 0 {
                return Err(io::Reply(SW_WRONG_P1P2));
            }

            if (comm.get_data()?.len()) == 0 {
                return Err(io::Reply(SW_WRONG_DATA_LENGTH));
            }

            return handler_get_public_key(comm, apdu_metadata.p1 == 1);
        }
        Ins::SignTx => {
            let out = sign_ui(comm.get_data()?)?;
            if let Some((signature_buf, length, _)) = out {
                comm.append(&signature_buf[..length as usize])
            }
        }
        Ins::UnknownIns => {
            return Err(io::Reply(SW_INS_NOT_SUPPORTED));
        }
    }
    Ok(())
}
