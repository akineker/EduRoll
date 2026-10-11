use crate::types::Account;
use crate::DEPTH;
use std::collections::BTreeMap;
use zk_precompiles::{poseidon2, poseidon3, Fr};

pub struct PoseidonMerkleTree {
    pub root: Fr,
    cache: BTreeMap<(usize, u64), Fr>,
    defaults: Vec<Fr>,
}

impl Default for PoseidonMerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
fn assert_canonical_index(index: u64) {
    assert!(
        index < (1u64 << DEPTH),
        "leaf_index {index} out of range for a depth-{DEPTH} tree"
    );
}

impl PoseidonMerkleTree {
    pub fn new() -> Self {
        let mut defaults = vec![Fr::ZERO; DEPTH + 1];
        for i in 0..DEPTH {
            defaults[i + 1] = poseidon2(&defaults[i], &defaults[i]);
        }
        let root = defaults[DEPTH];
        Self { root, cache: BTreeMap::new(), defaults }
    }

    pub fn hash_leaf(account: &Account) -> Fr {
        let pkx = Fr::from_be_bytes_mod_order(&account.l2_pubkey_x);
        let pky = Fr::from_be_bytes_mod_order(&account.l2_pubkey_y);
        let address = poseidon2(&pkx, &pky);
        poseidon3(
            &address,
            &Fr::from_u128(account.balance),
            &Fr::from_u64(account.nonce),
        )
    }

    pub fn update_leaf(&mut self, index: u64, account: &Account) -> Fr {
        assert_canonical_index(index);
        let mut current = Self::hash_leaf(account);
        self.cache.insert((0, index), current);

        let mut idx = index;
        for level in 0..DEPTH {
            let is_right = idx % 2 == 1;
            let sibling_idx = if is_right { idx - 1 } else { idx + 1 };
            let sibling = self
                .cache
                .get(&(level, sibling_idx))
                .copied()
                .unwrap_or(self.defaults[level]);
            current = if is_right {
                poseidon2(&sibling, &current)
            } else {
                poseidon2(&current, &sibling)
            };
            idx /= 2;
            self.cache.insert((level + 1, idx), current);
        }
        self.root = current;
        self.root
    }

    pub fn insert_leaf_with_path(&mut self, index: u64, account: &Account, siblings: &[Fr]) {
        assert_canonical_index(index);
        assert_eq!(
            siblings.len(),
            DEPTH,
            "witness path length must equal tree depth"
        );
        let leaf = Self::hash_leaf(account);
        if let Some(existing) = self.cache.get(&(0, index)).copied() {
            assert!(
                existing.to_be_bytes() == leaf.to_be_bytes(),
                "conflicting leaf at index {index}"
            );
        }
        self.cache.insert((0, index), leaf);

        let mut current = leaf;
        let mut idx = index;
        for level in 0..DEPTH {
            let is_right = idx % 2 == 1;
            let sibling_idx = if is_right { idx - 1 } else { idx + 1 };
            let sibling = match self.cache.get(&(level, sibling_idx)).copied() {
                Some(v) => v,
                None => {
                    self.cache.insert((level, sibling_idx), siblings[level]);
                    siblings[level]
                }
            };
            current = if is_right {
                poseidon2(&sibling, &current)
            } else {
                poseidon2(&current, &sibling)
            };
            idx /= 2;
            if let Some(existing) = self.cache.get(&(level + 1, idx)).copied() {
                assert!(
                    existing.to_be_bytes() == current.to_be_bytes(),
                    "inconsistent Merkle path at level {}",
                    level + 1
                );
            }
            self.cache.insert((level + 1, idx), current);
        }
        self.root = current;
    }

    pub fn get_path(&self, index: u64) -> Vec<Fr> {
        assert_canonical_index(index);
        let mut siblings = Vec::with_capacity(DEPTH);
        let mut idx = index;
        for level in 0..DEPTH {
            let is_right = idx % 2 == 1;
            let sibling_idx = if is_right { idx - 1 } else { idx + 1 };
            let sib = self
                .cache
                .get(&(level, sibling_idx))
                .copied()
                .unwrap_or(self.defaults[level]);
            siblings.push(sib);
            idx /= 2;
        }
        siblings
    }
}
