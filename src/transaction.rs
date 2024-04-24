use crate::keys::{Signature, KeyPair, PublicKey};
use crate::hash::Hash;
use serde::{Serialize, Deserialize};
use bincode;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InPoint {
    hash: Hash,
    index: u32,
    signature: Signature,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OutPoint {
    value: u64,
    pubkey: PublicKey,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransactionData {
    inputs: Vec<InPoint>,
    outputs: Vec<OutPoint>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Transaction {
    pub hash: Hash,
    pub data: TransactionData,
}

impl Transaction {
    pub fn new(tx_data: &TransactionData) -> Transaction {
        let bytes: Vec<u8> = bincode::serialize(&tx_data).unwrap();
        Transaction {
            hash: Hash::new(bytes.as_slice()),
            data: tx_data.clone(),
        }
    }
}

pub struct Utxo {
    hash: Hash,
    output: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashing_equality() {
        let key = KeyPair::new();
        let tx_data_1 = TransactionData {
            inputs: vec!(InPoint{ hash: Hash::new(b"test"), index: 0, signature: key.sign(b"test")}),
            outputs: vec!(OutPoint{value: 1, pubkey: key.public_key() })
        };

        let tx_data_2 = TransactionData {
            inputs: vec!(InPoint{ hash: Hash::new(b"test"), index: 0, signature: key.sign(b"test")}),
            outputs: vec!(OutPoint{value: 1, pubkey: key.public_key() })
        };

        let tx1 = Transaction::new(&tx_data_1);
        let tx2 = Transaction::new(&tx_data_2);

        assert_eq!(tx1.hash, tx2.hash)
    }
}
