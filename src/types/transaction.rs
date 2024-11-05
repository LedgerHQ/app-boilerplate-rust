use alloy_primitives::{Address, B256, U256};
use alloy_rlp::{encode_list, Decodable, Encodable, Error, Header, RlpDecodable, RlpEncodable};
use bytes::{Buf, BufMut, Bytes};
use alloc::vec::Vec;

pub const TX_RLP_PREFIX_2930: [u8; 4] = [0x63, 0x66, 0x78, 0x01]; // "cfx" + 1
pub const TX_RLP_PREFIX_1559: [u8; 4] = [0x63, 0x66, 0x78, 0x02]; // "cfx" + 2

pub const TX_TYPE_LEGACY: u8 = 0;
pub const TX_TYPE_EIP2930: u8 = 1;
pub const TX_TYPE_EIP1559: u8 = 2;

#[derive(Debug, Default)]
pub struct Transaction {
    pub to: Address,
    pub value: U256,
    pub nonce: u64,
    pub data: Bytes,
    pub gas: u64,
    pub gas_price: Option<U256>,
    pub storage_limit: u64,
    pub epoch_height: u64,
    pub chain_id: u64,
    pub access_list: Option<AccessList>,
    pub max_priority_fee_per_gas: Option<U256>,
    pub max_fee_per_gas: Option<U256>,
}

#[derive(Debug, Default, RlpDecodable, RlpEncodable)]
pub struct AccessListItem {
    pub address: Address,
    pub storage_keys: Vec<B256>,
}

pub type AccessList = Vec<AccessListItem>;

impl Transaction {
    pub fn tx_type(&self) -> u8 {
        if self.max_fee_per_gas.is_some() && self.max_priority_fee_per_gas.is_some() {
            TX_TYPE_EIP1559
        } else if self.gas_price.is_some() && self.access_list.is_some() {
            TX_TYPE_EIP2930
        } else { // gas_price is required
            TX_TYPE_LEGACY
        }
    }
}

impl Encodable for Transaction {
    fn encode(&self, out: &mut dyn BufMut) {
        match self.tx_type() {
            TX_TYPE_LEGACY => {
                let enc: [&dyn Encodable; 9] = [
                    &self.nonce,
                    &self.gas_price.expect("gas price is required"),
                    &self.gas,
                    &self.to,
                    &self.value,
                    &self.storage_limit,
                    &self.epoch_height,
                    &self.chain_id,
                    &self.data,
                ];
                encode_list::<_, dyn Encodable>(&enc, out);
            }
            TX_TYPE_EIP2930 => {
                out.put_slice(&TX_RLP_PREFIX_2930);
                let enc: [&dyn Encodable; 10] = [
                    &self.nonce,
                    &self.gas_price.expect("gas_price is required"),
                    &self.gas,
                    &self.to,
                    &self.value,
                    &self.storage_limit,
                    &self.epoch_height,
                    &self.chain_id,
                    &self.data,
                    &self.access_list.as_ref().expect("access_list is required"),
                ];
                encode_list::<_, dyn Encodable>(&enc, out);
            }
            TX_TYPE_EIP1559 => {
                let access_list: Vec<AccessListItem> = Vec::new();
                out.put_slice(&TX_RLP_PREFIX_1559);
                let enc: [&dyn Encodable; 11] = [
                    &self.nonce,
                    &self
                        .max_priority_fee_per_gas
                        .expect("max_priority_fee_per_gas is required"),
                    &self.max_fee_per_gas.expect("max_fee_per_gas is required"),
                    &self.gas,
                    &self.to,
                    &self.value,
                    &self.storage_limit,
                    &self.epoch_height,
                    &self.chain_id,
                    &self.data,
                    &self.access_list.as_ref().unwrap_or(&access_list),
                ];
                encode_list::<_, dyn Encodable>(&enc, out);
            }
            _ => unreachable!(),
        }
    }
}

