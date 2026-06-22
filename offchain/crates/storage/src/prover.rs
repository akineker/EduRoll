use anyhow::Result;
use sqlx::PgPool;
use types::{Batch, BatchStatus, L2Transaction, L2TransactionStatus};

// Fetches the oldest batch waiting to be proven.
// Returns None if no work is available — the prover will sleep and retry.
pub async fn fetch_pending_batch(pool: &PgPool) -> Result<Option<Batch>> {
    let row = sqlx::query!(
        r#"
        SELECT batch_id, pre_state_root_poseidon, post_state_root_poseidon,
               zk_proof, l1_block_number
        FROM batches
        WHERE status = 'PENDING_PROOF'
        ORDER BY batch_id ASC
        LIMIT 1
        "#
    )
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else { return Ok(None) };

    Ok(Some(Batch {
        batch_id: row.batch_id,
        transactions: vec![], // loaded separately below
        pre_state_root_poseidon: row.pre_state_root_poseidon.as_slice().try_into()
            .map(|b: [u8; 32]| b.into())
            .unwrap_or_default(),
        post_state_root_poseidon: row.post_state_root_poseidon.as_slice().try_into()
            .map(|b: [u8; 32]| b.into())
            .unwrap_or_default(),
        zk_proof: row.zk_proof,
        l1_tx_hash: None,
        l1_block_number: row.l1_block_number,
        status: BatchStatus::PendingProof,
    }))
}

// Fetches all transactions belonging to a batch, ordered by insertion order.
pub async fn fetch_batch_transactions(pool: &PgPool, batch_id: i32) -> Result<Vec<L2Transaction>> {
    let rows = sqlx::query!(
        r#"
        SELECT tx_hash, sender_address, recipient_address,
               amount::TEXT as "amount!", nonce, fee::TEXT as "fee!", signature
        FROM transactions
        WHERE batch_id = $1
        ORDER BY created_at ASC
        "#,
        batch_id
    )
    .fetch_all(pool)
    .await?;

    let txs = rows.into_iter().map(|row| {
        let sig = row.signature;
        let mut r8x = [0u8; 32];
        let mut r8y = [0u8; 32];
        let mut s   = [0u8; 32];
        if sig.len() == 96 {
            r8x.copy_from_slice(&sig[0..32]);
            r8y.copy_from_slice(&sig[32..64]);
            s.copy_from_slice(&sig[64..96]);
        }

        let mut sender = [0u8; 20];
        let mut recipient = [0u8; 20];
        if row.sender_address.len() == 20 { sender.copy_from_slice(&row.sender_address); }
        if row.recipient_address.len() == 20 { recipient.copy_from_slice(&row.recipient_address); }

        let mut tx_hash_bytes = [0u8; 32];
        if row.tx_hash.len() == 32 { tx_hash_bytes.copy_from_slice(&row.tx_hash); }

        L2Transaction {
            tx_hash: tx_hash_bytes.into(),
            sender: sender.into(),
            recipient: recipient.into(),
            amount: row.amount.to_string().parse::<u128>().unwrap_or(0),
            fee: row.fee.to_string().parse::<u128>().unwrap_or(0),
            nonce: row.nonce as u64,
            status: L2TransactionStatus::AcceptedOnL2,
            batch_id: Some(batch_id),
            sig_r8_x: r8x,
            sig_r8_y: r8y,
            sig_s: s,
        }
    }).collect();

    Ok(txs)
}

#[derive(Debug, Clone)]
pub struct SnapshotAccount {
    pub leaf_index:    u64,
    pub owner_address: [u8; 20],
    pub l2_pubkey_x:   [u8; 32],
    pub l2_pubkey_y:   [u8; 32],
    pub pre_balance:   u128,
    pub pre_nonce:     u64,
}

// Loads the complete pre-batch account snapshot
pub async fn fetch_batch_snapshot(
    pool: &PgPool,
    batch_id: i32,
) -> Result<Vec<SnapshotAccount>> {
    let rows = sqlx::query!(
        r#"
        SELECT leaf_index, owner_address, l2_pubkey_x, l2_pubkey_y,
               pre_balance::TEXT as "pre_balance!", pre_nonce
        FROM batch_snapshots
        WHERE batch_id = $1
        ORDER BY leaf_index ASC
        "#,
        batch_id
    )
    .fetch_all(pool)
    .await?;

    let out = rows.into_iter().map(|r| {
        let mut owner = [0u8; 20];
        if r.owner_address.len() == 20 { owner.copy_from_slice(&r.owner_address); }
        let mut pkx = [0u8; 32];
        let mut pky = [0u8; 32];
        if r.l2_pubkey_x.len() == 32 { pkx.copy_from_slice(&r.l2_pubkey_x); }
        if r.l2_pubkey_y.len() == 32 { pky.copy_from_slice(&r.l2_pubkey_y); }
        SnapshotAccount {
            leaf_index:    r.leaf_index as u64,
            owner_address: owner,
            l2_pubkey_x:   pkx,
            l2_pubkey_y:   pky,
            pre_balance:   r.pre_balance.parse::<u128>().unwrap_or(0),
            pre_nonce:     r.pre_nonce as u64,
        }
    }).collect();

    Ok(out)
}

// Writes the serialised proof into the batch row
pub async fn update_batch_to_proven(
    pool: &PgPool,
    batch_id: i32,
    proof_bytes: &[u8],
    pre_state_root_poseidon: &[u8; 32],
    post_state_root_poseidon: &[u8; 32],
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE batches
        SET zk_proof                 = $1,
            status                   = 'PROVEN',
            proven_at                = NOW(),
            pre_state_root_poseidon  = $3,
            post_state_root_poseidon = $4
        WHERE batch_id = $2
        "#,
        proof_bytes,
        batch_id,
        pre_state_root_poseidon as &[u8],
        post_state_root_poseidon as &[u8],
    )
    .execute(pool)
    .await?;

    Ok(())
}
