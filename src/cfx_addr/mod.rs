#![allow(unused)]
mod consts;
mod utils;
mod types;

use consts::*;
use utils::*;
use types::*;

use alloc::{string::String, vec::Vec};

pub fn cfx_addr_encode(
    raw: &[u8],
    network: Network
) -> Result<String, EncodingError> {
    // Calculate version byte
    let length = raw.len();
    let version_byte = match length {
        20 => consts::SIZE_160,
        // Conflux does not have other hash sizes. We don't use the sizes below
        // but we kept these for unit tests.
        24 => consts::SIZE_192,
        28 => consts::SIZE_224,
        32 => consts::SIZE_256,
        40 => consts::SIZE_320,
        48 => consts::SIZE_384,
        56 => consts::SIZE_448,
        64 => consts::SIZE_512,
        _ => return Err(EncodingError::InvalidLength(length)),
    };

    // Get prefix
    let prefix = network.to_prefix()?;

    // Convert payload to 5 bit array
    let mut payload = Vec::with_capacity(1 + raw.len());
    payload.push(version_byte);
    payload.extend(raw);
    let payload_5_bits =
        convert_bits(&payload, 8, 5, true).expect("no error is possible for encoding");

    // Construct payload string using CHARSET
    let payload_str: String = payload_5_bits
        .iter()
        .map(|b| CHARSET[*b as usize] as char)
        .collect();

    // Create checksum
    let expanded_prefix = expand_prefix(&prefix);
    let checksum_input = [&expanded_prefix[..], &payload_5_bits, &[0; 8][..]].concat();
    let checksum = polymod(&checksum_input);

    // Convert checksum to string
    let checksum_str: String = (0..8)
        .rev()
        .map(|i| CHARSET[((checksum >> (i * 5)) & 31) as usize] as char)
        .collect();

    let cfx_base32_addr = [&prefix, ":", &payload_str, &checksum_str].concat();

    Ok(cfx_base32_addr)
}