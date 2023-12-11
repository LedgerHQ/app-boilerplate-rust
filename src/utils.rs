use crate::AppSW;
use core::str::from_utf8;

pub const MAX_ALLOWED_PATH_LEN: usize = 10;

/// Convert serialized derivation path to u32 array elements
pub fn read_bip32_path(data: &[u8], path: &mut [u32]) -> Result<usize, AppSW> {
    // Check input length and path buffer capacity
    if data.is_empty() || path.len() < data.len() / 4 {
        return Err(AppSW::WrongApduLength);
    }

    let path_len = data[0] as usize; // First byte is the length of the path
    let path_data = &data[1..];

    // Check path data length and alignment
    if path_data.len() != path_len * 4
        || path_data.len() > MAX_ALLOWED_PATH_LEN * 4
        || path_data.len() % 4 != 0
    {
        return Err(AppSW::WrongApduLength);
    }

    let mut idx = 0;
    for (i, chunk) in path_data.chunks(4).enumerate() {
        path[idx] = u32::from_be_bytes(chunk.try_into().unwrap());
        idx = i + 1;
    }

    Ok(idx)
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
