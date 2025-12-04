use serde::{Serialize, Deserialize};
use crate::Hash;
use ethers::types::U256;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    pub leaf_index: u64,
    pub leaf_hash: Hash,
    pub siblings: Vec<Hash>, // The path to the root
    pub root: Hash,
}

/// Used for the "Withdraw" API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountProofResponse {
    pub address: String,
    pub balance: U256,
    pub nonce: U256,
    pub proof: MerkleProof,
}