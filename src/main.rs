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

mod settings;

use app_ui::menu::ui_menu_main;
use handlers::{
    get_public_key::handler_get_public_key,
    get_version::handler_get_version,
    sign_tx::{handler_sign_tx, TxContext},
};
use ledger_device_sdk::io::{self, ApduHeader, Comm, Command, Reply, StatusWords};

ledger_device_sdk::set_panic!(ledger_device_sdk::exiting_panic);

// Required for using String, Vec, format!...
extern crate alloc;

use ledger_device_sdk::nbgl::{NbglReviewStatus, StatusType};

// P2 for last APDU to receive.
const P2_SIGN_TX_LAST: u8 = 0x00;
// P2 for more APDU to receive.
const P2_SIGN_TX_MORE: u8 = 0x80;
// P1 for first APDU number.
const P1_SIGN_TX_START: u8 = 0x00;
// P1 for maximum APDU number.
const P1_SIGN_TX_MAX: u8 = 0x03;

// Application status words.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AppSW {
    Deny,
    WrongP1P2,
    InsNotSupported,
    ClaNotSupported,
    CommError,
    TxDisplayFail,
    AddrDisplayFail,
    TxWrongLength,
    TxParsingFail,
    TxHashFail,
    TxSignFail,
    KeyDeriveFail,
    VersionParsingFail,
    WrongApduLength,
    Ok,
    Unknown(u16),
}

impl From<AppSW> for Reply {
    fn from(sw: AppSW) -> Reply {
        let code: u16 = match sw {
            AppSW::Deny => 0x6985,
            AppSW::WrongP1P2 => 0x6A86,
            AppSW::InsNotSupported => 0x6D00,
            AppSW::ClaNotSupported => 0x6E00,
            AppSW::CommError => 0x6F00,
            AppSW::TxDisplayFail => 0xB001,
            AppSW::AddrDisplayFail => 0xB002,
            AppSW::TxWrongLength => 0xB004,
            AppSW::TxParsingFail => 0xB005,
            AppSW::TxHashFail => 0xB006,
            AppSW::TxSignFail => 0xB008,
            AppSW::KeyDeriveFail => 0xB009,
            AppSW::VersionParsingFail => 0xB00A,
            AppSW::WrongApduLength => StatusWords::BadLen as u16,
            AppSW::Ok => 0x9000,
            AppSW::Unknown(x) => x,
        };
        Reply(code)
    }
}

impl From<Reply> for AppSW {
    fn from(value: Reply) -> Self {
        match value.0 {
            0x6985 => AppSW::Deny,
            0x6A86 => AppSW::WrongP1P2,
            0x6D00 => AppSW::InsNotSupported,
            0x6E00 => AppSW::ClaNotSupported,
            0x6F00 => AppSW::CommError,
            0xB001 => AppSW::TxDisplayFail,
            0xB002 => AppSW::AddrDisplayFail,
            0xB004 => AppSW::TxWrongLength,
            0xB005 => AppSW::TxParsingFail,
            0xB006 => AppSW::TxHashFail,
            0xB008 => AppSW::TxSignFail,
            0xB009 => AppSW::KeyDeriveFail,
            0xB00A => AppSW::VersionParsingFail,
            x if x == StatusWords::BadLen as u16 => AppSW::WrongApduLength,
            0x9000 => AppSW::Ok,
            other => AppSW::Unknown(other),
        }
    }
}

impl From<io::CommError> for AppSW {
    fn from(_e: io::CommError) -> Self {
        AppSW::CommError
    }
}

/// Possible input commands received through APDUs.
#[derive(Debug)]
pub enum Instruction {
    GetVersion,
    GetAppName,
    GetPubkey { display: bool },
    SignTx { chunk: u8, more: bool },
}

impl TryFrom<ApduHeader> for Instruction {
    type Error = AppSW;

