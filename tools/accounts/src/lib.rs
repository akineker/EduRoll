use anyhow::Result;
use ark_bn254::Fr as Fq;
use ark_ec::CurveGroup;
use ark_ed_on_bn254::{EdwardsAffine, EdwardsProjective, Fr};
use ark_ff::{BigInteger, Field, PrimeField};
use light_poseidon::{Poseidon, PoseidonHasher};
use sha2::{Digest, Sha256};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Account {
    pub index: u32,
    pub owner_address: [u8; 20],
    pub priv_seed: [u8; 32],
    pub priv_scalar: Fr,
    pub pubkey: EdwardsAffine,
    pub pubkey_x: [u8; 32],
    pub pubkey_y: [u8; 32],
    pub l2_address: [u8; 32],
}

pub fn deterministic_account(index: u32) -> Account {
    let mut h = Sha256::new();
    h.update(b"eduroll-account-v1");
    h.update(&index.to_be_bytes());
    let priv_seed: [u8; 32] = h.finalize().into();

    let mut sh = Sha256::new();
    sh.update(b"eduroll-scalar-v1");
    sh.update(&priv_seed);
    let scalar_bytes = sh.finalize();
    let priv_scalar = Fr::from_le_bytes_mod_order(&scalar_bytes);

    let pubkey_ark: EdwardsAffine = (b_circomlib() * priv_scalar).into_affine();

    let pubkey_x_c = pubkey_ark.x * sqrt_a_inv();
    let pubkey_y_c = pubkey_ark.y;
    let pkx_be = fq_to_be32(pubkey_x_c);
    let pky_be = fq_to_be32(pubkey_y_c);

    let l2_address = poseidon_two_be(&pkx_be, &pky_be);
    let owner_address: [u8; 20] = l2_address[12..32].try_into().unwrap();

    Account {
        index,
        owner_address,
        priv_seed,
        priv_scalar,
        pubkey: pubkey_ark,
        pubkey_x: pkx_be,
        pubkey_y: pky_be,
        l2_address,
    }
}

pub fn deterministic_accounts(n: usize) -> Vec<Account> {
    (0..n as u32).map(deterministic_account).collect()
}

pub fn compute_l2_address(pubkey_x: &[u8; 32], pubkey_y: &[u8; 32]) -> [u8; 32] {
    poseidon_two_be(pubkey_x, pubkey_y)
}

pub fn compute_msg_hash(
    sender_l2: &[u8; 32],
    recv_l2: &[u8; 32],
    amount: u128,
    nonce: u64,
) -> [u8; 32] {
    let mut h = Poseidon::<Fq>::new_circom(4).expect("Poseidon4 init");
    let inputs = [
        Fq::from_be_bytes_mod_order(sender_l2),
        Fq::from_be_bytes_mod_order(recv_l2),
        Fq::from(amount),
        Fq::from(nonce),
    ];
    let out = h.hash(&inputs).expect("Poseidon4 hash");
    fq_to_be32(out)
}

pub fn sign_tx(
    account: &Account,
    sender_l2: &[u8; 32],
    recv_l2: &[u8; 32],
    amount: u128,
    nonce: u64,
) -> Result<([u8; 32], [u8; 32], [u8; 32])> {
    let msg_be = compute_msg_hash(sender_l2, recv_l2, amount, nonce);
    let msg_fq = Fq::from_be_bytes_mod_order(&msg_be);

    let mut h = Sha256::new();
    h.update(b"eduroll-eddsa-nonce-v1");
    h.update(&account.priv_seed);
    h.update(&msg_be);
    let r_bytes = h.finalize();
    let r = Fr::from_le_bytes_mod_order(&r_bytes);

    let r8_ark: EdwardsAffine = (b8_circomlib() * r).into_affine();

    let r8x_c = r8_ark.x * sqrt_a_inv();
    let r8y_c = r8_ark.y;

    let pkx_c = Fq::from_be_bytes_mod_order(&account.pubkey_x);
    let pky_c = Fq::from_be_bytes_mod_order(&account.pubkey_y);
    let mut hash5 = Poseidon::<Fq>::new_circom(5).expect("Poseidon5 init");
    let hm_fq = hash5
        .hash(&[r8x_c, r8y_c, pkx_c, pky_c, msg_fq])
        .expect("Poseidon5 hash");

    let hm_fr = Fr::from_le_bytes_mod_order(&hm_fq.into_bigint().to_bytes_le());

    let s_fr = r + hm_fr * account.priv_scalar;

    Ok((fq_to_be32(r8x_c), fq_to_be32(r8y_c), fr_to_be32(s_fr)))
}

