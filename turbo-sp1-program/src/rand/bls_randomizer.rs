use bls12_381::G1Projective;

use crate::rand::pcg::{xsh_rs, xsl_rr_from_parts};
pub struct BlsRandomizer {
    current: G1Projective,
    nonce: u64,
}

impl Default for BlsRandomizer {
    fn default() -> Self {
        Self::new()
    }
}

impl BlsRandomizer {
    pub fn new() -> Self {
        Self {
            current: G1Projective::identity(),
            nonce: 0,
        }
    }

    pub fn new_with_seed(seed: G1Projective) -> Self {
        Self {
            current: seed,
            nonce: 0,
        }
    }

    fn next_rand(&mut self) -> G1Projective {
        self.nonce += 1;
        if self.nonce % 2 == 1 {
            self.current = self.current + self.current;
        }
        self.current
    }

    pub fn next_rand_u32(&mut self) -> u32 {
        let rand = self.next_rand();
        let x = rand.x.to_bytes();

        if self.nonce % 2 == 1 {
            let result = u64::from_le_bytes(x[0..8].try_into().unwrap());
            xsh_rs(result)
        } else {
            let result = u64::from_le_bytes(x[16..24].try_into().unwrap());
            xsh_rs(result)
        }
    }

    pub fn next_rand_u64(&mut self) -> u64 {
        let rand = self.next_rand();
        let x = rand.x.to_bytes();

        if self.nonce % 2 == 1 {
            let hi = u64::from_le_bytes(x[0..8].try_into().unwrap());
            let lo = u64::from_le_bytes(x[8..16].try_into().unwrap());
            xsl_rr_from_parts(hi, lo)
        } else {
            let hi = u64::from_le_bytes(x[16..24].try_into().unwrap());
            let lo = u64::from_le_bytes(x[24..32].try_into().unwrap());
            xsl_rr_from_parts(hi, lo)
        }
    }
}
