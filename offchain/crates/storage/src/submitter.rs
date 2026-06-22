use anyhow::Result;
use sqlx::PgPool;
use types::{Batch, BatchStatus};

// Fetches the oldest batch that has a proof ready but hasnot been posted to L1 yet
pub async fn fetch_proven_batch(pool: &PgPool) -> Result<Option<Batch>> {
    let row = sqlx::query!(
        r#"
        SELECT batch_id, pre_state_root_poseidon, post_state_root_poseidon,
               zk_proof, l1_tx_hash, l1_block_number, status
        FROM batches
        WHERE status = 'PROVEN'
        ORDER BY batch_id ASC
        LIMIT 1
        "#
    )
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else { return Ok(None) };

    Ok(Some(Batch {
        batch_id: row.batch_id,
        transactions: vec![],
        pre_state_root_poseidon: row
            .pre_state_root_poseidon
            .as_slice()
            .try_into()
            .map(|b: [u8; 32]| b.into())
            .unwrap_or_default(),
        post_state_root_poseidon: row
            .post_state_root_poseidon
            .as_slice()
            .try_into()
            .map(|b: [u8; 32]| b.into())
            .unwrap_or_default(),
        zk_proof: row.zk_proof,
        l1_tx_hash: None,
        l1_block_number: row.l1_block_number,
        status: BatchStatus::Proven,
    }))
}

// Records the L1 tx hash after submitBatch() is sent and marks the batch SUBMITTED_TO_L1
pub async fn update_batch_submitted(
    pool: &PgPool,
    batch_id: i32,
    l1_tx_hash: &[u8; 32],
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE batches
        SET status       = 'SUBMITTED_TO_L1',
            l1_tx_hash   = $1,
            submitted_at = NOW()
        WHERE batch_id = $2
        "#,
        l1_tx_hash as &[u8],
        batch_id,
    )
    .execute(pool)
    .await?;

    Ok(())
}

// Records the L1 block number once the tx is confirmed and marks the batch FINALIZED
// Also marks every transaction in this batch as FINALIZED
pub async fn update_batch_finalized(
    pool: &PgPool,
    batch_id: i32,
    l1_block_number: i64,
) -> Result<()> {
    let mut db_tx = pool.begin().await?;

    sqlx::query!(
        r#"
        UPDATE batches
        SET status          = 'FINALIZED',
            l1_block_number = $1,
            finalized_at    = NOW()
        WHERE batch_id = $2
        "#,
        l1_block_number,
        batch_id,
    )
    .execute(&mut *db_tx)
    .await?;

    sqlx::query!(
        r#"
        UPDATE transactions
        SET status = 'FINALIZED'
        WHERE batch_id = $1
        "#,
        batch_id,
    )
    .execute(&mut *db_tx)
    .await?;

    db_tx.commit().await?;
    Ok(())
}
