// ============================================================================
// TEST FILE -  Unit tests for the Poseidon Merkle tree (PoseidonMerkleTree)
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use types::AccountState;

    #[test]
    fn test_smt_initialization() {
        let tree = PoseidonMerkleTree::new(20);
        assert_eq!(tree.root, tree.default_nodes[20]);
    }

    #[test]
    fn test_leaf_update_changes_root() {
        let mut tree = PoseidonMerkleTree::new(20);
        let initial_root = tree.root.clone();

        let account = AccountState::default();
        tree.update_leaf_from_account(0, &account);

        assert_ne!(initial_root, tree.root);
    }

    #[test]
    fn test_two_updates_are_deterministic() {
        let account_a = AccountState { balance: 1000, nonce: 1, ..Default::default() };
        let account_b = AccountState { balance: 500,  nonce: 2, ..Default::default() };

        let mut tree1 = PoseidonMerkleTree::new(20);
        tree1.update_leaf_from_account(0, &account_a);
        tree1.update_leaf_from_account(1, &account_b);

        let mut tree2 = PoseidonMerkleTree::new(20);
        tree2.update_leaf_from_account(0, &account_a);
        tree2.update_leaf_from_account(1, &account_b);

        assert_eq!(tree1.root, tree2.root);
    }

    #[test]
    fn test_proof_length_equals_depth() {
        let mut tree = PoseidonMerkleTree::new(20);
        let account = AccountState { balance: 42, ..Default::default() };
        tree.update_leaf_from_account(5, &account);

        let proof = tree.get_proof(5);
        assert_eq!(proof.len(), 20);
    }
}
