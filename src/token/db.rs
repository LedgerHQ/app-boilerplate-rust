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

//! Hardcoded token list.
//!
//! The entries are examples: the chain of this app does not exist. A fork replaces them with its
//! own tokens, with no Ledger coordination needed. A swap of a token of the list needs its ticker
//! and decimals to be those of the Ledger CAL, see `doc/TOKENS.md`.

use super::{is_valid_token_info, TOKEN_ADDRESS_LEN};
use crate::utils::NATIVE_TICKER;

pub(super) struct TokenEntry {
    pub address: [u8; TOKEN_ADDRESS_LEN],
    pub ticker: &'static str,
    pub decimals: u8,
}

pub(super) const TOKENS: &[TokenEntry] = &[
    TokenEntry {
        address: [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab,
            0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67,
            0x89, 0xab, 0xcd, 0xef,
        ],
        ticker: "USDC",
        decimals: 12,
    },
    TokenEntry {
        address: [
            0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32, 0x10, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54,
            0x32, 0x10, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32, 0x10, 0xfe, 0xdc, 0xba, 0x98,
            0x76, 0x54, 0x32, 0x10,
        ],
        ticker: "WETH",
        decimals: 14,
    },
    TokenEntry {
        address: [
            0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
            0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55,
            0x66, 0x77, 0x88, 0x99,
        ],
        ticker: "DAI",
        decimals: 14,
    },
    TokenEntry {
        address: [
            0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee,
            0xff, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc,
            0xdd, 0xee, 0xff, 0x00,
        ],
        ticker: "BTC",
        decimals: 12,
    },
    TokenEntry {
        address: [
            0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa, 0x99, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22,
            0x11, 0x00, 0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa, 0x99, 0x88, 0x77, 0x66, 0x55, 0x44,
            0x33, 0x22, 0x11, 0x00,
        ],
        ticker: "LINK",
        decimals: 14,
    },
];

// A faulty entry fails the build rather than the transactions of its token. The review identifies
// the asset by its ticker only, so tickers must be unique and differ from the native one, ignoring
// case: `usdc` next to `USDC` would confuse the user.
const _: () = {
    let mut i = 0;
    while i < TOKENS.len() {
        let entry = &TOKENS[i];
        assert!(is_valid_token_info(entry.ticker, entry.decimals));
        assert!(!bytes_eq(
            entry.ticker.as_bytes(),
            NATIVE_TICKER.as_bytes(),
            true
        ));
        let mut j = i + 1;
        while j < TOKENS.len() {
            assert!(!bytes_eq(&entry.address, &TOKENS[j].address, false));
            assert!(!bytes_eq(
                entry.ticker.as_bytes(),
                TOKENS[j].ticker.as_bytes(),
                true
            ));
            j += 1;
        }
        i += 1;
    }
};

/// Byte slice equality usable in `const` evaluation, where `==` on slices is not.
const fn bytes_eq(a: &[u8], b: &[u8], ignore_ascii_case: bool) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        let equal = if ignore_ascii_case {
            a[i].eq_ignore_ascii_case(&b[i])
        } else {
            a[i] == b[i]
        };
        if !equal {
            return false;
        }
        i += 1;
    }
    true
}
