use serde::{Serialize, Deserialize};
use crate::{L2Transaction, Hash};
use crate::messages::TxStatus;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZKProof {
    pub a: [u8; 32],
    pub b: [[u8; 32]; 2],
    pub c: [u8; 32],
}