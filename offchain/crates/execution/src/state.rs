use merkle::PoseidonMerkleTree;
use types::AccountState;
use std::collections::HashMap;

pub const TREE_DEPTH: usize = 20;
pub struct L2State {
    pub tree: PoseidonMerkleTree,
    pub account_indices: HashMap<[u8; 20], u64>,
    pub account_states: HashMap<[u8; 20], AccountState>,
}

impl L2State {
    pub fn new() -> Self {
        Self {
            tree: PoseidonMerkleTree::new(TREE_DEPTH),
            account_indices: HashMap::new(),
            account_states: HashMap::new(),
        }
    }

    pub fn get_account_index(&self, address: &[u8; 20]) -> Option<u64> {
        self.account_indices.get(address).copied()
    }
    
    pub fn next_leaf_index(&self) -> u64 {
        self.account_indices.len() as u64
    }
}

impl Default for L2State {
    fn default() -> Self {
        Self::new()
    }
}
