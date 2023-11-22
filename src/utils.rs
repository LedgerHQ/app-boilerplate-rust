use crate::AppSW;
use core::char;

pub const MAX_ALLOWED_PATH_LEN: usize = 10;
const MAX_HEX_LEN: usize = 64;

/// Convert serialized derivation path to u32 array elements
pub fn read_bip32_path(data: &[u8], path: &mut [u32]) -> Result<usize, AppSW> {
    // Check input length and path buffer capacity
    if data.is_empty() || path.len() < data.len() / 4 {
        return Err(AppSW::WrongDataLength);
    }

    let path_len = data[0] as usize; // First byte is the length of the path
    let path_data = &data[1..];

    // Check path data length and alignment
    if path_data.len() != path_len * 4
        || path_data.len() > MAX_ALLOWED_PATH_LEN * 4
        || path_data.len() % 4 != 0
    {
        return Err(AppSW::WrongDataLength);
    }

    let mut idx = 0;
    for (i, chunk) in path_data.chunks(4).enumerate() {
        path[idx] = u32::from_be_bytes(chunk.try_into().unwrap());
        idx = i + 1;
    }

    Ok(idx)
}

/// Concatenate multiple strings into a fixed-size array
pub fn concatenate(strings: &[&str], output: &mut [u8]) {
    let mut offset = 0;

    for s in strings {
        let s_len = s.len();
        let copy_len = core::cmp::min(s_len, output.len() - offset);

        if copy_len > 0 {
            output[offset..offset + copy_len].copy_from_slice(&s.as_bytes()[..copy_len]);
            offset += copy_len;
        } else {
            // If the output buffer is full, stop concatenating.
            break;
        }
    }
}
