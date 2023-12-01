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

use app_ui::menu::ui_menu_main;
use handlers::{
    get_public_key::handler_get_public_key,
    get_version::handler_get_version,
    sign_tx::{handler_sign_tx, TxContext},
};
use ledger_device_sdk::io::{ApduHeader, Comm, Event, Reply};

ledger_device_sdk::set_panic!(ledger_device_sdk::exiting_panic);

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

// Application status words.
#[repr(u16)]
pub enum AppSW {
    Deny = 0x6985,
    WrongP1P2 = 0x6A86,
    WrongDataLength = 0x6A87,
    InsNotSupported = 0x6D00,
    ClaNotSupported = 0x6E00,
    TxDisplayFail = 0xB001,
    AddrDisplayFail = 0xB002,
    TxWrongLength = 0xB004,
    TxParsingFail = 0xB005,
    TxHashFail = 0xB006,
    TxSignFail = 0xB008,
    KeyDeriveFail = 0xB009,
    VersionParsingFail = 0xB00A,
}

impl From<AppSW> for Reply {
    fn from(sw: AppSW) -> Reply {
        Reply(sw as u16)
    }
}

#[repr(u8)]
// Instruction set for the app.
enum Ins {
    GetVersion,
    GetAppName,
    GetPubkey,
    SignTx,
    Unknown,
}

impl From<ApduHeader> for Ins {
    fn from(header: ApduHeader) -> Ins {
        match header.ins {
            3 => Ins::GetVersion,
            4 => Ins::GetAppName,
            5 => Ins::GetPubkey,
            6 => Ins::SignTx,
            _ => Ins::Unknown,
        }
    }
}

// Developer mode / pending review popup
// must be cleared with user interaction
fn display_pending_review(comm: &mut Comm) {
    use ledger_device_sdk::buttons::ButtonEvent::{
        BothButtonsRelease, LeftButtonRelease, RightButtonRelease,
    };
    use ledger_device_sdk::ui::gadgets::clear_screen;
    use ledger_device_sdk::ui::layout::{Layout, Location, StringPlace};
    use ledger_device_sdk::ui::screen_util::screen_update;

    clear_screen();
    "Pending Review".place(Location::Middle, Layout::Centered, false);
    screen_update();

    loop {
        if let Event::Button(LeftButtonRelease | RightButtonRelease | BothButtonsRelease) =
            comm.next_event::<ApduHeader>()
        {
            break;
        }
    }
}

#[no_mangle]
extern "C" fn sample_main() {
    let mut comm = Comm::new();

    display_pending_review(&mut comm);

    let mut tx_ctx = TxContext::new();

    loop {
        // Wait for either a specific button push to exit the app
        // or an APDU command
        if let Event::Command(ins) = ui_menu_main(&mut comm) {
            match handle_apdu(&mut comm, ins.into(), &mut tx_ctx) {
                Ok(()) => comm.reply_ok(),
                Err(sw) => comm.reply(Reply::from(sw)),
            }
        }
    }
}

fn handle_apdu(comm: &mut Comm, ins: Ins, ctx: &mut TxContext) -> Result<(), AppSW> {
    // Reject any APDU that does not have a minimum length of 4 bytes
    // The APDU must have at least 4 bytes: CLA, INS, P1, P2
    if comm.rx < 4 {
        return Err(AppSW::WrongDataLength);
    }

    let data = comm.get_data().map_err(|_| AppSW::WrongDataLength)?;

    let apdu_metadata = comm.get_apdu_metadata();

    if apdu_metadata.cla != CLA {
        return Err(AppSW::ClaNotSupported);
    }

    match ins {
        Ins::GetAppName => {
            if apdu_metadata.p1 != 0 || apdu_metadata.p2 != 0 {
                return Err(AppSW::WrongP1P2);
            }
            comm.append(env!("CARGO_PKG_NAME").as_bytes());
        }
        Ins::GetVersion => {
            if apdu_metadata.p1 != 0 || apdu_metadata.p2 != 0 {
                return Err(AppSW::WrongP1P2);
            }
            return handler_get_version(comm);
        }
        Ins::GetPubkey => {
            if apdu_metadata.p1 > 1 || apdu_metadata.p2 != 0 {
                return Err(AppSW::WrongP1P2);
            }
            if data.is_empty() {
                return Err(AppSW::WrongDataLength);
            }

            return handler_get_public_key(comm, apdu_metadata.p1 == 1);
        }
        Ins::SignTx => {
            if (apdu_metadata.p1 == P1_SIGN_TX_START && apdu_metadata.p2 != P2_SIGN_TX_MORE)
                || apdu_metadata.p1 > P1_SIGN_TX_MAX
                || (apdu_metadata.p2 != P2_SIGN_TX_LAST && apdu_metadata.p2 != P2_SIGN_TX_MORE)
            {
                return Err(AppSW::WrongP1P2);
            }
            if data.is_empty() {
                return Err(AppSW::WrongDataLength);
            }

            return handler_sign_tx(
                comm,
                apdu_metadata.p1,
                apdu_metadata.p2 == P2_SIGN_TX_MORE,
                ctx,
            );
        }
        Ins::Unknown => {
            return Err(AppSW::InsNotSupported);
        }
    }
    Ok(())
}
