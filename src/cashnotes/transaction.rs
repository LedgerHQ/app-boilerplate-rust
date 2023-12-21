// Copyright (c) 2023, MaidSafe.
// All rights reserved.
//
// This SAFE Network Software is licensed under the BSD-3-Clause license.
// Please see the LICENSE file for more details.

use super::NanoTokens;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

#[derive(Eq, PartialEq, Debug, Clone, Serialize, Deserialize, Hash)]
pub struct Input {
    #[serde(with = "BigArray")]
    pub unique_pubkey: [u8; 48],
    pub amount: NanoTokens,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct Output {
    #[serde(with = "BigArray")]
    pub unique_pubkey: [u8; 48],
    pub amount: NanoTokens,
}

#[derive(Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct Transaction {
    pub inputs: [Input; 1],
    pub outputs: [Output; 1],
}
