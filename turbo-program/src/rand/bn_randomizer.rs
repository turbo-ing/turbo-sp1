use sp1_lib::{syscall_bn254_add, syscall_bn254_double};
use substrate_bn::*;

use crate::crypto::bn_math::{bn254_add, bn254_double};
use crate::crypto::bn_serialize::{bn254_export_affine_g1_memcpy, bn254_import_affine_g1_memcpy};
use crate::rand::pcg::{rxs_m_xs, xsh_rs};

#[derive(Clone)]
pub struct BnRandomizer {
    current: [u32; 16],
    nonce: u64,

    #[cfg(not(target_os = "zkvm"))]
    current_g1: AffineG1,
}

impl Default for BnRandomizer {
    fn default() -> Self {
        Self::new()
    }
}

impl BnRandomizer {
    pub fn new() -> Self {
        Self {
            current: bn254_export_affine_g1_memcpy(&AffineG1::one()),
            nonce: 0,

            #[cfg(not(target_os = "zkvm"))]
            current_g1: AffineG1::one(),
        }
    }

    pub fn new_with_seed(seed: &[u32; 16]) -> Self {
        #[cfg(not(target_os = "zkvm"))]
        let current_g1 = bn254_import_affine_g1_memcpy(seed);

        Self {
            current: *seed,
            nonce: 0,

            #[cfg(not(target_os = "zkvm"))]
            current_g1,
        }
    }

    pub fn new_with_seeds(seeds: Vec<[u32; 16]>) -> Self {
        #[cfg(target_os = "zkvm")]
        let mut seed: [u32; 16] = seeds[0];

        #[cfg(not(target_os = "zkvm"))]
        let mut seed = bn254_import_affine_g1_memcpy(&seeds[0]);

        for s in seeds.as_slice()[1..].iter() {
            #[cfg(target_os = "zkvm")]
            unsafe {
                syscall_bn254_add(&mut seed, s);
            }

            #[cfg(not(target_os = "zkvm"))]
            {
                let ss = bn254_import_affine_g1_memcpy(s);
                seed = bn254_add(seed, ss);
            }
        }

        Self {
            #[cfg(target_os = "zkvm")]
            current: seed,

            #[cfg(not(target_os = "zkvm"))]
            current: bn254_export_affine_g1_memcpy(&seed),

            nonce: 0,

            #[cfg(not(target_os = "zkvm"))]
            current_g1: seed,
        }
    }

    fn next_rand(&mut self) {
        if self.nonce % 2 == 0 {
            #[cfg(target_os = "zkvm")]
            unsafe {
                syscall_bn254_double(&mut self.current);
            }

            #[cfg(not(target_os = "zkvm"))]
            {
                self.current_g1 = bn254_double(self.current_g1);
                self.current = bn254_export_affine_g1_memcpy(&self.current_g1);
            }
        }
        self.nonce += 1;
    }

    pub fn next_u32(&mut self) -> u32 {
        self.next_rand();

        match self.nonce % 2 {
            0 => xsh_rs((self.current[7] as u64) << 32 | self.current[6] as u64),
            1 => xsh_rs((self.current[4] as u64) << 32 | self.current[5] as u64),
            _ => unreachable!(),
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.next_rand();

        match self.nonce % 2 {
            0 => rxs_m_xs((self.current[7] as u64) << 32 | self.current[6] as u64),
            1 => rxs_m_xs((self.current[4] as u64) << 32 | self.current[5] as u64),
            _ => unreachable!(),
        }
    }

    pub fn current_seed(&self) -> [u32; 16] {
        self.current
    }
}
