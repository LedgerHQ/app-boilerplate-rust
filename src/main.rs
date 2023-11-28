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
use ledger_device_sdk::io::{ApduHeader, Comm, Event, Reply, StatusWords};

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
    BadLen = StatusWords::BadLen as u16,
}

impl From<AppSW> for Reply {
    fn from(sw: AppSW) -> Reply {
        Reply(sw as u16)
    }
}

/// Possible input commands received through APDUs.
enum Instruction {
    GetVersion,
    GetAppName,
    GetPubkey { display: bool },
    SignTx { chunk: u8, more: bool },
}

/// APDU parsing logic.
///
/// Parses CLA, INS, P1 and P2 bytes to build an [`Ins`]. P1 and P2 are translated to strongly
/// typed variables depending on the APDU instruction code. Invalid INS, P1 or P2 values result in
/// errors with a status word, which are automatically sent to the host by the SDK.
///
/// This design allows a clear separation of the APDU parsing logic and commands handling.
impl TryFrom<ApduHeader> for Instruction {
    type Error = AppSW;

    fn try_from(value: ApduHeader) -> Result<Self, Self::Error> {
        if value.cla != CLA {
            return Err(AppSW::ClaNotSupported);
        }
        match (value.ins, value.p1, value.p2) {
            (3, 0, 0) => Ok(Instruction::GetVersion),
            (4, 0, 0) => Ok(Instruction::GetAppName),
            (5, 0 | 1, 0) => Ok(Instruction::GetPubkey {
                display: value.p1 != 0,
            }),
            (6, P1_SIGN_TX_START, P2_SIGN_TX_MORE)
            | (6, 1..=3, P2_SIGN_TX_LAST | P2_SIGN_TX_MORE) => Ok(Instruction::SignTx {
                chunk: value.p1,
                more: value.p2 == P2_SIGN_TX_MORE,
            }),
            (3..=6, _, _) => Err(AppSW::WrongP1P2),
            (_, _, _) => Err(AppSW::InsNotSupported),
        }
    }
}

// Developer mode / pending review popup
// must be cleared with user interaction
fn display_pending_review(comm: &mut Comm) {
    use ledger_device_sdk::buttons::ButtonEvent::{
        BothButtonsRelease, LeftButtonRelease, RightButtonRelease,
    };
    use ledger_device_ui_sdk::layout::{Layout, Location, StringPlace};
    use ledger_device_ui_sdk::screen_util::screen_update;
    use ledger_device_ui_sdk::ui::clear_screen;

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
            match handle_apdu(&mut comm, ins, &mut tx_ctx) {
                Ok(()) => comm.reply_ok(),
                Err(sw) => comm.reply(Reply::from(sw)),
            }
        }
    }
}

fn handle_apdu(comm: &mut Comm, ins: Instruction, ctx: &mut TxContext) -> Result<(), AppSW> {
    match ins {
        Instruction::GetAppName => {
            comm.append(env!("CARGO_PKG_NAME").as_bytes());
            Ok(())
        }
        Instruction::GetVersion => handler_get_version(comm),
        Instruction::GetPubkey { display } => handler_get_public_key(comm, display),
        Instruction::SignTx { chunk, more } => handler_sign_tx(comm, chunk, more, ctx)
    }
}
