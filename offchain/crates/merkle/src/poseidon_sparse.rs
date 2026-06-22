use std::collections::HashMap;
use types::{AccountState, PoseidonHash};
use types::crypto_utils::{poseidon_hash_two, poseidon_hash_three};

#[derive(Debug, Clone)]
pub struct PoseidonMerkleTree {
    pub root: PoseidonHash,
    pub depth: usize,
    pub node_cache: HashMap<(usize, u64), PoseidonHash>,
    pub default_nodes: Vec<PoseidonHash>,
}

impl PoseidonMerkleTree {
    pub fn new(depth: usize) -> Self {
        let default_nodes = Self::build_default_nodes(depth);
        let root = default_nodes[depth].clone();
        Self {
            root,
            depth,
            node_cache: HashMap::new(),
            default_nodes,
        }
    }

    fn build_default_nodes(depth: usize) -> Vec<PoseidonHash> {
        let mut nodes = vec![PoseidonHash::from([0u8; 32]); depth + 1];
        for i in 0..depth {
            nodes[i + 1] = poseidon_hash_two(&nodes[i], &nodes[i]);
        }
        nodes
    }

    fn hash_leaf(account: &AccountState) -> PoseidonHash {
        let pk_x = PoseidonHash::from(account.l2_pubkey_x);
        let pk_y = PoseidonHash::from(account.l2_pubkey_y);
        let address = poseidon_hash_two(&pk_x, &pk_y);

        let mut balance_bytes = [0u8; 32];
        // u128 → BE → right-aligned in the 32-byte field.
        balance_bytes[16..].copy_from_slice(&account.balance.to_be_bytes());

        let mut nonce_bytes = [0u8; 32];
        // u64 → BE → right-aligned in the 32-byte field.
        nonce_bytes[24..].copy_from_slice(&account.nonce.to_be_bytes());

        poseidon_hash_three(&address.0, &balance_bytes, &nonce_bytes)
    }

    pub fn update_leaf_from_account(&mut self, index: u64, account: &AccountState) -> PoseidonHash {
        let leaf_hash = Self::hash_leaf(account);
        self.node_cache.insert((0, index), leaf_hash.clone());

        let mut current_hash = leaf_hash;
        let mut current_index = index;

        for level in 0..self.depth {
            let is_right = current_index % 2 == 1;
            let sibling_index = if is_right { current_index - 1 } else { current_index + 1 };

            let sibling = self
                .node_cache
                .get(&(level, sibling_index))
                .cloned()
                .unwrap_or_else(|| self.default_nodes[level].clone());

            current_hash = if is_right {
                poseidon_hash_two(&sibling, &current_hash)
            } else {
                poseidon_hash_two(&current_hash, &sibling)
            };

            current_index /= 2;
            self.node_cache.insert((level + 1, current_index), current_hash.clone());
        }

        self.root = current_hash;
        self.root.clone()
    }

    pub fn get_proof(&self, index: u64) -> Vec<PoseidonHash> {
        let mut proof = Vec::with_capacity(self.depth);
        let mut current_index = index;

        for level in 0..self.depth {
            let is_right = current_index % 2 == 1;
            let sibling_index = if is_right { current_index - 1 } else { current_index + 1 };

            let sibling = self
                .node_cache
                .get(&(level, sibling_index))
                .cloned()
                .unwrap_or_else(|| self.default_nodes[level].clone());

            proof.push(sibling);
            current_index /= 2;
        }

        proof
    }
}
