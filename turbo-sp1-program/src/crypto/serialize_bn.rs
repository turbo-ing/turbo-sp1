use substrate_bn::{AffineG1, Fq, Fr, G1};

// BN254 missing functions

pub fn bn254_g1_one() -> G1 {
    G1::new(Fq::one(), Fq::from_str("2").unwrap(), Fq::one())
}

// BN254 serialization

pub fn bn254_export_g1(point: &G1) -> [u8; 64] {
    let mut bytes: [u8; 64] = [0u8; 64];
    if let Some(affine) = AffineG1::from_jacobian(*point) {
        affine.x().to_big_endian(&mut bytes[0..32]).unwrap();
        affine.y().to_big_endian(&mut bytes[32..64]).unwrap();
    } else {
        panic!("point is at infinity");
    }
    bytes
}

pub fn bn254_export_g1_u32(point: &G1) -> [u32; 16] {
    let bytes = bn254_export_g1(point);
    let mut result = [0u32; 16];
    for i in 0..16 {
        result[i] = u32::from_be_bytes(bytes[i * 4..(i + 1) * 4].try_into().unwrap());
    }
    result
}
