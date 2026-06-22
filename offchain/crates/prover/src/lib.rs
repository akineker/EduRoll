use anyhow::{anyhow, Context, Result};
use ethers::types::Address;
use merkle::PoseidonMerkleTree;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use tempfile::tempdir;
use tracing::info;
use types::{AccountState, crypto_utils::compute_l2_address};

const N_TXS: usize = 20;
const DEPTH: usize = 20;
const N_DEPOSITS: usize = 4;

#[derive(Deserialize)]
struct SnarkjsProof {
    pi_a: Vec<String>,
    pi_b: Vec<Vec<String>>,
    pi_c: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProofData {
    pub a: [String; 2],
    pub b: [[String; 2]; 2],
    pub c: [String; 2],
    pub public_signals: [String; 3],
}

#[derive(Debug, Clone, Copy)]
pub struct ProveTimings {
    pub witness_ms: u128,
    pub prove_ms: u128,
}

pub fn prove_batch(
    input_json_path: &Path,
    proving_key_path: &Path,
    wasm_path: &Path,
    wasm_js_path: &Path,
) -> Result<(ProofData, ProveTimings)> {
    let dir = tempdir().context("Failed to create temp directory")?;
    let witness_path = dir.path().join("witness.wtns");
    let proof_path = dir.path().join("proof.json");
    let public_path = dir.path().join("public.json");

    let t = Instant::now();
    run_command(
        "node",
        &[
            wasm_js_path.to_str().unwrap(),
            wasm_path.to_str().unwrap(),
            input_json_path.to_str().unwrap(),
            witness_path.to_str().unwrap(),
        ],
        "Witness generation",
    )?;
    let witness_ms = t.elapsed().as_millis();
    info!("  Witness computed in {witness_ms} ms");

    let t = Instant::now();
    run_command(
        "snarkjs",
        &[
            "groth16", "prove",
            proving_key_path.to_str().unwrap(),
            witness_path.to_str().unwrap(),
            proof_path.to_str().unwrap(),
            public_path.to_str().unwrap(),
        ],
        "Groth16 proving",
    )?;
    let prove_ms = t.elapsed().as_millis();
    info!("  Groth16 proof generated in {prove_ms} ms");

    let proof: SnarkjsProof = serde_json::from_str(
        &std::fs::read_to_string(&proof_path).context("read proof.json")?,
    )
    .context("parse proof.json")?;

    let public: Vec<String> = serde_json::from_str(
        &std::fs::read_to_string(&public_path).context("read public.json")?,
    )
    .context("parse public.json")?;
    if public.len() < 3 {
        return Err(anyhow!("expected 3 public signals, got {}", public.len()));
    }

    let data = ProofData {
        a: [proof.pi_a[0].clone(), proof.pi_a[1].clone()],
        b: [
            [proof.pi_b[0][1].clone(), proof.pi_b[0][0].clone()],
            [proof.pi_b[1][1].clone(), proof.pi_b[1][0].clone()],
        ],
        c: [proof.pi_c[0].clone(), proof.pi_c[1].clone()],
        public_signals: [public[0].clone(), public[1].clone(), public[2].clone()],
    };

    Ok((data, ProveTimings { witness_ms, prove_ms }))
}

fn run_command(program: &str, args: &[&str], label: &str) -> Result<()> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("{label}: failed to launch `{program}`"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("{label} failed:\n{stderr}"));
    }
    Ok(())
}

#[derive(Serialize)]
pub struct WitnessInput {
    pub old_root: String,
    pub new_root: String,
    pub deposits_root: String,

    pub deposit_pubkey_x:      Vec<String>,
    pub deposit_pubkey_y:      Vec<String>,
    pub deposit_amount:        Vec<String>,
    pub deposit_is_new:        Vec<String>,
    pub deposit_old_balance:   Vec<String>,
    pub deposit_old_nonce:     Vec<String>,
    pub deposit_path_elements: Vec<Vec<String>>,
    pub deposit_path_indices:  Vec<Vec<String>>,

    pub tx_sender_address:   Vec<String>,
    pub tx_receiver_address: Vec<String>,
    pub tx_amount:           Vec<String>,
    pub tx_nonce:            Vec<String>,

    pub sender_balance:       Vec<String>,
    pub sender_nonce:         Vec<String>,
    pub sender_path_elements: Vec<Vec<String>>,
    pub sender_path_indices:  Vec<Vec<String>>,

    pub receiver_balance:       Vec<String>,
    pub receiver_nonce:         Vec<String>,
    pub receiver_path_elements: Vec<Vec<String>>,
    pub receiver_path_indices:  Vec<Vec<String>>,

    #[serde(rename = "sig_R8x")]
    pub sig_r8x: Vec<String>,
    #[serde(rename = "sig_R8y")]
    pub sig_r8y: Vec<String>,
    #[serde(rename = "sig_S")]
    pub sig_s:   Vec<String>,

    pub sender_pub_key_x: Vec<String>,
    pub sender_pub_key_y: Vec<String>,
}

