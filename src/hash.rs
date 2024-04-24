use sha2::{Digest, Sha256};
use serde::{Serialize, Deserialize};

// let hash = Sha256::digest(b"my message");

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Hash {
    value: [u8; Hash::SIZE],
}

impl Hash {
    const SIZE: usize = 32;

    pub fn new(data: &[u8]) -> Hash {
        Hash {
            value: Sha256::digest(data).as_slice().try_into().unwrap(),
        }
    }

    pub fn digest(&self) -> [u8; Hash::SIZE] {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest() {
        assert_eq!(
            &Hash::new(b"test").digest(),
            &[
                159, 134, 208, 129, 136, 76, 125, 101, 154, 47, 234, 160, 197, 90, 208, 21, 163,
                191, 79, 27, 43, 11, 130, 44, 209, 93, 108, 21, 176, 240, 10, 8
            ]
        );
    }
}