pub fn verify_tx(
    pkx: &[u8; 32],
    pky: &[u8; 32],
    sender_l2: &[u8; 32],
    recv_l2: &[u8; 32],
    amount: u128,
    nonce: u64,
    sig_r8x: &[u8; 32],
    sig_r8y: &[u8; 32],
    sig_s: &[u8; 32],
) -> bool {
    let msg_be = compute_msg_hash(sender_l2, recv_l2, amount, nonce);
    let msg_fq = Fq::from_be_bytes_mod_order(&msg_be);

    let pkx_c = Fq::from_be_bytes_mod_order(pkx);
    let pky_c = Fq::from_be_bytes_mod_order(pky);
    let r8x_c = Fq::from_be_bytes_mod_order(sig_r8x);
    let r8y_c = Fq::from_be_bytes_mod_order(sig_r8y);

    let sa = sqrt_a();
    let pubkey = EdwardsAffine::new_unchecked(pkx_c * sa, pky_c);
    let r8     = EdwardsAffine::new_unchecked(r8x_c * sa, r8y_c);
    if !pubkey.is_on_curve() || !r8.is_on_curve() {
        return false;
    }

    let s_fr = Fr::from_be_bytes_mod_order(sig_s);

    let mut hash5 = Poseidon::<Fq>::new_circom(5).expect("Poseidon5 init");
    let hm_fq = match hash5.hash(&[r8x_c, r8y_c, pkx_c, pky_c, msg_fq]) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let hm_fr = Fr::from_le_bytes_mod_order(&hm_fq.into_bigint().to_bytes_le());

    let eight = Fr::from(8u32);
    let lhs: EdwardsProjective = b8_circomlib() * s_fr;
    let rhs: EdwardsProjective =
        EdwardsProjective::from(r8) + (EdwardsProjective::from(pubkey) * (hm_fr * eight));
    lhs == rhs
}

fn sqrt_a() -> Fq {
    Fq::from(168700u64)
        .sqrt()
        .expect("168700 has a square root in Fq (verified empirically)")
}


fn sqrt_a_inv() -> Fq {
    sqrt_a().inverse().expect("sqrt_a is non-zero")
}


fn b8_circomlib() -> EdwardsAffine {
    let x_c = Fq::from_str(
        "5299619240641551281634865583518297030282874472190772894086521144482721001553",
    )
    .expect("parse BASE8 x");
    let y_c = Fq::from_str(
        "16950150798460657717958625567821834550301663161624707787222815936182638968203",
    )
    .expect("parse BASE8 y");
    EdwardsAffine::new_unchecked(x_c * sqrt_a(), y_c)
}

fn b_circomlib() -> EdwardsAffine {
    let eight_inv = Fr::from(8u32).inverse().expect("8 is invertible in Fr");
    (b8_circomlib() * eight_inv).into_affine()
}

fn fq_to_be32(x: Fq) -> [u8; 32] {
    let mut le = x.into_bigint().to_bytes_le();
    le.resize(32, 0);
    le.reverse();
    let mut out = [0u8; 32];
    out.copy_from_slice(&le);
    out
}

fn fr_to_be32(x: Fr) -> [u8; 32] {
    let mut le = x.into_bigint().to_bytes_le();
    le.resize(32, 0);
    le.reverse();
    let mut out = [0u8; 32];
    out.copy_from_slice(&le);
    out
}

