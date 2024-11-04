use alloc::vec::Vec;

// The polymod function is used to calculate the checksum of the address.
pub fn polymod(v: &[u8]) -> u64 {
    let mut c = 1;
    for d in v {
        let c0 = (c >> 35) as u8;
        c = ((c & 0x07ffffffff) << 5) ^ u64::from(*d);
        if c0 & 0x01 != 0 {
            c ^= 0x98f2bc8e61;
        }
        if c0 & 0x02 != 0 {
            c ^= 0x79b76d99e2;
        }
        if c0 & 0x04 != 0 {
            c ^= 0xf33e5fb3c4;
        }
        if c0 & 0x08 != 0 {
            c ^= 0xae2eabe2a8;
        }
        if c0 & 0x10 != 0 {
            c ^= 0x1e4f43e470;
        }
    }
    c ^ 1
}

/// The checksum calculation includes the lower 5 bits of each character of the
/// prefix.
/// - e.g. "bit..." becomes 2,9,20,...
// Expand the address prefix for the checksum operation.
pub fn expand_prefix(prefix: &str) -> Vec<u8> {
    let mut ret: Vec<u8> = prefix.chars().map(|c| (c as u8) & 0x1f).collect();
    ret.push(0);
    ret
}

// This method assume that data is valid string of inbits.
// When pad is true, any remaining bits are padded and encoded into a new byte;
// when pad is false, any remaining bits are checked to be zero and discarded.
pub fn convert_bits(
    data: &[u8],
    inbits: u8,
    outbits: u8,
    pad: bool,
) -> Result<Vec<u8>, &'static str> {
    assert!(inbits <= 8 && outbits <= 8);
    let num_bytes = (data.len() * inbits as usize + outbits as usize - 1) / outbits as usize;
    let mut ret = Vec::with_capacity(num_bytes);
    let mut acc: u16 = 0; // accumulator of bits
    let mut num: u8 = 0; // num bits in acc
    let groupmask = (1 << outbits) - 1;
    for d in data.iter() {
        // We push each input chunk into a 16-bit accumulator
        acc = (acc << inbits) | u16::from(*d);
        num += inbits;
        // Then we extract all the output groups we can
        while num >= outbits {
            // Store only the highest outbits.
            ret.push((acc >> (num - outbits)) as u8);
            // Clear the highest outbits.
            acc &= !(groupmask << (num - outbits));
            num -= outbits;
        }
    }
    if pad {
        // If there's some bits left, pad and add it
        if num > 0 {
            ret.push((acc << (outbits - num)) as u8);
        }
    } else {
        // FIXME: add unit tests for it.
        // If there's some bits left, figure out if we need to remove padding
        // and add it
        let padding = ((data.len() * inbits as usize) % outbits as usize) as u8;
        if num >= inbits || acc != 0 {
            return Err("InvalidPadding");
        }
    }
    Ok(ret)
}
