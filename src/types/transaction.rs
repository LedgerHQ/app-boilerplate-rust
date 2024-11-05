use alloc::vec::Vec;
use bytes::Bytes;
use ethereum_types::{Address, H256, U256};
use rlp::{Decodable, DecoderError, Encodable, Rlp, RlpDecodable, RlpEncodable, RlpStream};

pub const TX_RLP_PREFIX_2930: [u8; 4] = [0x63, 0x66, 0x78, 0x01]; // "cfx" + 1
pub const TX_RLP_PREFIX_1559: [u8; 4] = [0x63, 0x66, 0x78, 0x02]; // "cfx" + 2

pub const TX_TYPE_LEGACY: u8 = 0;
pub const TX_TYPE_EIP2930: u8 = 1;
pub const TX_TYPE_EIP1559: u8 = 2;

#[derive(Debug, Default, Clone)]
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

#[derive(Debug, Clone, Default, RlpEncodable, RlpDecodable, PartialEq, Eq)]
pub struct AccessListItem {
    pub address: Address,
    pub storage_keys: Vec<H256>,
}

pub type AccessList = Vec<AccessListItem>;

impl Transaction {
    pub fn tx_type(&self) -> u8 {
        if self.max_fee_per_gas.is_some() && self.max_priority_fee_per_gas.is_some() {
            TX_TYPE_EIP1559
        } else if self.gas_price.is_some() && self.access_list.is_some() {
            TX_TYPE_EIP2930
        } else {
            // gas_price is required
            TX_TYPE_LEGACY
        }
    }
}

impl Decodable for Transaction {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        if rlp.as_raw().len() == 0 {
            return Err(DecoderError::RlpInvalidLength);
        };
        if rlp.is_list() {
            if rlp.item_count()? != 9 {
                return Err(DecoderError::RlpInvalidLength);
            }
            Ok(Transaction {
                nonce: rlp.val_at(0)?,
                gas_price: Some(rlp.val_at(1)?),
                gas: rlp.val_at(2)?,
                to: rlp.val_at(3)?,
                value: rlp.val_at(4)?,
                storage_limit: rlp.val_at(5)?,
                epoch_height: rlp.val_at(6)?,
                chain_id: rlp.val_at(7)?,
                data: rlp.val_at(8)?,
                access_list: None,
                max_priority_fee_per_gas: None,
                max_fee_per_gas: None,
            })
        } else {
            let data = rlp.as_raw();
            let first4_bytes: [u8; 4] = match data.get(0..4) {
                Some(bytes) => bytes.try_into().unwrap(),
                None => [0; 4],
            };
            let rlp = Rlp::new(&data[4..]);
            match first4_bytes {
                TX_RLP_PREFIX_2930 => {
                    if rlp.item_count()? != 10 {
                        return Err(DecoderError::RlpInvalidLength);
                    }
                    Ok(Transaction {
                        nonce: rlp.val_at(0)?,
                        gas_price: Some(rlp.val_at(1)?),
                        gas: rlp.val_at(2)?,
                        to: rlp.val_at(3)?,
                        value: rlp.val_at(4)?,
                        storage_limit: rlp.val_at(5)?,
                        epoch_height: rlp.val_at(6)?,
                        chain_id: rlp.val_at(7)?,
                        data: rlp.val_at(8)?,
                        access_list: Some(rlp.list_at(9)?),
                        max_priority_fee_per_gas: None,
                        max_fee_per_gas: None,
                    })
                }
                TX_RLP_PREFIX_1559 => {
                    if rlp.item_count()? != 11 {
                        return Err(DecoderError::RlpInvalidLength);
                    }
                    Ok(Transaction {
                        nonce: rlp.val_at(0)?,
                        gas_price: None,
                        max_priority_fee_per_gas: Some(rlp.val_at(1)?),
                        max_fee_per_gas: Some(rlp.val_at(2)?),
                        gas: rlp.val_at(3)?,
                        to: rlp.val_at(4)?,
                        value: rlp.val_at(5)?,
                        storage_limit: rlp.val_at(6)?,
                        epoch_height: rlp.val_at(7)?,
                        chain_id: rlp.val_at(8)?,
                        data: rlp.val_at(9)?,
                        access_list: Some(rlp.list_at(10)?),
                    })
                }
                _ => {
                    return Err(DecoderError::Custom("invalid raw transaction"));
                }
            }
        }
    }
}

