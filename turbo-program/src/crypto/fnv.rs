#[cfg(not(target_os = "zkvm"))]
use crypto_bigint::U256;

#[cfg(target_os = "zkvm")]
use sp1_lib::sys_bigint;

// dd268dbcaac550362d98c384c4e576ccc8b1536847b6bbb31023b4c8caee0535
#[cfg(target_os = "zkvm")]
const OFFSET: [u32; 8] = [
    0xcaee0535, 0x1023b4c8, 0x47b6bbb3, 0xc8b15368, 0xc4e576cc, 0x2d98c384, 0xaac55036, 0xdd268dbc,
];

#[cfg(target_os = "zkvm")]
const PRIME: [u32; 8] = [
    0x00000163, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000100, 0x00000000, 0x00000000,
];

#[cfg(target_os = "zkvm")]
const MODULUS: [u32; 8] = [0, 0, 0, 0, 0, 0, 0, 0];

#[cfg(not(target_os = "zkvm"))]
const OFFSET_U256: U256 =
    U256::from_be_hex("dd268dbcaac550362d98c384c4e576ccc8b1536847b6bbb31023b4c8caee0535");

#[cfg(not(target_os = "zkvm"))]
const PRIME_U256: U256 =
    U256::from_be_hex("0000000000000000000001000000000000000000000000000000000000000163");

// Modified FnvHasher that make it faster by hashing 32 bits at a time
pub struct FnvHasher {
    #[cfg(not(target_os = "zkvm"))]
    hash: U256,

    #[cfg(target_os = "zkvm")]
    hash: [u32; 8],

    shift: u32,
}

impl Default for FnvHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl FnvHasher {
    pub fn new() -> Self {
        Self {
            #[cfg(not(target_os = "zkvm"))]
            hash: OFFSET_U256,

            #[cfg(target_os = "zkvm")]
            hash: OFFSET,

            shift: 0,
        }
    }

    pub fn next_single(&mut self, data: u8) {
        #[cfg(not(target_os = "zkvm"))]
        {
            if self.shift >= 32 {
                self.hash = self.hash.wrapping_mul(&PRIME_U256);
                self.shift = 0;
            }
            self.hash = self.hash.wrapping_xor(&U256::from(data << self.shift));
            self.shift += 1;
        }

        #[cfg(target_os = "zkvm")]
        {
            if self.shift >= 32 {
                unsafe {
                    sys_bigint(&mut self.hash, 0, &self.hash, &PRIME, &MODULUS);
                }
                self.shift = 0;
            }

            self.hash[0] ^= (data as u32) << self.shift;
            self.shift += 1;
        }

        println!("hash: {:?}", self.hash);
    }

    pub fn next(&mut self, data: &[u8]) {
        for &byte in data {
            self.next_single(byte);
        }
    }

    pub fn get(&self) -> [u32; 8] {
        #[cfg(not(target_os = "zkvm"))]
        {
            let words = self.hash.to_words();
            let mut result = [0u32; 8];
            for i in 0..4 {
                let bytes = words[i].to_le_bytes();
                result[i * 2] = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
                result[i * 2 + 1] = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
            }
            result
        }

        #[cfg(target_os = "zkvm")]
        self.hash
    }
}
