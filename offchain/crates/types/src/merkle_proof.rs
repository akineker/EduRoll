use serde::{Serialize, Deserialize};
use crate::hashes::PoseidonHash;
use crate::crypto_utils::poseidon_hash_two;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    pub leaf_index: u64,
    pub leaf_hash: PoseidonHash,
    pub siblings: Vec<PoseidonHash>, 
    pub root: PoseidonHash,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountProofResponse {
    pub address: String,
    pub balance: u128, 
    pub nonce: u64, 
    pub proof: MerkleProof,
}

impl MerkleProof {
    pub fn verify(&self) -> bool {
        let mut current_hash = self.leaf_hash.clone();
        let mut index = self.leaf_index;

        for sibling in self.siblings.iter() {
            let (left, right) = if index % 2 == 0 {
                (&current_hash, sibling)
            } else {
                (sibling, &current_hash)
            };

            // Call the cryptographic utility directly
            current_hash = poseidon_hash_two(left, right);
            index /= 2;
        }
        current_hash == self.root
    }
}