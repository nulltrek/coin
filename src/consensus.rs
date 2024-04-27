use crate::traits::io::{ByteIO, FileIO};
use crate::types::hash::Hash;
use ethnum::U256;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConsensusRules {
    target: [u8; 32],
}

impl Default for ConsensusRules {
    fn default() -> ConsensusRules {
        ConsensusRules {
            target: U256::MAX.to_be_bytes(),
        }
    }
}

impl ConsensusRules {
    pub fn new(target: U256) -> ConsensusRules {
        ConsensusRules {
            target: target.to_be_bytes(),
        }
    }

    pub fn validate_target(&self, hash: &Hash) -> bool {
        U256::from_be_bytes(hash.digest().clone()) < U256::from_be_bytes(self.target)
    }
}

impl ByteIO for ConsensusRules {}
impl FileIO for ConsensusRules {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::io::ByteIO;

    #[test]
    fn target_cmp() {
        let hash = Hash::from_bytes(&[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ])
        .unwrap();

        let cr = ConsensusRules::default();
        assert!(cr.validate_target(&hash));

        let hash = Hash::from_bytes(&[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 3,
        ])
        .unwrap();

        let cr = ConsensusRules::new(U256::from(2_u32));
        assert!(!cr.validate_target(&hash));

        let hash = Hash::from_bytes(&[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0,
            0, 0, 0,
        ])
        .unwrap();

        let cr = ConsensusRules::new(U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0,
            0, 0, 0,
        ]));
        assert!(cr.validate_target(&hash));
        let cr = ConsensusRules::new(U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 0, 0,
            0, 0, 0, 0,
        ]));
        assert!(!cr.validate_target(&hash));
    }
}
