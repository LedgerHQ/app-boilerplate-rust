use crate::AppSW;
use core::char;

pub const MAX_ALLOWED_PATH_LEN: usize = 10;
const MAX_HEX_LEN: usize = 64;

/// Convert to hex. Returns a static buffer of 64 bytes
#[inline]
pub fn to_hex(m: &[u8]) -> Result<[u8; MAX_HEX_LEN], ()> {
    if 2 * m.len() > MAX_HEX_LEN {
        return Err(());
    }
    let mut hex = [0u8; MAX_HEX_LEN];
    let mut i = 0;
    for c in m {
        let c0 = char::from_digit((c >> 4).into(), 16).unwrap();
        let c1 = char::from_digit((c & 0xf).into(), 16).unwrap();
        hex[i] = c0 as u8;
        hex[i + 1] = c1 as u8;
        i += 2;
    }
    Ok(hex)
}

/// Convert to an all capitalized string. Returns a static buffer of 255 bytes
#[inline]
pub fn to_hex_all_caps(m: &[u8]) -> Result<[u8; MAX_HEX_LEN], ()> {
    match to_hex(m) {
        Ok(hex) => {
            let mut hex_all_caps = hex;
            hex_all_caps
                .iter_mut()
                .for_each(|x| *x = x.to_ascii_uppercase());
            Ok(hex_all_caps)
        }
        Err(_) => Err(()),
    }
}

/// Convert serialized derivation path to u32 array elements
pub fn read_bip32_path(data: &[u8], path: &mut [u32]) -> Result<usize, AppSW> {
    // Check input length and path buffer capacity
    if data.len() < 1 || path.len() < data.len() / 4 {
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

/// Get a subslice of a slice or return an error
#[inline]
pub fn slice_or_err(slice: &[u8], start: usize, len: usize) -> Result<&[u8], ()> {
    match slice.get(start..start + len) {
        Some(s) => Ok(s),
        None => Err(()),
    }
}

/// Read a varint from a slice
pub fn varint_read(input: &[u8]) -> Result<(u64, usize), ()> {
    let mut bytes = [0u8; 8];
    let int_length: usize;

    if input.is_empty() {
        return Err(());
    }

    let prefix = input[0];

    if prefix == 0xFD {
        if input.len() < 3 {
            return Err(());
        }
        int_length = 2;
    } else if prefix == 0xFE {
        if input.len() < 5 {
            return Err(());
        }
        int_length = 4;
    } else if prefix == 0xFF {
        if input.len() < 9 {
            return Err(());
        }
        int_length = 8;
    } else {
        return Ok((u64::from(prefix), 1));
    }

    let buf = slice_or_err(input, 1, int_length)?;
    bytes[..int_length].copy_from_slice(buf);
    let result = u64::from_le_bytes(bytes);
    Ok((result, int_length + 1))
}
