use crypto_bigint::{Uint, U256};

const FNV_OFFSET_BASIS: U256 =
    Uint::from_be_hex("dd268dbcaac550362d98c384c4e576ccc8b1536847b6bbb31023b4c8caee0535");

const FNV_PRIME: U256 =
    Uint::from_be_hex("0000000000000000000001000000000000000000000000000000000000000163");

/// Computes the 256-bit FNV-1a hash for a given byte slice.
///
/// # Arguments
///
/// * `data` - A slice of bytes to hash.
///
/// # Returns
///
/// * A `U256` that represents the 256-bit hash value.
pub fn fnv1a_256(data: &[u8]) -> U256 {
    let mut hash = FNV_OFFSET_BASIS;
    for &byte in data {
        // XOR the hash with the current byte, converting the byte into U256.
        hash ^= Uint::from_u8(byte);
        // Multiply with the FNV prime. (Arithmetic here wraps modulo 2^256.)
        hash = hash.wrapping_mul(&FNV_PRIME);
    }
    hash
}
