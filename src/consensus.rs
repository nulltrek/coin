use crate::types::hash::Hash;
use ethnum::U256;

struct ConsensusRules {
    target: U256,
}

impl Default for ConsensusRules {
    fn default() -> ConsensusRules {
        ConsensusRules { target: U256::MAX }
    }
}

impl ConsensusRules {
    fn new(target: U256) -> ConsensusRules {
        ConsensusRules { target }
    }

    fn validate_target(&self, hash: &Hash) -> bool {
        U256::from_be_bytes(hash.digest().clone()) < self.target
    }
}

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
