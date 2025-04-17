use sp1_lib::{syscall_bn254_add, syscall_bn254_double};
use substrate_bn::{AffineG1, Fq, G1};

use crate::{
    crypto::serialize_bn::{bn254_export_g1_u32, bn254_g1_one},
    rand::pcg::{xsh_rs, xsl_rr_from_parts},
};
pub struct BnRandomizer {
    current: [u32; 16],
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
            current: bn254_export_g1_u32(&bn254_g1_one()),
            nonce: 0,
        }
    }

    pub fn new_with_seed(seed: &[u32; 16]) -> Self {
        Self {
            current: *seed,
            nonce: 0,
        }
    }

    pub fn new_with_seeds(seeds: Vec<[u32; 16]>) -> Self {
        println!("try seeds: {:?}", seeds);

        let mut seed = AffineG1::from_jacobian(G1::new(
            Fq::from_str(
                &seeds[0][0..8]
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(""),
            )
            .unwrap(),
            Fq::from_str(
                &seeds[0][8..16]
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(""),
            )
            .unwrap(),
            Fq::one(),
        ))
        .unwrap();

        for i in 1..seeds.len() {
            let next_point = AffineG1::from_jacobian(G1::new(
                Fq::from_str(
                    &seeds[i][0..8]
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join(""),
                )
                .unwrap(),
                Fq::from_str(
                    &seeds[i][8..16]
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join(""),
                )
                .unwrap(),
                Fq::one(),
            ))
            .unwrap();
            seed = seed + next_point;
        }

        let seed_bytes = seeds[0];
        println!("seed: {:?}", seed_bytes);

        Self {
            current: seed_bytes,
            nonce: 0,
        }
    }

    fn next_rand(&mut self) {
        self.nonce += 1;
        if self.nonce % 2 == 1 {
            unsafe {
                syscall_bn254_double(&mut self.current);
            }
        }
    }

    pub fn next_rand_u32(&mut self) -> u32 {
        self.next_rand();

        if self.nonce % 2 == 1 {
            let result: u64 = (self.current[7] as u64) << 32 | self.current[6] as u64;
            xsh_rs(result)
        } else {
            let result: u64 = (self.current[5] as u64) << 32 | self.current[4] as u64;
            xsh_rs(result)
        }
    }
}
