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

use core::str::FromStr;
use nanos_sdk::io;

pub fn handler_get_version(comm: &mut io::Comm) -> Result<(), io::Reply> {
    if let Some((major, minor, patch)) = parse_version_string(env!("CARGO_PKG_VERSION")) {
        comm.append(&[major, minor, patch]);
        Ok(())
    } else {
        Err(io::StatusWords::Unknown.into())
    }
}

fn parse_version_string(input: &str) -> Option<(u8, u8, u8)> {
    // Split the input string by '.'.
    // Input should be of the form "major.minor.patch",
    // where "major", "minor", and "patch" are integers.
    let mut parts = input.split('.');
    let major = u8::from_str(parts.next()?).ok()?;
    let minor = u8::from_str(parts.next()?).ok()?;
    let patch = u8::from_str(parts.next()?).ok()?;
    Some((major, minor, patch))
}
