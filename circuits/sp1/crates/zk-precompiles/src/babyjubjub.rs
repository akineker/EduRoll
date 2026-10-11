use crate::fr::{geq, Fr};
use crate::poseidon_constants::{BASE8_X, BASE8_Y, SUBORDER};

fn coeff_a() -> Fr {
    Fr::from_u64(168700)
}

fn coeff_d() -> Fr {
    Fr::from_u64(168696)
}

#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub x: Fr,
    pub y: Fr,
    pub z: Fr,
    pub t: Fr,
}

impl Point {
    pub fn identity() -> Point {
        Point {
            x: Fr::ZERO,
            y: Fr::ONE,
            z: Fr::ONE,
            t: Fr::ZERO,
        }
    }

    pub fn from_affine(x: Fr, y: Fr) -> Point {
        Point {
            x,
            y,
            z: Fr::ONE,
            t: x.mul(&y),
        }
    }

    pub fn base8() -> Point {
        Point::from_affine(Fr(BASE8_X), Fr(BASE8_Y))
    }

    pub fn is_on_curve(x: &Fr, y: &Fr) -> bool {
        let x2 = x.square();
        let y2 = y.square();
        let lhs = coeff_a().mul(&x2).add(&y2);
        let rhs = Fr::ONE.add(&coeff_d().mul(&x2).mul(&y2));
        lhs == rhs
    }

    pub fn add(&self, other: &Point) -> Point {
        let a = self.x.mul(&other.x);
        let b = self.y.mul(&other.y);
        let c = coeff_d().mul(&self.t).mul(&other.t);
        let d = self.z.mul(&other.z);
        let e = self
            .x
            .add(&self.y)
            .mul(&other.x.add(&other.y))
            .sub(&a)
            .sub(&b);
        let f = d.sub(&c); // F = D − C
        let g = d.add(&c); // G = D + C
        let h = b.sub(&coeff_a().mul(&a)); // H = B − a·A
        Point {
            x: e.mul(&f),
            y: g.mul(&h),
            t: e.mul(&h),
            z: f.mul(&g),
        }
    }

    pub fn double(&self) -> Point {
        let a = self.x.square();
        let b = self.y.square();
        let c = self.z.square().double();
        let d = coeff_a().mul(&a);
        let e = self.x.add(&self.y).square().sub(&a).sub(&b);
        let g = d.add(&b); // G = D + B
        let f = g.sub(&c); // F = G − C
        let h = d.sub(&b); // H = D − B
        Point {
            x: e.mul(&f),
            y: g.mul(&h),
            t: e.mul(&h),
            z: f.mul(&g),
        }
    }

    pub fn scalar_mul_be(&self, scalar_be: &[u8; 32]) -> Point {
        let mut acc = Point::identity();
        let mut started = false;
        for byte in scalar_be.iter() {
            for bit in (0..8).rev() {
                if started {
                    acc = acc.double();
                }
                if (byte >> bit) & 1 == 1 {
                    if started {
                        acc = acc.add(self);
                    } else {
                        acc = *self;
                        started = true;
                    }
                }
            }
        }
        acc
    }

    pub fn eq(&self, other: &Point) -> bool {
        self.x.mul(&other.z) == other.x.mul(&self.z) && self.y.mul(&other.z) == other.y.mul(&self.z)
    }
}

pub fn scalar_below_suborder(s_be: &[u8; 32]) -> bool {
    let mut limbs = [0u64; 4];
    for i in 0..4 {
        limbs[3 - i] = u64::from_be_bytes(s_be[i * 8..(i + 1) * 8].try_into().unwrap());
    }
    !geq(&limbs, &SUBORDER)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base8_is_on_curve() {
        assert!(Point::is_on_curve(&Fr(BASE8_X), &Fr(BASE8_Y)));
    }

    #[test]
    fn double_matches_add() {
        let b8 = Point::base8();
        assert!(b8.double().eq(&b8.add(&b8)));
    }

    #[test]
    fn scalar_mul_small_values() {
        let b8 = Point::base8();
        let mut three = [0u8; 32];
        three[31] = 3;
        let lhs = b8.scalar_mul_be(&three);
        let rhs = b8.add(&b8).add(&b8);
        assert!(lhs.eq(&rhs));
    }

    #[test]
    fn scalar_mul_by_suborder_is_identity() {
        let b8 = Point::base8();
        let mut l_be = [0u8; 32];
        for i in 0..4 {
            l_be[i * 8..(i + 1) * 8].copy_from_slice(&SUBORDER[3 - i].to_be_bytes());
        }
        let res = b8.scalar_mul_be(&l_be);
        assert!(res.eq(&Point::identity()));
    }

    #[test]
    fn suborder_bound_check() {
        let mut l_be = [0u8; 32];
        for i in 0..4 {
            l_be[i * 8..(i + 1) * 8].copy_from_slice(&SUBORDER[3 - i].to_be_bytes());
        }
        assert!(!scalar_below_suborder(&l_be)); // s == l rejected
        let mut l_minus_1 = l_be;
        l_minus_1[31] -= 1; // l ends in ...f1, no borrow
        assert!(scalar_below_suborder(&l_minus_1));
        assert!(scalar_below_suborder(&[0u8; 32]));
    }
}
