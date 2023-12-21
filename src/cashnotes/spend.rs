// Copyright 2023 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.
#![allow(unused_variables)]

use super::{Hash, NanoTokens, Transaction};

use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

/// Represents the data to be signed by the DerivedSecretKey of the CashNote being spent.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spend {
    /// UniquePubkey of input CashNote that this SignedSpend is proving to be spent.
    #[serde(with = "BigArray")]
    pub unique_pubkey: [u8; 48],
    /// The transaction that the input CashNote is being spent in (where it is an input)
    //#[debug(skip)]
    pub spent_tx: Transaction,
    /// Reason why this CashNote was spent.
    //#[debug(skip)]
    pub reason: Hash,
    /// The amount of the input CashNote.
    pub token: NanoTokens,
    /// The transaction that the input CashNote was created in (where it is an output)
    //#[debug(skip)]
    pub parent_tx: Transaction,
}
