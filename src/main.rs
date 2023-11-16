#![no_std]
#![no_main]

mod utils;
mod app_ui {
    pub mod address;
    pub mod menu;
    pub mod sign;
}
mod handlers {
    pub mod get_public_key;
    pub mod get_version;
    pub mod sign_tx;
}

use ledger_device_sdk::buttons::ButtonEvent;
use ledger_device_sdk::io::{ApduHeader, Comm, Event, Reply, StatusWords};

use ledger_device_ui_sdk::ui;

use app_ui::menu::ui_menu_main;
use handlers::{
    get_public_key::handler_get_public_key,
    get_version::handler_get_version,
    sign_tx::{handler_sign_tx, TxContext},
};

ledger_device_sdk::set_panic!(ledger_device_sdk::exiting_panic);

pub const SW_INS_NOT_SUPPORTED: u16 = 0x6D00;
pub const SW_DENY: u16 = 0x6985;
pub const SW_WRONG_P1P2: u16 = 0x6A86;
pub const SW_WRONG_DATA_LENGTH: u16 = 0x6A87;
pub const SW_TX_DISPLAY_FAIL: u16 = 0xB001;
pub const SW_DISPLAY_ADDRESS_FAIL: u16 = 0xB002;
pub const SW_WRONG_TX_LENGTH: u16 = 0xB004;
pub const SW_TX_PARSING_FAIL: u16 = 0xB005;
pub const SW_TX_HASH_FAIL: u16 = 0xB006;
pub const SW_TX_SIGN_FAIL: u16 = 0xB008;

#[no_mangle]
extern "C" fn sample_pending() {
    let mut comm = Comm::new();

    loop {
        ui::SingleMessage::new("Pending").show();
        match comm.next_event::<Ins>() {
            Event::Button(ButtonEvent::RightButtonRelease) => break,
            _ => (),
        }
    }
    loop {
        ui::SingleMessage::new("Ledger review").show();
        match comm.next_event::<Ins>() {
            Event::Button(ButtonEvent::BothButtonsRelease) => break,
            _ => (),
        }
    }
}

#[no_mangle]
extern "C" fn sample_main() {
    let mut comm = Comm::new();
    let mut tx_ctx = TxContext::new();

    loop {
        // Wait for either a specific button push to exit the app
        // or an APDU command
        match ui_menu_main(&mut comm) {
            Event::Command(ins) => match handle_apdu(&mut comm, ins.into(), &mut tx_ctx) {
                Ok(()) => comm.reply_ok(),
                Err(sw) => comm.reply(sw),
            },
            _ => (),
        }
    }
}

#[repr(u8)]

// Instruction set for the app.
enum Ins {
    GetVersion,
    GetAppName,
    GetPubkey,
    SignTx,
    UnknownIns,
}
// CLA (APDU class byte) for all APDUs.
const CLA: u8 = 0xe0;
// P2 for last APDU to receive.
const P2_SIGN_TX_LAST: u8 = 0x00;
// P2 for more APDU to receive.
const P2_SIGN_TX_MORE: u8 = 0x80;
// P1 for first APDU number.
const P1_SIGN_TX_START: u8 = 0x00;
// P1 for maximum APDU number.
const P1_SIGN_TX_MAX: u8 = 0x03;

impl From<ApduHeader> for Ins {
    fn from(header: ApduHeader) -> Ins {
        match header.ins {
            3 => Ins::GetVersion,
            4 => Ins::GetAppName,
            5 => Ins::GetPubkey,
            6 => Ins::SignTx,
            _ => Ins::UnknownIns,
        }
    }
}

fn handle_apdu(comm: &mut Comm, ins: Ins, ctx: &mut TxContext) -> Result<(), Reply> {
    if comm.rx == 0 {
        return Err(StatusWords::NothingReceived.into());
    }

    let apdu_metadata = comm.get_apdu_metadata();

    if apdu_metadata.cla != CLA {
        return Err(StatusWords::BadCla.into());
    }

    match ins {
        Ins::GetAppName => {
            if apdu_metadata.p1 != 0 || apdu_metadata.p2 != 0 {
                return Err(Reply(SW_WRONG_P1P2));
            }
            comm.append(env!("CARGO_PKG_NAME").as_bytes());
        }
        Ins::GetVersion => {
            if apdu_metadata.p1 != 0 || apdu_metadata.p2 != 0 {
                return Err(Reply(SW_WRONG_P1P2));
            }
            return handler_get_version(comm);
        }
        Ins::GetPubkey => {
            if apdu_metadata.p1 > 1 || apdu_metadata.p2 != 0 {
                return Err(Reply(SW_WRONG_P1P2));
            }

            if (comm.get_data()?.len()) == 0 {
                return Err(Reply(SW_WRONG_DATA_LENGTH));
            }

            return handler_get_public_key(comm, apdu_metadata.p1 == 1);
        }
        Ins::SignTx => {
            if (apdu_metadata.p1 == P1_SIGN_TX_START && apdu_metadata.p2 != P2_SIGN_TX_MORE)
                || apdu_metadata.p1 > P1_SIGN_TX_MAX
                || (apdu_metadata.p2 != P2_SIGN_TX_LAST && apdu_metadata.p2 != P2_SIGN_TX_MORE)
            {
                return Err(Reply(SW_WRONG_P1P2));
            }

            if (comm.get_data()?.len()) == 0 {
                return Err(Reply(SW_WRONG_DATA_LENGTH));
            }

            return handler_sign_tx(
                comm,
                apdu_metadata.p1,
                apdu_metadata.p2 == P2_SIGN_TX_MORE,
                ctx,
            );
        }
        Ins::UnknownIns => {
            return Err(Reply(SW_INS_NOT_SUPPORTED));
        }
    }
    Ok(())
}