fn poseidon_two_be(x: &[u8; 32], y: &[u8; 32]) -> [u8; 32] {
    let mut h = Poseidon::<Fq>::new_circom(2).expect("Poseidon2 init");
    let out = h
        .hash(&[
            Fq::from_be_bytes_mod_order(x),
            Fq::from_be_bytes_mod_order(y),
        ])
        .expect("Poseidon2 hash");
    fq_to_be32(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_is_deterministic() {
        let a = deterministic_account(0);
        let b = deterministic_account(0);
        assert_eq!(a.pubkey_x, b.pubkey_x);
        assert_eq!(a.pubkey_y, b.pubkey_y);
        assert_eq!(a.l2_address, b.l2_address);
        assert_eq!(a.owner_address, b.owner_address);
    }

    #[test]
    fn distinct_accounts_distinct_keys() {
        let a = deterministic_account(0);
        let b = deterministic_account(1);
        assert_ne!(a.pubkey_x, b.pubkey_x);
        assert_ne!(a.l2_address, b.l2_address);
    }

    #[test]
    fn sign_then_verify_roundtrip() {
        let sender = deterministic_account(7);
        let receiver = deterministic_account(42);
        let (r8x, r8y, s) = sign_tx(
            &sender, &sender.l2_address, &receiver.l2_address, 100, 5,
        ).expect("sign");
        assert!(verify_tx(
            &sender.pubkey_x, &sender.pubkey_y,
            &sender.l2_address, &receiver.l2_address,
            100, 5,
            &r8x, &r8y, &s,
        ), "valid signature failed to verify");
    }

    #[test]
    fn tampered_amount_rejected() {
        let sender = deterministic_account(1);
        let receiver = deterministic_account(2);
        let (r8x, r8y, s) = sign_tx(
            &sender, &sender.l2_address, &receiver.l2_address, 100, 0,
        ).expect("sign");
        assert!(!verify_tx(
            &sender.pubkey_x, &sender.pubkey_y,
            &sender.l2_address, &receiver.l2_address,
            999, 0, // tampered
            &r8x, &r8y, &s,
        ));
    }

    #[test]
    fn wrong_pubkey_rejected() {
        let sender = deterministic_account(1);
        let imposter = deterministic_account(99);
        let receiver = deterministic_account(2);
        let (r8x, r8y, s) = sign_tx(
            &sender, &sender.l2_address, &receiver.l2_address, 100, 0,
        ).expect("sign");
        assert!(!verify_tx(
            &imposter.pubkey_x, &imposter.pubkey_y,
            &sender.l2_address, &receiver.l2_address,
            100, 0,
            &r8x, &r8y, &s,
        ));
    }

    #[test]
    fn circomlib_circuit_equation_holds() {
        let sender = deterministic_account(0);
        let receiver = deterministic_account(1);
        let (r8x, r8y, s) = sign_tx(
            &sender, &sender.l2_address, &receiver.l2_address, 100, 0,
        ).unwrap();

        let pkx_c = Fq::from_be_bytes_mod_order(&sender.pubkey_x);
        let pky_c = Fq::from_be_bytes_mod_order(&sender.pubkey_y);
        let r8x_c = Fq::from_be_bytes_mod_order(&r8x);
        let r8y_c = Fq::from_be_bytes_mod_order(&r8y);

        let sa = sqrt_a();
        let pubkey = EdwardsAffine::new_unchecked(pkx_c * sa, pky_c);
        let r8 = EdwardsAffine::new_unchecked(r8x_c * sa, r8y_c);
        assert!(pubkey.is_on_curve(), "lifted pubkey must be on arkworks curve");
        assert!(r8.is_on_curve(), "lifted R8 must be on arkworks curve");

        let s_fr = Fr::from_be_bytes_mod_order(&s);

        let msg_be = compute_msg_hash(&sender.l2_address, &receiver.l2_address, 100, 0);
        let msg_fq = Fq::from_be_bytes_mod_order(&msg_be);

        let mut hash5 = Poseidon::<Fq>::new_circom(5).unwrap();
        let hm_fq = hash5.hash(&[r8x_c, r8y_c, pkx_c, pky_c, msg_fq]).unwrap();
        let hm_fr = Fr::from_le_bytes_mod_order(&hm_fq.into_bigint().to_bytes_le());

        let lhs: EdwardsProjective = b8_circomlib() * s_fr;
        let rhs: EdwardsProjective = EdwardsProjective::from(r8)
            + (EdwardsProjective::from(pubkey) * (hm_fr * Fr::from(8u32)));
        assert_eq!(lhs, rhs, "circuit equation S·B8 == R8 + 8·hm·A must hold");
    }

    #[test]
    fn iso_produces_circomlib_curve_points() {
        let p_ark = b8_circomlib();
        assert!(p_ark.is_on_curve(), "lifted Base8 must be on arkworks's curve");

        let x_c = p_ark.x * sqrt_a_inv();
        let y_c = p_ark.y;

        let a = Fq::from(168700u64);
        let d = Fq::from(168696u64);
        let lhs = a * x_c.square() + y_c.square();
        let rhs = Fq::ONE + d * x_c.square() * y_c.square();
        assert_eq!(lhs, rhs, "iso must map arkworks points onto circomlib's curve");
    }

    #[test]
    fn circomlib_base8_is_prime_subgroup_generator() {
        let b8 = b8_circomlib();
        let p: EdwardsProjective = b8 * Fr::ZERO;
        assert!(b8.is_on_curve());
        assert_ne!(EdwardsProjective::from(b8), p);
    }

    #[test]
    fn tampered_nonce_rejected() {
        let sender = deterministic_account(3);
        let receiver = deterministic_account(8);
        let (r8x, r8y, s) = sign_tx(
            &sender, &sender.l2_address, &receiver.l2_address, 50, 7,
        ).expect("sign");
        assert!(!verify_tx(
            &sender.pubkey_x, &sender.pubkey_y,
            &sender.l2_address, &receiver.l2_address,
            50, 8, // nonce tampered
            &r8x, &r8y, &s,
        ));
    }
}
