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

//! Tokens this app can sign transfers of.
//!
//! A token is identified by its address only. Its ticker and decimals come from what the app knows
//! of that address, never from the transaction, so a host cannot make a token look like another.

mod db;

use arrayvec::ArrayString;

use crate::utils::{MAX_AMOUNT_LEN, MAX_DECIMALS};

/// Length of the address identifying a token.
pub const TOKEN_ADDRESS_LEN: usize = 32;
/// Longest token ticker.
pub const MAX_TICKER_LEN: usize = 12;

// `get_printable_amount` writes swap amounts as `{ticker} {amount}`. For a coin configuration
// describing a token of this app, any amount a transaction can carry fits.
const _: () = assert!(MAX_TICKER_LEN + 1 + MAX_AMOUNT_LEN <= crate::swap::MAX_PRINTABLE_AMOUNT_LEN);

/// Display metadata of a token.
///
/// The ticker is 1 to [`MAX_TICKER_LEN`] printable ASCII characters without spaces, and the
/// decimals are at most [`MAX_DECIMALS`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TokenInfo {
    ticker: ArrayString<MAX_TICKER_LEN>,
    decimals: u8,
}

impl TokenInfo {
    /// Returns `None` if the ticker or the decimals cannot be displayed.
    pub fn new(ticker: &str, decimals: u8) -> Option<Self> {
        if !is_valid_token_info(ticker, decimals) {
            return None;
        }
        Some(TokenInfo {
            ticker: ArrayString::from(ticker).ok()?,
            decimals,
        })
    }

    pub fn ticker(&self) -> &str {
        self.ticker.as_str()
    }

    pub fn decimals(&self) -> u8 {
        self.decimals
    }
}

/// Whether a ticker and decimals satisfy the invariants of [`TokenInfo`].
///
/// `const` so that the hardcoded token list is checked at compile time.
const fn is_valid_token_info(ticker: &str, decimals: u8) -> bool {
    let ticker = ticker.as_bytes();
    if ticker.is_empty() || ticker.len() > MAX_TICKER_LEN || decimals > MAX_DECIMALS {
        return false;
    }
    let mut i = 0;
    while i < ticker.len() {
        if !ticker[i].is_ascii_graphic() {
            return false;
        }
        i += 1;
    }
    true
}

/// Returns the metadata of the token at `address`, or `None` if the app does not know it.
///
/// Does not allocate, so it can run while the Exchange app shares BSS memory.
pub fn lookup(address: &[u8; TOKEN_ADDRESS_LEN]) -> Option<TokenInfo> {
    let entry = db::TOKENS.iter().find(|entry| &entry.address == address)?;
    TokenInfo::new(entry.ticker, entry.decimals)
}
