use serde::{Serialize, Deserialize};
use crate::{L2Transaction, Hash};
use crate::messages::TxStatus;

use ark_bls12_381::{G1Affine, G2Affine, Bls12_381};
use ark_ff::{Fp, MontBackend, BigInt, PrimeField};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Batch {
    pub batch_id: u64,
    pub transactions: Vec<L2Transaction>,
    pub pre_state_root_poseidon: Hash,
    pub post_state_root_poseidon: Hash,
    pub pre_state_root_keccak: Hash,
    pub post_state_root_keccak: Hash,
    pub proof: Option<ZKProof>,
    pub l1_tx_hash: Option<Hash>,
    pub status: TxStatus,
}

/// Scalar field definition
type Fr = <Bls12_381 as ark_ec::PairingEngine>::Fr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZKProof {
    pub a: G1Affine,
    pub b: G2Affine,
    pub c: G1Affine,
}