impl Encodable for Transaction {
    fn rlp_append(&self, stream: &mut RlpStream) {
        match self.tx_type() {
            TX_TYPE_LEGACY => {
                stream.begin_list(9);
                stream.append(&self.nonce);
                stream.append(&self.gas_price.expect("gas_price is required"));
                stream.append(&self.gas);
                stream.append(&self.to);
                stream.append(&self.value);
                stream.append(&self.storage_limit);
                stream.append(&self.epoch_height);
                stream.append(&self.chain_id);
                stream.append(&self.data);
            }
            TX_TYPE_EIP2930 => {
                stream.append_raw(&TX_RLP_PREFIX_2930, 0);
                stream.begin_list(10);
                stream.append(&self.nonce);
                stream.append(&self.gas_price.expect("gas_price is required"));
                stream.append(&self.gas);
                stream.append(&self.to);
                stream.append(&self.value);
                stream.append(&self.storage_limit);
                stream.append(&self.epoch_height);
                stream.append(&self.chain_id);
                stream.append(&self.data);
                stream.append_list(&self.access_list.as_ref().expect("access_list is required"));
            }
            TX_TYPE_EIP1559 => {
                let empty_access_list: Vec<AccessListItem> = Vec::new();
                stream.append_raw(&TX_RLP_PREFIX_1559, 0);
                stream.begin_list(11);
                stream.append(&self.nonce);
                stream.append(
                    &self
                        .max_priority_fee_per_gas
                        .expect("max_priority_fee_per_gas is required"),
                );
                stream.append(&self.max_fee_per_gas.expect("max_fee_per_gas is required"));
                stream.append(&self.gas);
                stream.append(&self.to);
                stream.append(&self.value);
                stream.append(&self.storage_limit);
                stream.append(&self.epoch_height);
                stream.append(&self.chain_id);
                stream.append(&self.data);
                stream.append_list(&self.access_list.as_ref().unwrap_or(&empty_access_list));
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rlp::{decode, encode};
    use rustc_hex::{FromHex, ToHex};

    #[test]
    fn encode_basic() {
        let to = Address::from_slice(
            hex::decode("0123456789012345678901234567890123456789")
                .unwrap()
                .as_slice(),
        );
        let mut tx = Transaction {
            to,
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

        let buf = encode(&tx);
        assert_eq!(
            buf.to_hex::<String>(),
            "dd0101019401234567890123456789012345678901234567890101010180"
        );

        tx.data = Bytes::from("hello");
        let buf2 = encode(&tx);
        assert_eq!(
            buf2.to_hex::<String>(),
            "e2010101940123456789012345678901234567890123456789010101018568656c6c6f"
        );
    }

    #[test]
    fn encode_2930() {
        let to = Address::from_slice(
            hex::decode("0123456789012345678901234567890123456789")
                .unwrap()
                .as_slice(),
        );
        let mut tx = Transaction {
            to,
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

        let buf = encode(&tx);
        assert_eq!(
            buf.to_hex::<String>(),
            "63667801e3010101940123456789012345678901234567890123456789010101018568656c6c6fc0"
        );

        tx.access_list = Some(vec![AccessListItem {
            address: to,
            storage_keys: vec![H256::from_slice(
                hex::decode("3d709d64e3b668ddc615a5b05d6f109275096d27571d99ba02d28e84feac6b00")
                    .unwrap()
                    .as_slice(),
            )],
        }]);

        let buf2 = encode(&tx);
        assert_eq!(buf2.to_hex::<String>(), "63667801f85c010101940123456789012345678901234567890123456789010101018568656c6c6ff838f7940123456789012345678901234567890123456789e1a03d709d64e3b668ddc615a5b05d6f109275096d27571d99ba02d28e84feac6b00");
    }

    #[test]
    fn encode_1559() {
        let to = Address::from_slice(
            hex::decode("0123456789012345678901234567890123456789")
                .unwrap()
                .as_slice(),
        );
        let mut tx = Transaction {
            to,
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

        let buf = encode(&tx);
        assert_eq!(
            buf.to_hex::<String>(),
            "63667802e401010101940123456789012345678901234567890123456789010101018568656c6c6fc0"
        );

        tx.access_list = Some(vec![AccessListItem {
            address: to,
            storage_keys: vec![H256::from_slice(
                hex::decode("3d709d64e3b668ddc615a5b05d6f109275096d27571d99ba02d28e84feac6b00")
                    .unwrap()
                    .as_slice(),
            )],
        }]);
        let buf2 = encode(&tx);
        assert_eq!(buf2.to_hex::<String>(), "63667802f85d01010101940123456789012345678901234567890123456789010101018568656c6c6ff838f7940123456789012345678901234567890123456789e1a03d709d64e3b668ddc615a5b05d6f109275096d27571d99ba02d28e84feac6b00");
    }

    #[test]
    fn decode_basic() {
        let to = Address::from_slice(
            hex::decode("0123456789012345678901234567890123456789")
                .unwrap()
                .as_slice(),
        );
        let mut tx = Transaction {
            to,
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
        let decode_tx: Transaction = decode(&out).expect("decode should success");
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
        let decode_tx: Transaction = decode(&out).expect("decode should success");
        assert_eq!(tx.data, decode_tx.data);
    }

    #[test]
    fn decode_2930() {
        let to = Address::from_slice(
            hex::decode("0123456789012345678901234567890123456789")
                .unwrap()
                .as_slice(),
        );
        let mut tx = Transaction {
            to,
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
        let decode_tx: Transaction = decode(&out).expect("decode should success");
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
            address: to,
            storage_keys: vec![H256::from_slice(
                hex::decode("3d709d64e3b668ddc615a5b05d6f109275096d27571d99ba02d28e84feac6b00")
                    .unwrap()
                    .as_slice(),
            )],
        }]);
        let out = "63667801f85c010101940123456789012345678901234567890123456789010101018568656c6c6ff838f7940123456789012345678901234567890123456789e1a03d709d64e3b668ddc615a5b05d6f109275096d27571d99ba02d28e84feac6b00".from_hex::<Vec<u8>>().unwrap();
        let decode_tx: Transaction = decode(&out).expect("decode should success");
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
        let to = Address::from_slice(
            hex::decode("0123456789012345678901234567890123456789")
                .unwrap()
                .as_slice(),
        );
        let mut tx = Transaction {
            to,
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
        let decode_tx: Transaction = decode(&out).expect("decode should success");
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
            address: to,
            storage_keys: vec![H256::from_slice(
                hex::decode("3d709d64e3b668ddc615a5b05d6f109275096d27571d99ba02d28e84feac6b00")
                    .unwrap()
                    .as_slice(),
            )],
        }]);
        let out = "63667802f85d01010101940123456789012345678901234567890123456789010101018568656c6c6ff838f7940123456789012345678901234567890123456789e1a03d709d64e3b668ddc615a5b05d6f109275096d27571d99ba02d28e84feac6b00".from_hex::<Vec<u8>>().unwrap();
        let decode_tx: Transaction = decode(&out).expect("decode should success");
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