impl Decodable for Transaction {
    fn decode(data: &mut &[u8]) -> Result<Self, Error> {
        let tx;
        let first_bytes: [u8; 4] = match data.get(0..4) {
            Some(bytes) => bytes.try_into().unwrap(),
            None => [0; 4],
        };

        match first_bytes {
            TX_RLP_PREFIX_2930 => {
                data.advance(4);
                let mut data = Header::decode_bytes(data, true)?;
                tx = Transaction {
                    nonce: u64::decode(&mut data)?,
                    gas_price: Some(U256::decode(&mut data)?),
                    gas: u64::decode(&mut data)?,
                    to: Address::decode(&mut data)?,
                    value: U256::decode(&mut data)?,
                    storage_limit: u64::decode(&mut data)?,
                    epoch_height: u64::decode(&mut data)?,
                    chain_id: u64::decode(&mut data)?,
                    data: Bytes::decode(&mut data)?,
                    access_list: Some(AccessList::decode(&mut data)?),
                    max_priority_fee_per_gas: None,
                    max_fee_per_gas: None,
                }
            }
            TX_RLP_PREFIX_1559 => {
                data.advance(4);
                let mut data = Header::decode_bytes(data, true)?;
                tx = Transaction {
                    nonce: u64::decode(&mut data)?,
                    max_priority_fee_per_gas: Some(U256::decode(&mut data)?),
                    max_fee_per_gas: Some(U256::decode(&mut data)?),
                    gas: u64::decode(&mut data)?,
                    to: Address::decode(&mut data)?,
                    value: U256::decode(&mut data)?,
                    storage_limit: u64::decode(&mut data)?,
                    epoch_height: u64::decode(&mut data)?,
                    chain_id: u64::decode(&mut data)?,
                    data: Bytes::decode(&mut data)?,
                    access_list: Some(AccessList::decode(&mut data)?),
                    gas_price: None,
                }
            }
            _ => {
                let mut data = Header::decode_bytes(data, true)?;
                tx = Transaction {
                    nonce: u64::decode(&mut data)?,
                    gas_price: Some(U256::decode(&mut data)?),
                    gas: u64::decode(&mut data)?,
                    to: Address::decode(&mut data)?,
                    value: U256::decode(&mut data)?,
                    storage_limit: u64::decode(&mut data)?,
                    epoch_height: u64::decode(&mut data)?,
                    chain_id: u64::decode(&mut data)?,
                    data: Bytes::decode(&mut data)?,
                    access_list: None,
                    max_priority_fee_per_gas: None,
                    max_fee_per_gas: None,
                }
            }
        };
        Ok(tx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::hex::FromHex;
    use rustc_hex::{FromHex as RustcFromHex, ToHex};
    use alloc::vec;

    #[test]
    fn encode_basic() {
        let mut tx = Transaction {
            to: Address::from_hex("0123456789012345678901234567890123456789")
                .expect("valid address"),
            value: U256::from(1),
            nonce: 1,
            data: Bytes::from(""),
            gas: 1,
            gas_price: Some(U256::from(1)),
            storage_limit: 1,
            epoch_height: 1,
            chain_id: 1,
            access_list: None,
            max_priority_fee_per_gas: None,
            max_fee_per_gas: None,
        };
        let mut buf = Vec::new();

        tx.encode(&mut buf);
        assert_eq!(
            buf.to_hex::<String>(),
            "dd0101019401234567890123456789012345678901234567890101010180"
        );

        tx.data = Bytes::from("hello");
        let mut buf2 = Vec::new();
        tx.encode(&mut buf2);
        assert_eq!(
            buf2.to_hex::<String>(),
            "e2010101940123456789012345678901234567890123456789010101018568656c6c6f"
        );
    }

    #[test]
    fn encode_2930() {
        let mut tx = Transaction {
            to: Address::from_hex("0123456789012345678901234567890123456789")
                .expect("valid address"),
            value: U256::from(1),
            nonce: 1,
            data: Bytes::from("hello"),
            gas: 1,
            gas_price: Some(U256::from(1)),
            storage_limit: 1,
            epoch_height: 1,
            chain_id: 1,
            access_list: Some(Vec::new()),
            max_priority_fee_per_gas: None,
            max_fee_per_gas: None,
        };
        let mut buf = Vec::new();

        tx.encode(&mut buf);
        assert_eq!(
            buf.to_hex::<String>(),
            "63667801e3010101940123456789012345678901234567890123456789010101018568656c6c6fc0"
        );

        tx.access_list = Some(vec![AccessListItem {
            address: Address::from_hex("0123456789012345678901234567890123456789")
                .expect("valid address"),
            storage_keys: vec![B256::from_hex(
                "3d709d64e3b668ddc615a5b05d6f109275096d27571d99ba02d28e84feac6b00",
            )
            .expect("valid storage key")],
        }]);
        let mut buf2 = Vec::new();
        tx.encode(&mut buf2);
        assert_eq!(buf2.to_hex::<String>(), "63667801f85c010101940123456789012345678901234567890123456789010101018568656c6c6ff838f7940123456789012345678901234567890123456789e1a03d709d64e3b668ddc615a5b05d6f109275096d27571d99ba02d28e84feac6b00");
    }

    #[test]
    fn encode_1559() {
        let mut tx = Transaction {
            to: Address::from_hex("0123456789012345678901234567890123456789")
                .expect("valid address"),
            value: U256::from(1),
            nonce: 1,
            data: Bytes::from("hello"),
            gas: 1,
            gas_price: None,
            storage_limit: 1,
            epoch_height: 1,
            chain_id: 1,
            access_list: None,
            max_priority_fee_per_gas: Some(U256::from(1)),
            max_fee_per_gas: Some(U256::from(1)),
        };
        let mut buf = Vec::new();

        tx.encode(&mut buf);
        assert_eq!(
            buf.to_hex::<String>(),
            "63667802e401010101940123456789012345678901234567890123456789010101018568656c6c6fc0"
        );

        tx.access_list = Some(vec![AccessListItem {
            address: Address::from_hex("0123456789012345678901234567890123456789")
                .expect("valid address"),
            storage_keys: vec![B256::from_hex(
                "3d709d64e3b668ddc615a5b05d6f109275096d27571d99ba02d28e84feac6b00",
            )
            .expect("valid storage key")],
        }]);
        let mut buf2 = Vec::new();
        tx.encode(&mut buf2);
        assert_eq!(buf2.to_hex::<String>(), "63667802f85d01010101940123456789012345678901234567890123456789010101018568656c6c6ff838f7940123456789012345678901234567890123456789e1a03d709d64e3b668ddc615a5b05d6f109275096d27571d99ba02d28e84feac6b00");
    }

    #[test]
    fn decode_basic() {
        let mut tx = Transaction {
            to: Address::from_hex("0123456789012345678901234567890123456789")
                .expect("valid address"),
            value: U256::from(1),
            nonce: 1,
            data: Bytes::from(""),
            gas: 1,
            gas_price: Some(U256::from(1)),
            storage_limit: 1,
            epoch_height: 1,
            chain_id: 1,
            access_list: None,
            max_priority_fee_per_gas: None,
            max_fee_per_gas: None,
        };
        // let out = encode(&tx);
        let out = "dd0101019401234567890123456789012345678901234567890101010180"
            .from_hex::<Vec<u8>>()
            .unwrap();
        let decode_tx = Transaction::decode(&mut out.as_ref()).unwrap();
        assert_eq!(tx.value, decode_tx.value);
        assert_eq!(tx.nonce, decode_tx.nonce);
        assert_eq!(tx.gas, decode_tx.gas);
        assert_eq!(tx.gas_price, decode_tx.gas_price);
        assert_eq!(tx.storage_limit, decode_tx.storage_limit);
        assert_eq!(tx.epoch_height, decode_tx.epoch_height);
        assert_eq!(tx.chain_id, decode_tx.chain_id);
        assert_eq!(tx.data, decode_tx.data);
        assert_eq!(tx.to, decode_tx.to);
        assert_eq!(tx.access_list.is_none(), decode_tx.access_list.is_none());
        assert_eq!(
            tx.max_priority_fee_per_gas,
            decode_tx.max_priority_fee_per_gas
        );
        assert_eq!(tx.max_fee_per_gas, decode_tx.max_fee_per_gas);

        tx.data = Bytes::from("hello");
        let out = "e2010101940123456789012345678901234567890123456789010101018568656c6c6f"
            .from_hex::<Vec<u8>>()
            .unwrap();
        let decode_tx = Transaction::decode(&mut out.as_ref()).unwrap();
        assert_eq!(tx.data, decode_tx.data);
    }

    #[test]
    fn decode_2930() {
        let mut tx = Transaction {
            to: Address::from_hex("0123456789012345678901234567890123456789")
                .expect("valid address"),
            value: U256::from(1),
            nonce: 1,
            data: Bytes::from("hello"),
            gas: 1,
            gas_price: Some(U256::from(1)),
            storage_limit: 1,
            epoch_height: 1,
            chain_id: 1,
            access_list: Some(vec![]),
            max_priority_fee_per_gas: None,
            max_fee_per_gas: None,
        };
        // let out = encode(&tx);
        let out =
            "63667801e3010101940123456789012345678901234567890123456789010101018568656c6c6fc0"
                .from_hex::<Vec<u8>>()
                .unwrap();
        let decode_tx = Transaction::decode(&mut out.as_ref()).unwrap();
        assert_eq!(tx.value, decode_tx.value);
        assert_eq!(tx.nonce, decode_tx.nonce);
        assert_eq!(tx.gas, decode_tx.gas);
        assert_eq!(tx.gas_price, decode_tx.gas_price);
        assert_eq!(tx.storage_limit, decode_tx.storage_limit);
        assert_eq!(tx.epoch_height, decode_tx.epoch_height);
        assert_eq!(tx.chain_id, decode_tx.chain_id);
        assert_eq!(tx.data, decode_tx.data);
        assert_eq!(tx.to, decode_tx.to);
        assert_eq!(tx.access_list.is_none(), decode_tx.access_list.is_none());
        assert_eq!(
            tx.max_priority_fee_per_gas,
            decode_tx.max_priority_fee_per_gas
        );
        assert_eq!(tx.max_fee_per_gas, decode_tx.max_fee_per_gas);

        tx.access_list = Some(vec![AccessListItem {
            address: Address::from_hex("0123456789012345678901234567890123456789")
                .expect("valid address"),
            storage_keys: vec![B256::from_hex(
                "3d709d64e3b668ddc615a5b05d6f109275096d27571d99ba02d28e84feac6b00",
            )
            .expect("valid storage key")],
        }]);
        let out = "63667801f85c010101940123456789012345678901234567890123456789010101018568656c6c6ff838f7940123456789012345678901234567890123456789e1a03d709d64e3b668ddc615a5b05d6f109275096d27571d99ba02d28e84feac6b00".from_hex::<Vec<u8>>().unwrap();
        let decode_tx = Transaction::decode(&mut out.as_ref()).unwrap();
        assert_eq!(
            tx.access_list.as_ref().expect("")[0].address,
            decode_tx.access_list.as_ref().expect("")[0].address
        );
        assert_eq!(
            tx.access_list.expect("")[0].storage_keys,
            decode_tx.access_list.expect("")[0].storage_keys
        );
    }

    #[test]
    fn decode_1559() {
        let mut tx = Transaction {
            to: Address::from_hex("0123456789012345678901234567890123456789")
                .expect("valid address"),
            value: U256::from(1),
            nonce: 1,
            data: Bytes::from("hello"),
            gas: 1,
            gas_price: None,
            storage_limit: 1,
            epoch_height: 1,
            chain_id: 1,
            access_list: None,
            max_priority_fee_per_gas: Some(U256::from(1)),
            max_fee_per_gas: Some(U256::from(1)),
        };
        // let out = encode(&tx);
        let out =
            "63667802e401010101940123456789012345678901234567890123456789010101018568656c6c6fc0"
                .from_hex::<Vec<u8>>()
                .unwrap();
        let decode_tx = Transaction::decode(&mut out.as_ref()).unwrap();
        assert_eq!(tx.value, decode_tx.value);
        assert_eq!(tx.nonce, decode_tx.nonce);
        assert_eq!(tx.gas, decode_tx.gas);
        assert_eq!(tx.gas_price, decode_tx.gas_price);
        assert_eq!(tx.storage_limit, decode_tx.storage_limit);
        assert_eq!(tx.epoch_height, decode_tx.epoch_height);
        assert_eq!(tx.chain_id, decode_tx.chain_id);
        assert_eq!(tx.data, decode_tx.data);
        assert_eq!(tx.to, decode_tx.to);
        // assert_eq!(tx.access_list.is_none(), decode_tx.access_list.is_none());
        assert_eq!(decode_tx.access_list.unwrap().len(), 0);
        assert_eq!(
            tx.max_priority_fee_per_gas,
            decode_tx.max_priority_fee_per_gas
        );
        assert_eq!(tx.max_fee_per_gas, decode_tx.max_fee_per_gas);

        tx.access_list = Some(vec![AccessListItem {
            address: Address::from_hex("0123456789012345678901234567890123456789")
                .expect("valid address"),
            storage_keys: vec![B256::from_hex(
                "3d709d64e3b668ddc615a5b05d6f109275096d27571d99ba02d28e84feac6b00",
            )
            .expect("valid storage key")],
        }]);
        let out = "63667802f85d01010101940123456789012345678901234567890123456789010101018568656c6c6ff838f7940123456789012345678901234567890123456789e1a03d709d64e3b668ddc615a5b05d6f109275096d27571d99ba02d28e84feac6b00".from_hex::<Vec<u8>>().unwrap();
        let decode_tx = Transaction::decode(&mut out.as_ref()).unwrap();
        assert_eq!(tx.data, decode_tx.data);
        assert_eq!(
            tx.access_list.as_ref().expect("")[0].address,
            decode_tx.access_list.as_ref().expect("")[0].address
        );
        assert_eq!(
            tx.access_list.expect("")[0].storage_keys,
            decode_tx.access_list.expect("")[0].storage_keys
        );
    }
}
