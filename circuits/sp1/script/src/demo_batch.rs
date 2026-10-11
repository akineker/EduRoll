//! Builds a realistic demo batch for the SP1 guest: deterministic EduRoll
//! accounts (the same `SHA-256("eduroll-account-v1" || index)` derivation the
//! simulation seeds) and transfers signed with the REAL EdDSA signer from
//! `tools/accounts`. Transactions are generated the SAME way the `test_client`
//! docker container does (`tools/test_client/src/main.rs::send_random_tx`): a
//! random sender, a random DIFFERENT recipient drawn from the full account set,
//! a fixed amount, and the sender's running nonce. The guest receives only the
//! accounts the batch touches, each with its Merkle path against the pre-batch
//! root (a sparse witness), so its cost is independent of the total account
//! count - exactly like the Circom circuit, which also takes only Merkle paths.

use rand::Rng;
use std::collections::BTreeSet;
use stf_core::{Account, AccountWitness, BatchInput, PoseidonMerkleTree, Tx};

/// `n_accounts` registered accounts (1_000_000 balance each) and `n_txs` signed
/// transfers between random accounts (mirroring `test_client`). Only the
/// touched accounts go into the witness.
pub fn build(n_accounts: usize, n_txs: usize) -> BatchInput {
    assert!(n_accounts >= 2, "need at least 2 accounts");
    let accts = accounts::deterministic_accounts(n_accounts);

    let snapshot: Vec<Account> = accts
        .iter()
        .map(|a| Account {
            owner_address: a.owner_address,
            l2_pubkey_x: a.pubkey_x,
            l2_pubkey_y: a.pubkey_y,
            balance: 1_000_000,
            nonce: 0,
            leaf_index: a.index as u64,
        })
        .collect();

    // Build the full pre-batch tree once (host side, native) to get the
    // pre-batch root and each touched account's Merkle path.
    let mut tree = PoseidonMerkleTree::new();
    for acct in &snapshot {
        tree.update_leaf(acct.leaf_index, acct);
    }
    let old_root = tree.root.to_be_bytes();

    // Same tx generation as `test_client`'s `send_random_tx`: random sender,
    // random different recipient, fixed amount, sender's running nonce.
    let mut rng = rand::thread_rng();
    let amount = 100u128;
    let mut nonces = vec![0u64; n_accounts];
    let mut txs = Vec::with_capacity(n_txs);
    let mut touched: BTreeSet<usize> = BTreeSet::new();
    for _ in 0..n_txs {
        let s_idx = rng.gen_range(0..n_accounts);
        let mut r_idx = rng.gen_range(0..n_accounts);
        while r_idx == s_idx {
            r_idx = rng.gen_range(0..n_accounts);
        }
        let (s, r) = (&accts[s_idx], &accts[r_idx]);
        let nonce = nonces[s_idx];
        nonces[s_idx] += 1;

        let (r8x, r8y, sig_s) = accounts::sign_tx(s, &s.l2_address, &r.l2_address, amount, nonce)
            .expect("sign demo tx");
        txs.push(Tx {
            sender: s.owner_address,
            recipient: r.owner_address,
            amount,
            nonce,
            sig_r8_x: r8x,
            sig_r8_y: r8y,
            sig_s,
        });
        touched.insert(s_idx);
        touched.insert(r_idx);
    }

    // Sparse witness: only the touched accounts, each with its Merkle path.
    let accounts: Vec<AccountWitness> = touched
        .iter()
        .map(|&idx| AccountWitness {
            account: snapshot[idx].clone(),
            siblings: tree
                .get_path(snapshot[idx].leaf_index)
                .iter()
                .map(|s| s.to_be_bytes())
                .collect(),
        })
        .collect();

    BatchInput { batch_number: 1, old_root, accounts, txs }
}
