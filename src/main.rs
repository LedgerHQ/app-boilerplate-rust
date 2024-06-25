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
/*mod app_ui {
    pub mod address;
    pub mod menu;
    pub mod sign;
}
mod handlers {
    pub mod get_public_key;
    pub mod get_version;
    pub mod sign_tx;
}

mod settings;*/

//use app_ui::menu::ui_menu_main;
/*use handlers::{
    get_public_key::handler_get_public_key,
    get_version::handler_get_version,
    sign_tx::{handler_sign_tx, TxContext},
//};*/
#[cfg(feature = "pending_review_screen")]
#[cfg(not(any(target_os = "stax", target_os = "flex")))]
use ledger_device_sdk::ui::gadgets::display_pending_review;
use ledger_device_sdk::{
    io::{self, ApduHeader, Comm, Event, Reply, StatusWords},
    nbgl_layout,
};

ledger_device_sdk::set_panic!(ledger_device_sdk::exiting_panic);

// Required for using String, Vec, format!...
extern crate alloc;

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
    WrongApduLength = StatusWords::BadLen as u16,
}

impl From<AppSW> for Reply {
    fn from(sw: AppSW) -> Reply {
        Reply(sw as u16)
    }
}

use include_gif::include_gif;
use ledger_device_sdk::nbgl_layout::NbglGlyph;

enum Instruction {
    Select,
    ReadBinary,
}

impl TryFrom<ApduHeader> for Instruction {
    type Error = StatusWords;
    fn try_from(h: ApduHeader) -> Result<Self, Self::Error> {
        match h.ins {
            0xa4 => Ok(Self::Select),
            0xb0 => Ok(Self::ReadBinary),
            _ => Err(StatusWords::BadIns),
        }
    }
}

#[no_mangle]
extern "C" fn sample_main() {
    // Create the communication manager, and configure it to accept only APDU from the 0xe0 class.
    // If any APDU with a wrong class value is received, comm will respond automatically with
    // BadCla status word.
    let mut comm = Comm::new().set_expected_cla(0xe0);

    const FERRIS: NbglGlyph = NbglGlyph::from_include(include_gif!("crab_64x64.gif", NBGL));

    nbgl_layout::display(&FERRIS);

    loop {
        // Wait for either a specific button push to exit the app
        // or an APDU command
        match comm.next_event() {
            Event::Command(Instruction::Select) => {
                ledger_device_sdk::testing::debug_print("Command received\n");
            }
            Event::Command(Instruction::ReadBinary) => {
                ledger_device_sdk::testing::debug_print("Command received\n");
            }

            Event::TouchEvent => {
                ledger_device_sdk::testing::debug_print("Touch event received\n");
            }
            _ => (),
        }
    }
}

/*fn handle_apdu(comm: &mut Comm, ins: Instruction, ctx: &mut TxContext) -> Result<(), AppSW> {
    match ins {
        Instruction::GetAppName => {
            comm.append(env!("CARGO_PKG_NAME").as_bytes());
            Ok(())
        }
        Instruction::GetVersion => handler_get_version(comm),
        Instruction::GetPubkey { display } => handler_get_public_key(comm, display),
        Instruction::SignTx { chunk, more } => handler_sign_tx(comm, chunk, more, ctx),
    }
}*/
