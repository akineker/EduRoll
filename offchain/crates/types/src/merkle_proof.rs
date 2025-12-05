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

impl MerkleProof {
    pub fn verify(&self, hash_function: impl Fn(&Hash, &Hash) -> Hash) -> bool {
        let mut current_hash = self.leaf_hash;
        let mut index = self.leaf_index;

        for sibling in self.siblings.iter() {
            let (left, right) = if index % 2 == 0 {
                (&current_hash, sibling)
            } else {
                (sibling, &current_hash)
            };

            current_hash = hash_function(left, right);
            index /= 2;
        }
        current_hash == self.root
    }
}