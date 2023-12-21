use ledger_device_sdk::testing;

use crate::AppSW;
use core::str::from_utf8;

pub const MAX_ALLOWED_PATH_LEN: usize = 10;

/// Required length of the buffer to hold a PK formatted for UI display
pub const BUFFER_LEN_FOR_PK_BYTES_TO_DISPLAY: usize = 4 + 4 * PK_BYTES_TO_DISPLAY;

// Display only the first and last PK_BYTES_TO_DISPLAY bytes of the address
const PK_BYTES_TO_DISPLAY: usize = 2;

/// Convert serialized derivation path to u32 array elements
pub fn read_bip32_path(data: &[u8], path: &mut [u32]) -> Result<usize, AppSW> {
    // Check input length and path buffer capacity
    if data.is_empty() || path.len() < data.len() / 4 {
        return Err(AppSW::WrongDataLength);
    }
    testing::debug_print("path length first check is ok!!\n");

    let path_len = data[0] as usize; // First byte is the length of the path
    let path_data = &data[1..];

    concat_and_debug_print("path length:", path_data.len())?;

    // Check path data length and alignment
    if path_data.len() != path_len * 4
        || path_data.len() > MAX_ALLOWED_PATH_LEN * 4
        || path_data.len() % 4 != 0
    {
        return Err(AppSW::WrongDataLength);
    }
    testing::debug_print("path length is ok!\n");
    let mut idx = 0;
    for (i, chunk) in path_data.chunks(4).enumerate() {
        path[idx] = u32::from_be_bytes(chunk.try_into().unwrap());
        idx = i + 1;
    }

    Ok(idx)
}

/// Concatenate a value as a base10 number to a string and debug_print it
pub fn concat_and_debug_print(p: &str, v: usize) -> Result<(), AppSW> {
    let mut numtoa_buf = [0u8; 20];
    let mut value_buf = [0u8; 40];
    use numtoa::NumToA;
    let value_str = concatenate(
        &[p, " ", v.numtoa_str(10, &mut numtoa_buf), "\n"],
        &mut value_buf,
    )
    .map_err(|_| AppSW::WrongDataLength)?; // Fails if value_buf is too small

    testing::debug_print(value_str);

    Ok(())
}

/// Returns concatenated strings, or an error if the concatenation buffer is too small.
pub fn concatenate<'a>(strings: &[&str], output: &'a mut [u8]) -> Result<&'a str, ()> {
    let mut offset = 0;

    for s in strings {
        let s_len = s.len();
        if offset + s_len > output.len() {
            return Err(());
        }

        output[offset..offset + s_len].copy_from_slice(s.as_bytes());
        offset += s_len;
    }

    Ok(from_utf8(&output[..offset]).unwrap())
}

/// Generate address string in hexadecimal format for UI from given BLS PK.
pub fn addr_hex_for_ui<'a>(
    pk: &[u8],
    output: &'a mut [u8; BUFFER_LEN_FOR_PK_BYTES_TO_DISPLAY],
) -> Result<&'a str, AppSW> {
    let mut addr_str_head = [0u8; 2 + 2 * PK_BYTES_TO_DISPLAY];
    addr_str_head[..2].copy_from_slice("0x".as_bytes());
    hex::encode_to_slice(&pk[..PK_BYTES_TO_DISPLAY], &mut addr_str_head[2..])
        .map_err(|_| AppSW::AddrDisplayFail)?;
    addr_str_head[2..].make_ascii_uppercase();

    let mut addr_str_tail = [0u8; 2 + 2 * PK_BYTES_TO_DISPLAY];
    addr_str_tail[..2].copy_from_slice("..".as_bytes());
    let pk_len = pk.len();
    hex::encode_to_slice(&pk[pk_len - PK_BYTES_TO_DISPLAY..], &mut addr_str_tail[2..])
        .map_err(|_| AppSW::AddrDisplayFail)?;
    addr_str_tail[2..].make_ascii_uppercase();

    concatenate(
        &[
            core::str::from_utf8(&addr_str_head).unwrap(),
            core::str::from_utf8(&addr_str_tail).unwrap(),
        ],
        output,
    )
    .map_err(|_| AppSW::AddrDisplayFail)
}