    /// APDU parsing logic.
    ///
    /// Parses INS, P1 and P2 bytes to build an [`Instruction`]. P1 and P2 are translated to
    /// strongly typed variables depending on the APDU instruction code. Invalid INS, P1 or P2
    /// values result in errors with a status word, which are automatically sent to the host by the
    /// SDK.
    ///
    /// This design allows a clear separation of the APDU parsing logic and commands handling.
    ///
    /// Note that CLA is not checked here. Instead the method [`Comm::set_expected_cla`] is used in
    /// [`sample_main`] to have this verification automatically performed by the SDK.
    fn try_from(value: ApduHeader) -> Result<Self, Self::Error> {
        match (value.ins, value.p1, value.p2) {
            (3, 0, 0) => Ok(Instruction::GetVersion),
            (4, 0, 0) => Ok(Instruction::GetAppName),
            (5, 0 | 1, 0) => Ok(Instruction::GetPubkey {
                display: value.p1 != 0,
            }),
            (6, P1_SIGN_TX_START, P2_SIGN_TX_MORE)
            | (6, 1..=P1_SIGN_TX_MAX, P2_SIGN_TX_LAST | P2_SIGN_TX_MORE) => {
                Ok(Instruction::SignTx {
                    chunk: value.p1,
                    more: value.p2 == P2_SIGN_TX_MORE,
                })
            }
            (3..=6, _, _) => Err(AppSW::WrongP1P2),
            (_, _, _) => Err(AppSW::InsNotSupported),
        }
    }
}

fn show_status_and_home_if_needed(ins: &Instruction, tx_ctx: &mut TxContext, status: &AppSW) {
    let (show_status, status_type) = match (ins, status) {
        (Instruction::GetPubkey { display: true }, AppSW::Deny | AppSW::Ok) => {
            (true, StatusType::Address)
        }
        (Instruction::SignTx { .. }, AppSW::Deny | AppSW::Ok) if tx_ctx.finished() => {
            (true, StatusType::Transaction)
        }
        (_, _) => (false, StatusType::Transaction),
    };

    if show_status {
        let success = *status == AppSW::Ok;
        NbglReviewStatus::new()
            .status_type(status_type)
            .show(success);

        // call home.show_and_return() to show home and setting screen
        tx_ctx.home.show_and_return();
    }
}

#[no_mangle]
extern "C" fn sample_main() {
    // Create the communication manager, and configure it to accept only APDU from the 0xe0 class.
    // If any APDU with a wrong class value is received, comm will respond automatically with
    // BadCla status word.
    let mut comm = Comm::new().set_expected_cla(0xe0);

    let mut tx_ctx = TxContext::new();

    tx_ctx.home = ui_menu_main(&mut comm);
    tx_ctx.home.show_and_return();

    loop {
        let command = comm.next_command();
        let decoded = command.decode::<Instruction>().map_err(AppSW::from);
        let Ok(ins) = decoded else {
            let _ = comm.send(&[], decoded.unwrap_err());
            continue;
        };

        let _status = match handle_apdu(command, &ins, &mut tx_ctx) {
            Ok(reply) => {
                let _ = reply.send(AppSW::Ok);
                AppSW::Ok
            }
            Err(sw) => {
                let _ = comm.send(&[], sw);
                sw
            }
        };
        show_status_and_home_if_needed(&ins, &mut tx_ctx, &_status);
    }
}

fn handle_apdu<'a>(
    command: Command<'a>,
    ins: &'a Instruction,
    ctx: &mut TxContext,
) -> Result<io::CommandResponse<'a>, AppSW> {
    match ins {
        Instruction::GetAppName => {
            let mut response = command.into_response();
            response.append(env!("CARGO_PKG_NAME").as_bytes())?;
            Ok(response)
        }
        Instruction::GetVersion => handler_get_version(command),
        Instruction::GetPubkey { display } => handler_get_public_key(command, *display),
        Instruction::SignTx { chunk, more } => handler_sign_tx(command, *chunk, *more, ctx),
    }
}
