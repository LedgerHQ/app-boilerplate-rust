/*****************************************************************************
 *   Ledger App Boilerplate Rust.
 *   (c) 2026 Ledger SAS.
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

//! Token descriptors signed by Ledger, sent with PROVIDE_TOKEN_INFO.
//!
//! A descriptor gives the ticker and decimals of a token address. The SDK checks its TLV format
//! and its signature, which must verify with the PKI certificate the host loaded beforehand, of
//! usage COIN_META. This module adds the checks that are specific to this app: the descriptor
//! targets this app and its coin, its ticker and decimals can be displayed, and its TUID holds a
//! token address.
//!
//! The SDK requires the structure type tag but does not give its value, so the app cannot check
//! that it is the dynamic token type (`0x90`). The descriptor is still bound to the CAL by its
//! signature, which only a key with a COIN_META certificate can make.
//!
//! Parsing allocates, as the SDK output holds a `Vec` and `String`s, so it must not run while the
//! Exchange app shares BSS memory. The parsed token is stored without heap.

use ledger_device_sdk::log;
use ledger_device_sdk::tag_to_flag_u64;
use ledger_device_sdk::tlv::tlv_dynamic_token::{parse_dynamic_token_tlv, DynamicTokenOut};
use ledger_device_sdk::tlv::tlv_generic::{
    parse, Handler, ParseCfg, Received, Result as TlvResult, Tag, TlvData, TlvError,
};

use super::{TokenInfo, TOKEN_ADDRESS_LEN};
use crate::utils::{DESCRIPTOR_APP_NAME, HARDENED, SLIP44_COIN_TYPE};

/// Version of the descriptor format this app supports.
const DESCRIPTOR_VERSION: u8 = 1;

/// Longest coin type encoding in a descriptor, a big-endian `u32`.
const MAX_COIN_TYPE_LEN: usize = core::mem::size_of::<u32>();

/// Token described by a signed descriptor.
pub struct DynamicToken {
    pub(super) address: [u8; TOKEN_ADDRESS_LEN],
    pub(super) info: TokenInfo,
}

/// Reason a descriptor is rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DescriptorError {
    /// Malformed TLV, missing or repeated tag, or signature that the loaded certificate does not
    /// verify.
    Tlv,
    /// Descriptor format version other than [`DESCRIPTOR_VERSION`].
    Version,
    /// Coin type other than the SLIP-0044 coin type of this app.
    CoinType,
    /// Descriptor for another app.
    AppName,
    /// Ticker or decimals that [`TokenInfo`] does not accept.
    TokenInfo,
    /// TUID that does not hold exactly one 32-byte token address.
    Tuid,
    /// Ticker of the native coin or of another token of the hardcoded list.
    TickerTaken,
}

/// Parses and checks a signed token descriptor.
///
/// The checks run in this order:
/// 1. TLV format and signature, by the SDK.
/// 2. Version.
/// 3. Coin type.
/// 4. App name.
/// 5. Ticker and decimals.
/// 6. TUID.
/// 7. Ticker not naming another asset.
///
/// The SDK reads the version and the decimals from 1, 2, 4 or 8 bytes and keeps the low byte only,
/// so a multi-byte value whose low byte passes steps 2 and 5 is accepted, e.g. `0x0101` as version 1.
/// The app cannot detect it, as the SDK gives no raw value; such a descriptor needs a Ledger
/// signature all the same.
pub fn parse_descriptor(payload: &[u8]) -> Result<DynamicToken, DescriptorError> {
    let mut out = DynamicTokenOut::default();
    parse_dynamic_token_tlv(payload, &mut out).map_err(|error| {
        log::error!("Descriptor TLV rejected: {:?}", error);
        DescriptorError::Tlv
    })?;

    if out.version != DESCRIPTOR_VERSION {
        log::error!("Descriptor version {} not supported", out.version);
        return Err(DescriptorError::Version);
    }
    if !is_app_coin_type(&out.coin_type) {
        log::error!(
            "Descriptor coin type {:x?} is not the app's",
            out.coin_type.as_slice()
        );
        return Err(DescriptorError::CoinType);
    }
    if out.app_name != DESCRIPTOR_APP_NAME {
        log::error!("Descriptor for app {}", out.app_name.as_str());
        return Err(DescriptorError::AppName);
    }
    let Some(info) = TokenInfo::new(&out.ticker, out.magnitude) else {
        log::error!("Descriptor ticker or decimals ({}) rejected", out.magnitude);
        return Err(DescriptorError::TokenInfo);
    };
    let address = parse_tuid(&out.tuid).map_err(|error| {
        log::error!("Descriptor TUID rejected: {:?}", error);
        DescriptorError::Tuid
    })?;
    if super::ticker_names_other_asset(info.ticker(), &address) {
        log::error!("Descriptor ticker {} names another asset", info.ticker());
        return Err(DescriptorError::TickerTaken);
    }

    log::debug!(
        "Descriptor accepted: {} with {} decimals",
        info.ticker(),
        info.decimals()
    );
    Ok(DynamicToken { address, info })
}

/// Whether a descriptor coin type, 1 to 4 big-endian bytes, is the SLIP-0044 coin type of this
/// app, hardened or not.
fn is_app_coin_type(coin_type: &[u8]) -> bool {
    if coin_type.is_empty() || coin_type.len() > MAX_COIN_TYPE_LEN {
        return false;
    }
    let value = coin_type
        .iter()
        .fold(0u32, |value, &byte| (value << 8) | u32::from(byte));
    value == SLIP44_COIN_TYPE || value == SLIP44_COIN_TYPE | HARDENED
}

// The TUID (Token Unique IDentifier) is a TLV whose content each app defines. This app identifies
// a token by its address only, so its TUID holds a single tag. An app whose tokens need more
// fields (a token standard, a module name...) adds a tag and a handler for each.

/// TUID tag of the token address.
const TAG_TOKEN_ADDRESS: Tag = 0x10;

tag_to_flag_u64!(TAG_TOKEN_ADDRESS);

/// Content of the TUID.
#[derive(Default)]
struct Tuid {
    address: [u8; TOKEN_ADDRESS_LEN],
}

fn on_token_address(data: &TlvData<'_>, out: &mut Tuid) -> TlvResult<bool> {
    let address = data.as_bounded(TOKEN_ADDRESS_LEN, TOKEN_ADDRESS_LEN)?;
    out.address.copy_from_slice(address);
    Ok(true)
}

/// TUID tags. The SDK rejects any tag missing from this table with [`TlvError::UnknownTag`].
static TUID_HANDLERS: &[Handler<Tuid>] = &[Handler {
    tag: TAG_TOKEN_ADDRESS,
    unique: true,
    func: Some(on_token_address),
}];

/// Reads the token address from the TUID, which must hold tag `0x10` with exactly 32 bytes, and
/// no other tag.
fn parse_tuid(tuid: &[u8]) -> TlvResult<[u8; TOKEN_ADDRESS_LEN]> {
    let mut out = Tuid::default();
    let mut received = Received::new(tag_to_flag_u64);
    parse(&ParseCfg::new(TUID_HANDLERS), tuid, &mut out, &mut received)?;

    let mandatory_tags = tag_to_flag_u64(TAG_TOKEN_ADDRESS);
    if received.flags & mandatory_tags != mandatory_tags {
        return Err(TlvError::MissingMandatoryTag);
    }
    Ok(out.address)
}
