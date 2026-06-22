// ============================================================================
// TEST FILE - Unit tests for the Keccak Merkle tree used for the L1 batch commitment
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_batch_root() {
        let txs = vec![vec![0u8; 32]; BATCH_SIZE];
        let tree = KeccakMerkleTree::new(txs);
        let root = tree.get_root();

        assert_ne!(root.as_ref(), &[0u8; 32]);

        // Same input must always produce the same root.
        let tree2 = KeccakMerkleTree::new(vec![vec![0u8; 32]; BATCH_SIZE]);
        assert_eq!(root, tree2.get_root());
    }

    #[test]
    #[should_panic]
    fn test_invalid_batch_size() {
        let _ = KeccakMerkleTree::new(vec![vec![0u8; 32]; 10]);
    }
}
