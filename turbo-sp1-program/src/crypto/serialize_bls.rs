use bls12_381::{G1Affine, G1Projective, G2Affine, G2Projective, Scalar};

// BLS12-381 serialization

pub fn bls12_381_public_key(private_key: Scalar) -> G2Projective {
    G2Projective::generator() * private_key
}

pub fn bls12_381_export_g1(point: &G1Projective) -> [u8; 48] {
    G1Affine::from(point).to_compressed()
}

pub fn bls12_381_import_g1(bytes: &[u8; 48]) -> G1Projective {
    let affine = G1Affine::from_compressed(bytes).unwrap();
    G1Projective::from(affine)
}

pub fn bls12_381_export_g2(point: &G2Projective) -> [u8; 96] {
    G2Affine::from(point).to_compressed()
}

pub fn bls12_381_import_g2(bytes: &[u8; 96]) -> G2Projective {
    let affine = G2Affine::from_compressed(bytes).unwrap();
    G2Projective::from(affine)
}
