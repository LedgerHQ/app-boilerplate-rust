// Copyright 2023 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.
#![allow(dead_code)]
#![allow(unused_variables)]

use serde::{Deserialize, Serialize};

/// The conversion from NanoTokens to raw value
const TOKEN_TO_RAW_POWER_OF_10_CONVERSION: u32 = 9;

/// The conversion from NanoTokens to raw value
const TOKEN_TO_RAW_CONVERSION: u64 = 1_000_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// An amount in SNT Nanos. 10^9 Nanos = 1 SNT.
pub struct NanoTokens(u64);

impl NanoTokens {
    /// Type safe representation of zero NanoTokens.
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Returns whether it's a representation of zero NanoTokens.
    pub const fn is_zero(&self) -> bool {
        self.0 == 0
    }

    /// New value from a number of nano tokens.
    pub const fn from(value: u64) -> Self {
        Self(value)
    }

    /// Total NanoTokens expressed in number of nano tokens.
    pub const fn as_nano(self) -> u64 {
        self.0
    }

    /// Computes `self + rhs`, returning `None` if overflow occurred.
    pub fn checked_add(self, rhs: NanoTokens) -> Option<NanoTokens> {
        self.0.checked_add(rhs.0).map(Self::from)
    }

    /// Computes `self - rhs`, returning `None` if overflow occurred.
    pub fn checked_sub(self, rhs: NanoTokens) -> Option<NanoTokens> {
        self.0.checked_sub(rhs.0).map(Self::from)
    }

    /// Converts the Nanos into bytes
    pub fn to_bytes(&self) -> [u8; 8] {
        self.0.to_ne_bytes()
    }
}

impl From<u64> for NanoTokens {
    fn from(value: u64) -> Self {
        Self(value)
    }
}
