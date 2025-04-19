use sp1_lib::sys_bigint;

// 0xdd268dbcaac550362d98c384c4e576ccc8b1536847b6bbb31023b4c8caee0535 % 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
const OFFSET: [u32; 8] = [
    0x68fa1019, 0x1fa1846d, 0xa5ef917e, 0x6aaba922, 0xbee01556, 0x4c57acaa, 0x25fecf8f, 0x1b9553f1,
];

const PRIME: [u32; 8] = [
    0x00000163, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000100, 0x00000000, 0x00000000,
];

const MODULUS: [u32; 8] = [
    0xd87cfd47, 0x3c208c16, 0x6871ca8d, 0x97816a91, 0x8181585d, 0xb85045b6, 0xe131a029, 0x30644e72,
];

// Modified FnvHasher that make it faster by hashing 32 bits at a time
pub struct FnvHasher {
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
            hash: OFFSET,
            shift: 0,
        }
    }

    pub fn next_single(&mut self, data: u8) {
        if self.shift >= 32 {
            unsafe {
                sys_bigint(&mut self.hash, 0, &self.hash, &PRIME, &MODULUS);
            }
            self.shift = 0;
        }

        self.hash[0] ^= (data as u32) << self.shift;
        self.shift += 1;
    }

    pub fn next(&mut self, data: &[u8]) {
        for &byte in data {
            self.next_single(byte);
        }
    }

    pub fn get(&self) -> [u32; 8] {
        self.hash
    }
}
