use crate::AppSW;
use core::str::from_utf8;

/// BIP32 path stored as an array of [`u32`].
///
/// # Generic arguments
///
/// * `S` - Maximum possible path length, i.e. the capacity of the internal buffer.
pub struct Bip32Path<const S: usize = 10> {
    buffer: [u32; S],
    len: usize,
}

impl AsRef<[u32]> for Bip32Path {
    fn as_ref(&self) -> &[u32] {
        &self.buffer[..self.len]
    }
}

impl<const S: usize> Default for Bip32Path<S> {
    fn default() -> Self {
        Self {
            buffer: [0u32; S],
            len: 0,
        }
    }
}

impl<const S: usize> TryFrom<&[u8]> for Bip32Path<S> {
    type Error = AppSW;

    /// Constructs a [`Bip32Path`] from a given byte array.
    ///
    /// This method will return an error in the following cases:
    /// - the input array is empty,
    /// - the number of bytes in the input array is not a multiple of 4,
    /// - the input array exceeds the capacity of the [`Bip32Path`] internal buffer.
    ///
    /// # Arguments
    ///
    /// * `data` - Encoded BIP32 path. First byte is the length of the path, as encoded by ragger.
    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let input_path_len = (data.len() - 1) / 4;
        // Check data length
        if data.is_empty() // At least the length byte is required
            || (input_path_len > S)
            || (data[0] as usize * 4 != data.len() - 1)
        {
            return Err(AppSW::WrongApduLength);
        }

        let mut path = [0; S];
        for (chunk, p) in data[1..].chunks(4).zip(path.iter_mut()) {
            *p = u32::from_be_bytes(chunk.try_into().unwrap());
        }

        Ok(Self {
            buffer: path,
            len: input_path_len,
        })
    }
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
