use crate::keys::{Signature, PublicKey};
use crate::hash::Hash;
use serde::{Serialize, Deserialize};
use bincode;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InPoint {
    pub hash: Hash,
    pub index: u32,
    pub signature: Signature,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OutPoint {
    pub value: u64,
    pub pubkey: PublicKey,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransactionData {
    pub inputs: Vec<InPoint>,
    pub outputs: Vec<OutPoint>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

    pub fn is_valid(&self) -> bool {
        let bytes: Vec<u8> = bincode::serialize(&self.data).unwrap();
        return Hash::new(bytes.as_slice()).digest() == self.hash.digest();
    }
}

pub struct Utxo {
    hash: Hash,
    output: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::KeyPair;

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

    #[test]
    fn validation() {
        let key = KeyPair::new();
        let tx_data = TransactionData {
            inputs: vec!(InPoint{ hash: Hash::new(b"test"), index: 0, signature: key.sign(b"test")}),
            outputs: vec!(OutPoint{value: 1, pubkey: key.public_key() })
        };

        let tx_1 = Transaction::new(&tx_data);
        assert!(tx_1.is_valid());

        let tx_2 = Transaction {
            hash: Hash::new(b"test"),
            data: tx_data.clone(),
        };
        assert!(!tx_2.is_valid());
    }

    #[test]
    fn unserialize_validation() {
        let key = KeyPair::new();
        let tx_data = TransactionData {
            inputs: vec!(InPoint{ hash: Hash::new(b"test"), index: 0, signature: key.sign(b"test")}),
            outputs: vec!(OutPoint{value: 1, pubkey: key.public_key() })
        };

        let tx = Transaction {
            hash: Hash::new(b"test"),
            data: tx_data.clone(),
        };

        let bytes: Vec<u8> = bincode::serialize(&tx).unwrap();

        let deserialized_tx: Transaction = bincode::deserialize(&bytes).unwrap();
        assert!(!deserialized_tx.is_valid());
    }
}
