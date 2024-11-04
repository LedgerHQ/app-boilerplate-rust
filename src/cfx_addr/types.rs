use super::consts::{MAINNET_PREFIX, RESERVED_NETWORK_IDS, TESTNET_PREFIX};
use alloc::{format, string::String};
use core::fmt;

#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash)]
pub enum Network {
    /// Main network.
    Main,
    /// Test network.
    Test,
    /// Specific Network Id.
    Id(u64),
}

impl Network {
    pub fn to_prefix(&self) -> Result<String, EncodingError> {
        match self {
            Network::Main => Ok(MAINNET_PREFIX.into()),
            Network::Test => Ok(TESTNET_PREFIX.into()),
            Network::Id(network_id) => {
                if RESERVED_NETWORK_IDS.contains(network_id) {
                    Err(EncodingError::InvalidNetworkId(*network_id))
                } else {
                    Ok(format!("net{}", network_id))
                }
            }
        }
    }

    pub fn from_network_id(network_id: u64) -> Self {
        match network_id {
            1029 => Self::Main,
            1 => Self::Test,
            _ => Self::Id(network_id),
        }
    }
}

/// Error concerning encoding of cfx_base32_addr.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum EncodingError {
    InvalidAddressType(u8),
    InvalidLength(usize),
    InvalidNetworkId(u64),
}

impl fmt::Display for EncodingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidAddressType(type_byte) => {
                write!(f, "unrecognized type bits 0x{:02x}", type_byte)
            }
            Self::InvalidLength(length) => {
                write!(f, "invalid length ({})", length)
            }
            Self::InvalidNetworkId(network_id) => {
                write!(f, "invalid network_id (reserved: {})", network_id)
            }
        }
    }
}

#[derive(Copy, Clone)]
pub enum EncodingOptions {
    Simple,
    QrCode,
}
