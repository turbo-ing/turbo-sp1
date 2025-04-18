use sp1_lib::{syscall_bn254_add, syscall_bn254_double};
use substrate_bn::*;

use crate::{
    crypto::serialize_bn::{bn254_export_g1_u32, bn254_import_affine_g1_u32},
    rand::pcg::xsh_rs,
};
pub struct BnRandomizer {
    current: AffineG1,
    nonce: u64,
}

impl Default for BnRandomizer {
    fn default() -> Self {
        Self::new()
    }
}

impl BnRandomizer {
    pub fn new() -> Self {
        Self {
            current: AffineG1::one(),
            nonce: 0,
        }
    }

    pub fn new_with_seed(seed: &[u32; 16]) -> Self {
        Self {
            current: bn254_import_affine_g1_u32(seed),
            nonce: 0,
        }
    }

    pub fn new_with_seeds(seeds: Vec<[u32; 16]>) -> Self {
        // let mut seed: [u32; 16] = [1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0];
        // for s in seeds {
        //     unsafe {
        //         syscall_bn254_add(&mut seed, &s);
        //     }
        // }

        Self {
            current: bn254_import_affine_g1_u32(&seeds[0]),
            nonce: 0,
        }
    }

    fn next_rand(&mut self) {
        if self.nonce % 4 == 0 {
            self.current = self.current + self.current;
        }
        self.nonce += 1;
    }

    pub fn next_rand_u32(&mut self) -> u32 {
        self.next_rand();

        return 0;

        // match self.nonce % 4 {
        //     0 => xsh_rs((self.current[7] as u64) << 32 | self.current[6] as u64),
        //     1 => xsh_rs((self.current[6] as u64) << 32 | self.current[5] as u64),
        //     2 => xsh_rs((self.current[5] as u64) << 32 | self.current[4] as u64),
        //     3 => xsh_rs((self.current[4] as u64) << 32 | self.current[3] as u64),
        //     _ => unreachable!(),
        // }
    }
}
