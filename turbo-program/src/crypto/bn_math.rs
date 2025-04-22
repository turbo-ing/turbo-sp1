use substrate_bn::*;

pub fn bn254_add(a: AffineG1, b: AffineG1) -> AffineG1 {
    let p: G1 = a.into();
    let q: G1 = b.into();
    AffineG1::from_jacobian(p + q).unwrap()
}

pub fn bn254_double(a: AffineG1) -> AffineG1 {
    bn254_add(a, a)
}
