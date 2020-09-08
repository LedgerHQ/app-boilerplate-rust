#![no_std]
#![no_main]

mod crypto_helpers;
mod utils;

use nanos_sdk::io;
use nanos_sdk::ecc::{CurvesId, DEREncodedECDSASignature};
use nanos_ui::ui;
use core::str::from_utf8;
use crypto_helpers::*;

nanos_sdk::set_panic!(nanos_sdk::exiting_panic);

fn handle_apdus(comm: &mut io::Comm) -> Result<(), io::StatusWords> {
    if comm.rx == 0 {
        return Err(io::StatusWords::NothingReceived)
    }

    let (cla, ins) = comm.get_cla_ins();

    if cla != 0x80 {
        return Err(io::StatusWords::BadCLA)
    }

    match ins {
        0x02 => comm.append(&get_pubkey().W), 
        0x03 => {
            let out = sign_ui(comm.get_data()?)
                            .map_err(|_| io::StatusWords::UserCancelled)?;
            if let Some(o) = out { comm.append(&o) }
        }
        0x04 => comm.append(&[menu_ui()]),
        0xfe => comm.append(&bip32_derive_secp256k1(&BIP32_PATH)),
        0xff => nanos_sdk::exit_app(0),
        _ => return Err(io::StatusWords::Unknown),
    }
    Ok(())
}

fn menu_ui() -> u8 {
    let list = [
                "First",
                "Second",
                "Third",
                "Fourth",
                "Fifth",
                "Sixth",
                "Seventh"
    ];
    ui::Menu::new(&list).show() as u8
}

/// This is the UI flow for signing, composed of a scroller
/// to read the incoming message, a panel that requests user
/// validation, and an exit message.
fn sign_ui(message: &[u8]) -> Result<Option<DEREncodedECDSASignature>, ()> {
    let hex = utils::to_hex(&message)?;
    let m = from_utf8(&hex).map_err(|_| ())?;

    ui::SingleMessage::new("Message review").show_and_wait();
    ui::MessageScroller::new(&m).event_loop();

    match ui::Validator::new("Sign ?").ask() {
        true => {
            let mut k = get_private_key();
            let (sig, sig_len) = detecdsa_sign(&message, &k).unwrap();

            // Signature verification so we're sure the bindings are OK !
            let pubkey = nanos_sdk::ecc::ec_get_pubkey(CurvesId::Secp256k1, &mut k);
            if !detecdsa_verify(&message, &sig[..sig_len as usize], &pubkey) {
                ui::SingleMessage::new("Invalid :(").show_and_wait();
                return Err(())
            }

            ui::SingleMessage::new("Done !").show_and_wait();
            Ok(Some(sig))
        },
        false => {
            ui::SingleMessage::new("Cancelled").show_and_wait();
            Ok(None)
        }
    }
}

#[no_mangle]
extern "C" fn sample_main() {
    let mut comm = io::Comm::new();

    loop {
        ui::SingleMessage::new("W e l c o m e").show();

        comm.io_exch(0);

        match handle_apdus(&mut comm) {
            Ok(()) => comm.set_status_word(io::StatusWords::OK),
            Err(sw) => comm.set_status_word(sw),
        }
    }
}