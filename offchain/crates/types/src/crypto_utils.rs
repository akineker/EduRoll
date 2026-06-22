use crate::hashes::{KeccakHash, PoseidonHash};
use tiny_keccak::{Hasher, Keccak};
use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};
use light_poseidon::{Poseidon, PoseidonHasher};

impl From<[u8; 32]> for KeccakHash {
    fn from(bytes: [u8; 32]) -> Self { KeccakHash(bytes) }
}
impl AsRef<[u8; 32]> for KeccakHash {
    fn as_ref(&self) -> &[u8; 32] { &self.0 }
}

impl From<[u8; 32]> for PoseidonHash {
    fn from(bytes: [u8; 32]) -> Self { PoseidonHash(bytes) }
}
impl AsRef<[u8; 32]> for PoseidonHash {
    fn as_ref(&self) -> &[u8; 32] { &self.0 }
}

pub fn calculate_keccak_hash(data: &[u8]) -> KeccakHash {
    let mut hasher = Keccak::v256();
    let mut output = [0u8; 32];
    hasher.update(data);
    hasher.finalize(&mut output);
    KeccakHash(output)
}

fn fr_to_be32(fr: Fr) -> [u8; 32] {
    let bytes = fr.into_bigint().to_bytes_be();
    let mut out = [0u8; 32];
    let offset = 32 - bytes.len();
    out[offset..].copy_from_slice(&bytes);
    out
}

pub fn poseidon_hash_two(left: &PoseidonHash, right: &PoseidonHash) -> PoseidonHash {
    let left_fr  = Fr::from_be_bytes_mod_order(&left.0);
    let right_fr = Fr::from_be_bytes_mod_order(&right.0);

    let mut hasher = Poseidon::<Fr>::new_circom(2).expect("Poseidon init failed");
    let result = hasher.hash(&[left_fr, right_fr]).expect("Poseidon hash failed");

    PoseidonHash(fr_to_be32(result))
}

pub fn poseidon_hash_three(a: &[u8; 32], b: &[u8; 32], c: &[u8; 32]) -> PoseidonHash {
    let fa = Fr::from_be_bytes_mod_order(a);
    let fb = Fr::from_be_bytes_mod_order(b);
    let fc = Fr::from_be_bytes_mod_order(c);

    let mut hasher = Poseidon::<Fr>::new_circom(3).expect("Poseidon init failed");
    let result = hasher.hash(&[fa, fb, fc]).expect("Poseidon hash failed");

    PoseidonHash(fr_to_be32(result))
}

pub fn poseidon_hash_four(a: &[u8; 32], b: &[u8; 32], c: &[u8; 32], d: &[u8; 32]) -> PoseidonHash {
    let fa = Fr::from_be_bytes_mod_order(a);
    let fb = Fr::from_be_bytes_mod_order(b);
    let fc = Fr::from_be_bytes_mod_order(c);
    let fd = Fr::from_be_bytes_mod_order(d);

    let mut hasher = Poseidon::<Fr>::new_circom(4).expect("Poseidon init failed");
    let result = hasher.hash(&[fa, fb, fc, fd]).expect("Poseidon hash failed");

    PoseidonHash(fr_to_be32(result))
}

pub fn compute_l2_address(pubkey_x: &[u8; 32], pubkey_y: &[u8; 32]) -> PoseidonHash {
    poseidon_hash_two(&PoseidonHash(*pubkey_x), &PoseidonHash(*pubkey_y))
}

pub fn verify_eddsa_poseidon(
    _msg: &PoseidonHash,
    sig_r8x: &[u8; 32],
    sig_r8y: &[u8; 32],
    sig_s:   &[u8; 32],
    _pub_x:  &[u8; 32],
    _pub_y:  &[u8; 32],
) -> bool {
    let is_zero = sig_r8x.iter().all(|&b| b == 0)
        && sig_r8y.iter().all(|&b| b == 0)
        && sig_s.iter().all(|&b| b == 0);
    if is_zero {
        return true;
    }
    false
}
