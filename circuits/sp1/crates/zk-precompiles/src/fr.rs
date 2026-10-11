use crate::poseidon_constants::MODULUS;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Fr(pub [u64; 4]);

impl Fr {
    pub const ZERO: Fr = Fr([0, 0, 0, 0]);
    pub const ONE: Fr = Fr([1, 0, 0, 0]);

    #[inline]
    pub const fn modulus() -> [u64; 4] {
        MODULUS
    }

    pub fn from_u64(v: u64) -> Fr {
        Fr([v, 0, 0, 0])
    }

    pub fn from_u128(v: u128) -> Fr {
        Fr([v as u64, (v >> 64) as u64, 0, 0])
    }

    pub fn from_be_bytes_mod_order(bytes: &[u8; 32]) -> Fr {
        let mut limbs = [0u64; 4];
        for i in 0..4 {

            limbs[3 - i] = u64::from_be_bytes(bytes[i * 8..(i + 1) * 8].try_into().unwrap());
        }
        while geq(&limbs, &MODULUS) {
            limbs = sub_limbs(&limbs, &MODULUS);
        }
        Fr(limbs)
    }

    pub fn to_be_bytes(&self) -> [u8; 32] {
        let mut out = [0u8; 32];
        for i in 0..4 {
            out[i * 8..(i + 1) * 8].copy_from_slice(&self.0[3 - i].to_be_bytes());
        }
        out
    }

    pub fn is_zero(&self) -> bool {
        self.0 == [0, 0, 0, 0]
    }

    pub fn add(&self, other: &Fr) -> Fr {
        let mut sum = add_limbs(&self.0, &other.0);
        if geq(&sum, &MODULUS) {
            sum = sub_limbs(&sum, &MODULUS);
        }
        Fr(sum)
    }

    pub fn sub(&self, other: &Fr) -> Fr {
        if geq(&self.0, &other.0) {
            Fr(sub_limbs(&self.0, &other.0))
        } else {
            let p_minus_b = sub_limbs(&MODULUS, &other.0);
            Fr(add_limbs(&self.0, &p_minus_b))
        }
    }

    pub fn mul(&self, other: &Fr) -> Fr {
        mulmod(&self.0, &other.0)
    }

    pub fn square(&self) -> Fr {
        self.mul(self)
    }

    pub fn pow5(&self) -> Fr {
        let x2 = self.square();
        let x4 = x2.square();
        x4.mul(self)
    }

    pub fn double(&self) -> Fr {
        self.add(self)
    }
}

#[inline]
fn add_limbs(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let mut out = [0u64; 4];
    let mut carry = 0u64;
    for i in 0..4 {
        let (s1, c1) = a[i].overflowing_add(b[i]);
        let (s2, c2) = s1.overflowing_add(carry);
        out[i] = s2;
        carry = (c1 as u64) + (c2 as u64);
    }
    out
}

#[inline]
fn sub_limbs(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let mut out = [0u64; 4];
    let mut borrow = 0u64;
    for i in 0..4 {
        let (d1, b1) = a[i].overflowing_sub(b[i]);
        let (d2, b2) = d1.overflowing_sub(borrow);
        out[i] = d2;
        borrow = (b1 as u64) + (b2 as u64);
    }
    out
}

#[inline]
pub(crate) fn geq(a: &[u64; 4], b: &[u64; 4]) -> bool {
    for i in (0..4).rev() {
        if a[i] > b[i] {
            return true;
        }
        if a[i] < b[i] {
            return false;
        }
    }
    true
}


#[cfg(target_os = "zkvm")]
fn mulmod(a: &[u64; 4], b: &[u64; 4]) -> Fr {
    let mut result = [0u64; 4];
    unsafe {
        sp1_zkvm::syscalls::sys_bigint(
            &mut result as *mut [u64; 4],
            0,
            a as *const [u64; 4],
            b as *const [u64; 4],
            &MODULUS as *const [u64; 4],
        );
    }
    Fr(result)
}

#[cfg(not(target_os = "zkvm"))]
fn mulmod(a: &[u64; 4], b: &[u64; 4]) -> Fr {
    use num_bigint::BigUint;
    fn to_big(l: &[u64; 4]) -> BigUint {
        let mut bytes = [0u8; 32];
        for i in 0..4 {
            bytes[i * 8..(i + 1) * 8].copy_from_slice(&l[i].to_le_bytes());
        }
        BigUint::from_bytes_le(&bytes)
    }
    let p = to_big(&MODULUS);
    let r = (to_big(a) * to_big(b)) % p;
    let mut bytes = r.to_bytes_le();
    bytes.resize(32, 0);
    let mut limbs = [0u64; 4];
    for i in 0..4 {
        limbs[i] = u64::from_le_bytes(bytes[i * 8..(i + 1) * 8].try_into().unwrap());
    }
    Fr(limbs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr as ArkFr;
    use ark_ff::{BigInteger, PrimeField};

    fn ark_to_fr(x: ArkFr) -> Fr {
        let mut be = x.into_bigint().to_bytes_be();
        while be.len() < 32 {
            be.insert(0, 0);
        }
        Fr::from_be_bytes_mod_order(&be.try_into().unwrap())
    }

    #[test]
    fn mul_matches_arkworks() {
        let a = ArkFr::from(1234567890123456789u64);
        let b = ArkFr::from(987654321987654321u64);
        let ours = Fr::from_u64(1234567890123456789).mul(&Fr::from_u64(987654321987654321));
        assert_eq!(ours, ark_to_fr(a * b));
    }

    #[test]
    fn add_sub_roundtrip() {
        let a = Fr::from_u128(u128::MAX);
        let b = Fr::from_u64(42);
        assert_eq!(a.add(&b).sub(&b), a);
        assert_eq!(Fr::ZERO.sub(&b).add(&b), Fr::ZERO);
    }

    #[test]
    fn pow5_matches_arkworks() {
        let x = ArkFr::from(31337u64);
        let expected = x * x * x * x * x;
        assert_eq!(Fr::from_u64(31337).pow5(), ark_to_fr(expected));
    }

    #[test]
    fn be_bytes_roundtrip() {
        let x = Fr::from_u128(0xdead_beef_cafe_babe_1234_5678_9abc_def0);
        assert_eq!(Fr::from_be_bytes_mod_order(&x.to_be_bytes()), x);
    }

    #[test]
    fn reduces_oversized_be_input() {
        let big = [0xffu8; 32];
        let ours = Fr::from_be_bytes_mod_order(&big);
        let theirs = ArkFr::from_be_bytes_mod_order(&big);
        assert_eq!(ours, ark_to_fr(theirs));
    }
}
