use bls12_381::{fp::Fp, hash_to_curve::MapToCurve, G1Projective};

pub fn hashed_to_curve(limbs: [u8; 32]) -> G1Projective {
    let mut padded_limbs = [0u8; 48];
    padded_limbs[..32].copy_from_slice(&limbs);
    let fp = Fp::from_bytes(&padded_limbs).unwrap();
    G1Projective::map_to_curve(&fp)
}
