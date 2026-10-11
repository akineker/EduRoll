use crate::merkle_tree::PoseidonMerkleTree;
use crate::types::{Account, BatchInput, StfOutput, Tx};
use crate::N_TXS;
use std::collections::{BTreeMap, BTreeSet};
use tiny_keccak::{Hasher, Keccak};
use zk_precompiles::{poseidon2, poseidon4, verify_eddsa, Fr};

struct Identity {
    l2: Fr,
    pkx: Fr,
    pky: Fr,
}

pub fn encode_batch_data(txs: &[Tx]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + txs.len() * 160);
    out.extend_from_slice(&(txs.len() as u32).to_be_bytes());
    for tx in txs {
        out.extend_from_slice(&tx.sender);
        out.extend_from_slice(&tx.recipient);
        out.extend_from_slice(&tx.amount.to_be_bytes());
        out.extend_from_slice(&tx.nonce.to_be_bytes());
        out.extend_from_slice(&tx.sig_r8_x);
        out.extend_from_slice(&tx.sig_r8_y);
        out.extend_from_slice(&tx.sig_s);
    }
    out
}

fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak::v256();
    let mut out = [0u8; 32];
    hasher.update(data);
    hasher.finalize(&mut out);
    out
}

pub fn execute_batch(input: &BatchInput) -> StfOutput {
    assert!(
        input.txs.len() <= N_TXS,
        "batch has {} txs, circuit parity allows at most {N_TXS}",
        input.txs.len()
    );

    let mut tree = PoseidonMerkleTree::new();
    let mut by_addr: BTreeMap<[u8; 20], usize> = BTreeMap::new();
    let mut seen_leaves: BTreeSet<u64> = BTreeSet::new();
    let mut accounts: Vec<Account> = Vec::with_capacity(input.accounts.len());
    let mut ids: Vec<Identity> = Vec::with_capacity(input.accounts.len());
    for (i, witness) in input.accounts.iter().enumerate() {
        let acct = &witness.account;

        assert!(
            seen_leaves.insert(acct.leaf_index),
            "account {i}: duplicate leaf_index {} in witness",
            acct.leaf_index
        );

        let pkx = Fr::from_be_bytes_mod_order(&acct.l2_pubkey_x);
        let pky = Fr::from_be_bytes_mod_order(&acct.l2_pubkey_y);
        let l2 = poseidon2(&pkx, &pky);
        assert!(
            l2.to_be_bytes()[12..] == acct.owner_address[..],
            "account {i}: owner_address is not Poseidon(pubkey)[12..32] (spoofed DA identity)"
        );

        let siblings: Vec<Fr> = witness
            .siblings
            .iter()
            .map(|s| Fr::from_be_bytes_mod_order(s))
            .collect();
        tree.insert_leaf_with_path(acct.leaf_index, acct, &siblings);
        let prev = by_addr.insert(acct.owner_address, i);
        assert!(prev.is_none(), "duplicate account in witness");
        accounts.push(acct.clone());
        ids.push(Identity { l2, pkx, pky });
    }
    assert_eq!(
        tree.root.to_be_bytes(),
        input.old_root,
        "witness does not reconstruct the committed old_root"
    );
    let old_root = tree.root;

    let deposits_root = [0u8; 32];

    for (i, tx) in input.txs.iter().enumerate() {
        let sender_i = *by_addr
            .get(&tx.sender)
            .unwrap_or_else(|| panic!("tx {i}: unknown sender"));
        let recipient_i = *by_addr
            .get(&tx.recipient)
            .unwrap_or_else(|| panic!("tx {i}: unknown recipient"));

        let (sender, recipient) = (&ids[sender_i], &ids[recipient_i]);
        assert!(
            sender_i != recipient_i
                && sender.l2 != recipient.l2
                && accounts[sender_i].leaf_index != accounts[recipient_i].leaf_index,
            "tx {i}: self-transfer"
        );
        let (sender_l2, sender_pkx, sender_pky) = (sender.l2, sender.pkx, sender.pky);
        let recv_l2 = recipient.l2;

        let msg = poseidon4(
            &sender_l2,
            &recv_l2,
            &Fr::from_u128(tx.amount),
            &Fr::from_u64(tx.nonce),
        );
        let r8x = Fr::from_be_bytes_mod_order(&tx.sig_r8_x);
        let r8y = Fr::from_be_bytes_mod_order(&tx.sig_r8_y);
        assert!(
            verify_eddsa(&msg, &sender_pkx, &sender_pky, &r8x, &r8y, &tx.sig_s),
            "tx {i}: invalid EdDSA signature"
        );

        {
            let s = &accounts[sender_i];
            assert_eq!(s.nonce, tx.nonce, "tx {i}: nonce mismatch");
            assert!(s.balance >= tx.amount, "tx {i}: insufficient balance");
        }

        {
            let s = &mut accounts[sender_i];
            s.balance -= tx.amount;
            s.nonce += 1;
            let s = accounts[sender_i].clone();
            tree.update_leaf(s.leaf_index, &s);
        }
        {
            let r = &mut accounts[recipient_i];
            r.balance = r
                .balance
                .checked_add(tx.amount)
                .unwrap_or_else(|| panic!("tx {i}: receiver balance overflow"));
            let r = accounts[recipient_i].clone();
            tree.update_leaf(r.leaf_index, &r);
        }
    }

    let withdrawals_root = [0u8; 32];

    let batch_data_hash = keccak256(&encode_batch_data(&input.txs));

    StfOutput {
        old_root: old_root.to_be_bytes(),
        new_root: tree.root.to_be_bytes(),
        deposits_root,
        withdrawals_root,
        batch_data_hash,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Account, AccountWitness};
    use std::collections::BTreeSet;

    fn demo_input(n_accounts: u32, n_txs: usize) -> BatchInput {
        let accts = accounts::deterministic_accounts(n_accounts as usize);
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

        let mut tree = PoseidonMerkleTree::new();
        for a in &snapshot {
            tree.update_leaf(a.leaf_index, a);
        }
        let old_root = tree.root.to_be_bytes();

        let mut txs = Vec::new();
        let mut touched: BTreeSet<usize> = BTreeSet::new();
        for i in 0..n_txs {
            let s_idx = i % accts.len();
            let r_idx = (i + 1) % accts.len();
            let (s, r) = (&accts[s_idx], &accts[r_idx]);
            let amount = 100u128 + i as u128;
            let nonce = (0..i).filter(|j| j % accts.len() == s_idx).count() as u64;
            let (r8x, r8y, s_sig) =
                accounts::sign_tx(s, &s.l2_address, &r.l2_address, amount, nonce).unwrap();
            txs.push(Tx {
                sender: s.owner_address,
                recipient: r.owner_address,
                amount,
                nonce,
                sig_r8_x: r8x,
                sig_r8_y: r8y,
                sig_s: s_sig,
            });
            touched.insert(s_idx);
            touched.insert(r_idx);
        }

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

    #[test]
    fn tree_root_parity_with_native_merkle_crate() {
        let accts = accounts::deterministic_accounts(8);

        let mut ours = PoseidonMerkleTree::new();
        let mut theirs = merkle::PoseidonMerkleTree::new(crate::DEPTH);
        for a in &accts {
            let acct = Account {
                owner_address: a.owner_address,
                l2_pubkey_x: a.pubkey_x,
                l2_pubkey_y: a.pubkey_y,
                balance: 1_000_000,
                nonce: 0,
                leaf_index: a.index as u64,
            };
            ours.update_leaf(acct.leaf_index, &acct);
            let native_account = types::AccountState {
                owner_address: ethers::types::Address::from(acct.owner_address),
                l2_address: types::crypto_utils::compute_l2_address(
                    &acct.l2_pubkey_x,
                    &acct.l2_pubkey_y,
                ),
                l2_pubkey_x: acct.l2_pubkey_x,
                l2_pubkey_y: acct.l2_pubkey_y,
                balance: acct.balance,
                nonce: acct.nonce,
                leaf_index: acct.leaf_index,
            };
            theirs.update_leaf_from_account(acct.leaf_index, &native_account);
        }
        assert_eq!(ours.root.to_be_bytes(), theirs.root.0, "tree root diverged");
    }

    #[test]
    fn sparse_witness_matches_full_tree() {
        let accts = accounts::deterministic_accounts(16);
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

        let mut full = PoseidonMerkleTree::new();
        for a in &snapshot {
            full.update_leaf(a.leaf_index, a);
        }

        let mut sparse = PoseidonMerkleTree::new();
        for &idx in &[0usize, 1usize] {
            let siblings = full.get_path(snapshot[idx].leaf_index);
            sparse.insert_leaf_with_path(snapshot[idx].leaf_index, &snapshot[idx], &siblings);
        }
        assert_eq!(
            sparse.root.to_be_bytes(),
            full.root.to_be_bytes(),
            "sparse reconstruction diverged from the full tree"
        );

        let mut a0 = snapshot[0].clone();
        a0.balance -= 100;
        a0.nonce += 1;
        full.update_leaf(a0.leaf_index, &a0);
        sparse.update_leaf(a0.leaf_index, &a0);

        let mut a1 = snapshot[1].clone();
        a1.balance += 100;
        full.update_leaf(a1.leaf_index, &a1);
        sparse.update_leaf(a1.leaf_index, &a1);

        assert_eq!(
            sparse.root.to_be_bytes(),
            full.root.to_be_bytes(),
            "sparse chained update diverged from the full tree"
        );
    }

    #[test]
    fn executes_signed_batch() {
        let input = demo_input(8, 5);
        let out = execute_batch(&input);
        assert_ne!(out.old_root, out.new_root);
        assert_eq!(out.deposits_root, [0u8; 32]);
        assert_eq!(out.withdrawals_root, [0u8; 32]);
        assert_eq!(
            out.batch_data_hash,
            {
                let mut h = tiny_keccak::Keccak::v256();
                let mut o = [0u8; 32];
                tiny_keccak::Hasher::update(&mut h, &encode_batch_data(&input.txs));
                tiny_keccak::Hasher::finalize(h, &mut o);
                o
            }
        );
    }

    #[test]
    #[should_panic(expected = "invalid EdDSA signature")]
    fn tampered_amount_is_unprovable() {
        let mut input = demo_input(4, 1);
        input.txs[0].amount += 1;
        execute_batch(&input);
    }

    #[test]
    #[should_panic(expected = "nonce mismatch")]
    fn wrong_nonce_is_unprovable() {
        let mut input = demo_input(4, 1);
        let accts = accounts::deterministic_accounts(4);
        let s = &accts[0];
        let r = &accts[1];
        let (r8x, r8y, sig) =
            accounts::sign_tx(s, &s.l2_address, &r.l2_address, 100, 7).unwrap();
        input.txs[0] = Tx {
            sender: s.owner_address,
            recipient: r.owner_address,
            amount: 100,
            nonce: 7, // account nonce is 0
            sig_r8_x: r8x,
            sig_r8_y: r8y,
            sig_s: sig,
        };
        execute_batch(&input);
    }

    #[test]
    #[should_panic(expected = "self-transfer")]
    fn self_transfer_is_unprovable() {
        let mut input = demo_input(4, 1);
        let accts = accounts::deterministic_accounts(4);
        let s = &accts[0];
        let (r8x, r8y, sig) =
            accounts::sign_tx(s, &s.l2_address, &s.l2_address, 100, 0).unwrap();
        input.txs[0] = Tx {
            sender: s.owner_address,
            recipient: s.owner_address,
            amount: 100,
            nonce: 0,
            sig_r8_x: r8x,
            sig_r8_y: r8y,
            sig_s: sig,
        };
        execute_batch(&input);
    }

    fn snapshot(n: usize) -> Vec<Account> {
        accounts::deterministic_accounts(n)
            .iter()
            .map(|a| Account {
                owner_address: a.owner_address,
                l2_pubkey_x: a.pubkey_x,
                l2_pubkey_y: a.pubkey_y,
                balance: 1_000_000,
                nonce: 0,
                leaf_index: a.index as u64,
            })
            .collect()
    }

    fn full_tree(snapshot: &[Account]) -> PoseidonMerkleTree {
        let mut tree = PoseidonMerkleTree::new();
        for a in snapshot {
            tree.update_leaf(a.leaf_index, a);
        }
        tree
    }

    fn witness(tree: &PoseidonMerkleTree, a: &Account) -> AccountWitness {
        AccountWitness {
            account: a.clone(),
            siblings: tree
                .get_path(a.leaf_index)
                .iter()
                .map(|s| s.to_be_bytes())
                .collect(),
        }
    }

    fn assert_rejected(input: &BatchInput, expected: &str, minted_root: Option<[u8; 32]>) {
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| execute_batch(input)));
        match res {
            Ok(out) => panic!(
                "STF ACCEPTED a malicious batch: new_root = 0x{}{}",
                hex::encode(out.new_root),
                if Some(out.new_root) == minted_root { "  == the attacker's MINTED root" } else { "" }
            ),
            Err(e) => {
                let msg = e
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                assert!(
                    msg.contains(expected),
                    "rejected for the wrong reason: {msg:?} (expected it to contain {expected:?})"
                );
            }
        }
    }

    fn duplicate_leaf_exploit() -> (BatchInput, [u8; 32]) {
        let accts = accounts::deterministic_accounts(4);
        let snap = snapshot(4);
        let tree = full_tree(&snap);
        let attacker = &accts[0];
        let amount = 500_000u128;

        let (r8x, r8y, s) =
            accounts::sign_tx(attacker, &attacker.l2_address, &attacker.l2_address, amount, 0)
                .unwrap();

        let real_label = snap[0].owner_address;
        let alias_label = [0xAA; 20];
        let honest = witness(&tree, &snap[0]);
        let mut alias = witness(&tree, &snap[0]);
        alias.account.owner_address = alias_label;

        let input = BatchInput {
            batch_number: 1,
            old_root: tree.root.to_be_bytes(),
            accounts: vec![honest, alias],
            txs: vec![Tx {
                sender: real_label,
                recipient: alias_label,
                amount,
                nonce: 0,
                sig_r8_x: r8x,
                sig_r8_y: r8y,
                sig_s: s,
            }],
        };

        let mut minted = snap.clone();
        minted[0].balance += amount;
        (input, full_tree(&minted).root.to_be_bytes())
    }

    #[test]
    fn test_reject_duplicate_leaf_minting() {
        let (input, minted_root) = duplicate_leaf_exploit();
        assert_rejected(&input, "duplicate leaf_index", Some(minted_root));
    }

    #[test]
    fn test_reject_duplicate_leaf_same_label() {
        let (mut input, _) = duplicate_leaf_exploit();
        let label = input.accounts[0].account.owner_address;
        input.accounts[1].account.owner_address = label;
        input.txs[0].recipient = label;
        assert_rejected(&input, "duplicate", None);
    }

    #[test]
    fn test_reject_spoofed_owner_address() {
        let accts = accounts::deterministic_accounts(4);
        let snap = snapshot(4);
        let tree = full_tree(&snap);
        let spoof = [0xEE; 20];

        let (r8x, r8y, s) =
            accounts::sign_tx(&accts[0], &accts[0].l2_address, &accts[1].l2_address, 100, 0)
                .unwrap();
        let mut w1 = witness(&tree, &snap[1]);
        w1.account.owner_address = spoof;

        let input = BatchInput {
            batch_number: 1,
            old_root: tree.root.to_be_bytes(),
            accounts: vec![witness(&tree, &snap[0]), w1],
            txs: vec![Tx {
                sender: snap[0].owner_address,
                recipient: spoof,
                amount: 100,
                nonce: 0,
                sig_r8_x: r8x,
                sig_r8_y: r8y,
                sig_s: s,
            }],
        };
        assert_rejected(&input, "owner_address", None);
    }

    #[test]
    fn test_reject_out_of_range_leaf_index() {
        let accts = accounts::deterministic_accounts(4);
        let snap = snapshot(4);
        let tree = full_tree(&snap);

        let (r8x, r8y, s) =
            accounts::sign_tx(&accts[0], &accts[0].l2_address, &accts[1].l2_address, 100, 0)
                .unwrap();
        let mut aliased = witness(&tree, &snap[0]);
        aliased.account.leaf_index += 1u64 << crate::DEPTH;

        let input = BatchInput {
            batch_number: 1,
            old_root: tree.root.to_be_bytes(),
            accounts: vec![aliased, witness(&tree, &snap[1])],
            txs: vec![Tx {
                sender: snap[0].owner_address,
                recipient: snap[1].owner_address,
                amount: 100,
                nonce: 0,
                sig_r8_x: r8x,
                sig_r8_y: r8y,
                sig_s: s,
            }],
        };
        assert_rejected(&input, "out of range", None);
    }

    #[test]
    fn eddsa_parity_with_accounts_crate() {
        let accts = accounts::deterministic_accounts(2);
        let (s, r) = (&accts[0], &accts[1]);
        let (r8x, r8y, sig) =
            accounts::sign_tx(s, &s.l2_address, &r.l2_address, 12345, 3).unwrap();

        let sender_l2 = Fr::from_be_bytes_mod_order(&s.l2_address);
        let recv_l2 = Fr::from_be_bytes_mod_order(&r.l2_address);
        let msg = poseidon4(&sender_l2, &recv_l2, &Fr::from_u128(12345), &Fr::from_u64(3));

        assert!(verify_eddsa(
            &msg,
            &Fr::from_be_bytes_mod_order(&s.pubkey_x),
            &Fr::from_be_bytes_mod_order(&s.pubkey_y),
            &Fr::from_be_bytes_mod_order(&r8x),
            &Fr::from_be_bytes_mod_order(&r8y),
            &sig,
        ));
        
        let bad_msg = poseidon4(&sender_l2, &recv_l2, &Fr::from_u128(99999), &Fr::from_u64(3));
        assert!(!verify_eddsa(
            &bad_msg,
            &Fr::from_be_bytes_mod_order(&s.pubkey_x),
            &Fr::from_be_bytes_mod_order(&s.pubkey_y),
            &Fr::from_be_bytes_mod_order(&r8x),
            &Fr::from_be_bytes_mod_order(&r8y),
            &sig,
        ));
    }
}
