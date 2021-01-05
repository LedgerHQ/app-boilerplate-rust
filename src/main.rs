#![no_std]
#![no_main]

mod crypto_helpers;
mod utils;

use nanos_sdk::buttons::ButtonEvent;
use nanos_sdk::io;
use nanos_sdk::ecc::{CurvesId, DEREncodedECDSASignature};
use nanos_ui::ui;
use core::str::from_utf8;
use crypto_helpers::*;

nanos_sdk::set_panic!(nanos_sdk::exiting_panic);

/// Display public key in two separate
/// message scrollers
fn show_pubkey() {
    let pubkey = get_pubkey();
    {
        let hex0 = utils::to_hex(&pubkey.W[1..33]).unwrap();
        let m = from_utf8(&hex0).unwrap();
        ui::MessageScroller::new(&m).event_loop();
    }
    {
        let hex1 = utils::to_hex(&pubkey.W[33..65]).unwrap();
        let m = from_utf8(&hex1).unwrap();
        ui::MessageScroller::new(&m).event_loop();
    }
}

/// Basic nested menu. Will be subject
/// to simplifications in the future.
fn menu_example() {
    loop {
        match ui::Menu::new(&[&"PubKey", &"Infos", &"Back", &"Exit App"]).show() {
            0 => show_pubkey(),
            1 => loop {
                match ui::Menu::new(&[&"Copyright", &"Authors", &"Back"]).show() {
                    0 => ui::popup("2020 Ledger"),
                    1 => ui::popup("???"),
                    _ => break 
                }
            }
            2 => return,
            3 => nanos_sdk::exit_app(0),
            _ => () 
        }
    } 
}

/// This is the UI flow for signing, composed of a scroller
/// to read the incoming message, a panel that requests user
/// validation, and an exit message.
fn sign_ui(message: &[u8]) -> Result<Option<DEREncodedECDSASignature>, ()> {
    ui::popup("Message review");

    {
        let hex = utils::to_hex(&message)?;
        let m = from_utf8(&hex).map_err(|_| ())?;

        ui::MessageScroller::new(&m).event_loop();
    }

    if ui::Validator::new("Sign ?").ask() {
        let mut k = get_private_key();
        let (sig, sig_len) = detecdsa_sign(&message, &k).unwrap();

        // Signature verification so we're sure the bindings are OK !
        let pubkey = nanos_sdk::ecc::ec_get_pubkey(CurvesId::Secp256k1, &mut k);
        if !detecdsa_verify(&message, &sig[..sig_len as usize], &pubkey) {
            ui::popup("Invalid :(");
            return Err(())
        }

        ui::popup("Done !");
        Ok(Some(sig))
    } else {
        ui::popup("Cancelled");
        Ok(None)
    }
}

#[no_mangle]
extern "C" fn sample_main() {
    let mut comm = io::Comm::new();

    loop {
        // Draw some 'welcome' screen
        ui::SingleMessage::new("W e l c o m e").show();

        // Wait for either a specific button push to exit the app
        // or an APDU command
        match comm.next_event() {
            io::Event::Button(ButtonEvent::RightButtonRelease) => nanos_sdk::exit_app(0),
            io::Event::Command(ins) => {
                match handle_apdu(&mut comm, ins) {
                    Ok(()) => comm.reply_ok(),
                    Err(sw) => comm.reply(sw)
                }
            }
            _ => ()
        }
    }
}

#[repr(u8)]
enum Ins {
    GetPubkey,
    Sign,
    Menu,
    SingleMessage,
    DoubleMessage,
    ShowPrivateKey,
    Exit,
}

impl From<u8> for Ins {
    fn from(ins: u8) -> Ins {
        match ins {
            2 => Ins::GetPubkey,
            3 => Ins::Sign,
            4 => Ins::Menu,
            0x10 => Ins::SingleMessage,
            0x20 => Ins::DoubleMessage,
            0xfe => Ins::ShowPrivateKey,
            0xff => Ins::Exit,
            _ => panic!()
        }
    }
}


fn handle_apdu(comm: &mut io::Comm, ins: Ins) -> Result<(), io::StatusWords> {
    if comm.rx == 0 {
        return Err(io::StatusWords::NothingReceived)
    }

    match ins {
        Ins::GetPubkey => comm.append(&get_pubkey().W), 
        Ins::Sign => {
            let out = sign_ui(comm.get_data()?)
                            .map_err(|_| io::StatusWords::UserCancelled)?;
            if let Some(o) = out { comm.append(&o) }
        }
        Ins::Menu => menu_example(),
        Ins::ShowPrivateKey => comm.append(&bip32_derive_secp256k1(&BIP32_PATH)),
        Ins::SingleMessage => comm.append(&[0xb8; 32]),
        Ins::DoubleMessage => comm.append(&[0xf7; 64]),
        Ins::Exit => nanos_sdk::exit_app(0),
    }
    Ok(())
}