fn bytes_to_decimal(bytes: &[u8; 32]) -> String {
    let mut n = num_bigint::BigUint::from(0u32);
    let base = num_bigint::BigUint::from(256u32);
    for &byte in bytes.iter() {
        n = n * &base + num_bigint::BigUint::from(byte);
    }
    n.to_string()
}

fn zero_decimal() -> String { "0".to_string() }

pub async fn build_witness_from_db(
    pool: &PgPool,
    batch_id: i32,
    out_dir: &Path,
) -> Result<PathBuf> {
    let txs = storage::prover::fetch_batch_transactions(pool, batch_id).await
        .context("fetch_batch_transactions")?;
    let snap = storage::prover::fetch_batch_snapshot(pool, batch_id).await
        .context("fetch_batch_snapshot")?;

    if snap.is_empty() {
        return Err(anyhow!("batch_snapshots is empty for batch {batch_id}"));
    }

    let ref_acct = snap[0].clone();
    let mut tree = PoseidonMerkleTree::new(DEPTH);
    for a in &snap {
        let account = AccountState {
            owner_address: Address::from(a.owner_address),
            l2_address:    compute_l2_address(&a.l2_pubkey_x, &a.l2_pubkey_y),
            l2_pubkey_x:   a.l2_pubkey_x,
            l2_pubkey_y:   a.l2_pubkey_y,
            balance:       a.pre_balance,
            nonce:         a.pre_nonce,
            leaf_index:    a.leaf_index,
        };
        tree.update_leaf_from_account(a.leaf_index, &account);
    }
    let old_root = tree.root;

    let mut by_addr: HashMap<[u8; 20], storage::prover::SnapshotAccount> =
        snap.into_iter().map(|a| (a.owner_address, a)).collect();

    let mut w = WitnessInput {
        old_root: bytes_to_decimal(&old_root.0),
        new_root: String::new(),
        deposits_root: zero_decimal(),
        deposit_pubkey_x:      vec![zero_decimal(); N_DEPOSITS],
        deposit_pubkey_y:      vec![zero_decimal(); N_DEPOSITS],
        deposit_amount:        vec![zero_decimal(); N_DEPOSITS],
        deposit_is_new:        vec![zero_decimal(); N_DEPOSITS],
        deposit_old_balance:   vec![zero_decimal(); N_DEPOSITS],
        deposit_old_nonce:     vec![zero_decimal(); N_DEPOSITS],
        deposit_path_elements: vec![vec![zero_decimal(); DEPTH]; N_DEPOSITS],
        deposit_path_indices:  vec![vec![zero_decimal(); DEPTH]; N_DEPOSITS],
        tx_sender_address:      vec![zero_decimal(); N_TXS],
        tx_receiver_address:    vec![zero_decimal(); N_TXS],
        tx_amount:              vec![zero_decimal(); N_TXS],
        tx_nonce:               vec![zero_decimal(); N_TXS],
        sender_balance:         vec![zero_decimal(); N_TXS],
        sender_nonce:           vec![zero_decimal(); N_TXS],
        sender_path_elements:   vec![vec![zero_decimal(); DEPTH]; N_TXS],
        sender_path_indices:    vec![vec![zero_decimal(); DEPTH]; N_TXS],
        receiver_balance:       vec![zero_decimal(); N_TXS],
        receiver_nonce:         vec![zero_decimal(); N_TXS],
        receiver_path_elements: vec![vec![zero_decimal(); DEPTH]; N_TXS],
        receiver_path_indices:  vec![vec![zero_decimal(); DEPTH]; N_TXS],
        sig_r8x:          vec![zero_decimal(); N_TXS],
        sig_r8y:          vec![zero_decimal(); N_TXS],
        sig_s:            vec![zero_decimal(); N_TXS],
        sender_pub_key_x: vec![zero_decimal(); N_TXS],
        sender_pub_key_y: vec![zero_decimal(); N_TXS],
    };

    {
        let ref_idx = ref_acct.leaf_index;
        let dep_proof = tree.get_proof(ref_idx);
        let dep_bits: Vec<u8> = (0..DEPTH).map(|j| ((ref_idx >> j) & 1) as u8).collect();
        for d in 0..N_DEPOSITS {
            w.deposit_pubkey_x[d]    = bytes_to_decimal(&ref_acct.l2_pubkey_x);
            w.deposit_pubkey_y[d]    = bytes_to_decimal(&ref_acct.l2_pubkey_y);
            w.deposit_amount[d]      = zero_decimal();
            w.deposit_is_new[d]      = zero_decimal();
            w.deposit_old_balance[d] = ref_acct.pre_balance.to_string();
            w.deposit_old_nonce[d]   = ref_acct.pre_nonce.to_string();
            for j in 0..DEPTH {
                w.deposit_path_elements[d][j] = bytes_to_decimal(&dep_proof[j].0);
                w.deposit_path_indices[d][j]  = dep_bits[j].to_string();
            }
        }
        w.deposits_root = zero_decimal();
    }

    for (i, tx) in txs.iter().enumerate().take(N_TXS) {
        let sender = by_addr.get(&tx.sender.0)
            .ok_or_else(|| anyhow!("sender {:?} not in snapshot", tx.sender.0))?
            .clone();
        let recipient = by_addr.get(&tx.recipient.0)
            .ok_or_else(|| anyhow!("recipient {:?} not in snapshot", tx.recipient.0))?
            .clone();

        let sender_l2 = compute_l2_address(&sender.l2_pubkey_x, &sender.l2_pubkey_y);
        let receiver_l2 = compute_l2_address(&recipient.l2_pubkey_x, &recipient.l2_pubkey_y);

        // Sender proof at CURRENT tree.
        let sender_path = tree.get_proof(sender.leaf_index);
        let sender_idx_bits: Vec<u8> =
            (0..DEPTH).map(|j| ((sender.leaf_index >> j) & 1) as u8).collect();

        // Apply sender update (balance -= amount, nonce += 1).
        let new_sender_bal = sender.pre_balance.checked_sub(tx.amount)
            .ok_or_else(|| anyhow!("sender underflow at tx {i}"))?;
        let new_sender_nonce = sender.pre_nonce.checked_add(1)
            .ok_or_else(|| anyhow!("sender nonce overflow at tx {i}"))?;
        let updated_sender = AccountState {
            owner_address: Address::from(sender.owner_address),
            l2_address:    sender_l2,
            l2_pubkey_x:   sender.l2_pubkey_x,
            l2_pubkey_y:   sender.l2_pubkey_y,
            balance:       new_sender_bal,
            nonce:         new_sender_nonce,
            leaf_index:    sender.leaf_index,
        };
        tree.update_leaf_from_account(sender.leaf_index, &updated_sender);

        let recv_path = tree.get_proof(recipient.leaf_index);
        let recv_idx_bits: Vec<u8> =
            (0..DEPTH).map(|j| ((recipient.leaf_index >> j) & 1) as u8).collect();

        let new_recv_bal = recipient.pre_balance.checked_add(tx.amount)
            .ok_or_else(|| anyhow!("receiver overflow at tx {i}"))?;
        let updated_recipient = AccountState {
            owner_address: Address::from(recipient.owner_address),
            l2_address:    receiver_l2,
            l2_pubkey_x:   recipient.l2_pubkey_x,
            l2_pubkey_y:   recipient.l2_pubkey_y,
            balance:       new_recv_bal,
            nonce:         recipient.pre_nonce, // receiver nonce doesn't change
            leaf_index:    recipient.leaf_index,
        };
        tree.update_leaf_from_account(recipient.leaf_index, &updated_recipient);

        let s_addr = sender.owner_address;
        let r_addr = recipient.owner_address;
        by_addr.insert(s_addr, storage::prover::SnapshotAccount {
            pre_balance: new_sender_bal, pre_nonce: new_sender_nonce, ..sender
        });
        by_addr.insert(r_addr, storage::prover::SnapshotAccount {
            pre_balance: new_recv_bal, ..recipient
        });

        w.tx_sender_address[i]   = bytes_to_decimal(&sender_l2.0);
        w.tx_receiver_address[i] = bytes_to_decimal(&receiver_l2.0);
        w.tx_amount[i]           = tx.amount.to_string();
        w.tx_nonce[i]            = tx.nonce.to_string();

        let _ = (s_addr, r_addr);
        w.sender_balance[i] = sender.pre_balance.to_string();
        w.sender_nonce[i]   = sender.pre_nonce.to_string();

        for j in 0..DEPTH {
            w.sender_path_elements[i][j] = bytes_to_decimal(&sender_path[j].0);
            w.sender_path_indices[i][j]  = sender_idx_bits[j].to_string();
        }

        w.receiver_balance[i] = recipient.pre_balance.to_string();
        w.receiver_nonce[i]   = recipient.pre_nonce.to_string();

        for j in 0..DEPTH {
            w.receiver_path_elements[i][j] = bytes_to_decimal(&recv_path[j].0);
            w.receiver_path_indices[i][j]  = recv_idx_bits[j].to_string();
        }

        w.sig_r8x[i] = bytes_to_decimal(&tx.sig_r8_x);
        w.sig_r8y[i] = bytes_to_decimal(&tx.sig_r8_y);
        w.sig_s[i]   = bytes_to_decimal(&tx.sig_s);
        w.sender_pub_key_x[i] = bytes_to_decimal(&sender.l2_pubkey_x);
        w.sender_pub_key_y[i] = bytes_to_decimal(&sender.l2_pubkey_y);
    }

    w.new_root = bytes_to_decimal(&tree.root.0);

    std::fs::create_dir_all(out_dir).context("create out_dir")?;
    let path = out_dir.join("input.json");
    let json = serde_json::to_string_pretty(&w).context("serialise WitnessInput")?;
    std::fs::write(&path, &json).context("write input.json")?;
    Ok(path)
}
