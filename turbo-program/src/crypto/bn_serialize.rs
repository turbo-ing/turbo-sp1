use std::mem::transmute;

use substrate_bn::{AffineG1, Fq, G1};

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

pub fn bn254_export_affine_g1(affine: &AffineG1) -> [u8; 64] {
    let mut bytes: [u8; 64] = [0u8; 64];
    affine.x().to_big_endian(&mut bytes[0..32]).unwrap();
    affine.y().to_big_endian(&mut bytes[32..64]).unwrap();
    bytes
}

pub fn bn254_export_affine_g1_memcpy(affine: &AffineG1) -> [u32; 16] {
    let mut result = [0u32; 16];

    unsafe {
        core::ptr::copy_nonoverlapping(transmute(affine), result.as_mut_ptr(), 16);
    }

    result
}

pub fn bn254_import_affine_g1(bytes: &[u8; 64]) -> AffineG1 {
    let x = Fq::from_slice(&bytes[0..32]).unwrap();
    let y = Fq::from_slice(&bytes[32..64]).unwrap();

    AffineG1::new(x, y).unwrap()
}

pub fn bn254_import_affine_g1_memcpy(data: &[u32; 16]) -> AffineG1 {
    let mut affine = AffineG1::zero();
    unsafe {
        core::ptr::copy_nonoverlapping(data.as_ptr(), transmute(&mut affine), 16);
    }
    affine
}
