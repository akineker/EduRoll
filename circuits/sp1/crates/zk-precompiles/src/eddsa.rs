use crate::babyjubjub::{scalar_below_suborder, Point};
use crate::fr::Fr;
use crate::poseidon::poseidon5;

pub fn verify_eddsa(
    msg: &Fr,
    pk_x: &Fr,
    pk_y: &Fr,
    r8_x: &Fr,
    r8_y: &Fr,
    s_be: &[u8; 32],
) -> bool {
    if !scalar_below_suborder(s_be) {
        return false;
    }
    if !Point::is_on_curve(pk_x, pk_y) || !Point::is_on_curve(r8_x, r8_y) {
        return false;
    }

    let a = Point::from_affine(*pk_x, *pk_y);
    let r8 = Point::from_affine(*r8_x, *r8_y);

    let hm = poseidon5(r8_x, r8_y, pk_x, pk_y, msg);

    let lhs = Point::base8().scalar_mul_be(s_be);

    let hm_a = a.scalar_mul_be(&hm.to_be_bytes());
    let hm_a_8 = hm_a.double().double().double();
    let rhs = r8.add(&hm_a_8);

    lhs.eq(&rhs)
}
