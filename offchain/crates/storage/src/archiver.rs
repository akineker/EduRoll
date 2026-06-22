use anyhow::Result;
use sqlx::PgPool;

// Fetches all finalized batch IDs that haven't been archived yet, in ascending order
pub async fn fetch_unarchived_finalized_batches(pool: &PgPool) -> Result<Vec<i32>> {
    let rows = sqlx::query!(
        r#"
        SELECT batch_id
        FROM batches
        WHERE status = 'FINALIZED' AND archived_at IS NULL
        ORDER BY batch_id ASC
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.batch_id).collect())
}

// Fetches the raw zk_proof blob stored for a batch
pub async fn fetch_batch_proof_bytes(pool: &PgPool, batch_id: i32) -> Result<Option<Vec<u8>>> {
    let row = sqlx::query!(
        r#"
        SELECT zk_proof
        FROM batches
        WHERE batch_id = $1
        "#,
        batch_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.and_then(|r| r.zk_proof))
}

// Fetches all tx_hashes belonging to a batch for inclusion in the archive record
pub async fn fetch_batch_transaction_hashes(pool: &PgPool, batch_id: i32) -> Result<Vec<Vec<u8>>> {
    let rows = sqlx::query!(
        r#"
        SELECT tx_hash
        FROM transactions
        WHERE batch_id = $1
        ORDER BY created_at ASC
        "#,
        batch_id
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.tx_hash).collect())
}

// Marks a batch as archived and records where the archive was stored
pub async fn mark_batch_archived(
    pool: &PgPool,
    batch_id: i32,
    archive_uri: &str,
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE batches
        SET archived_at = NOW(),
            archive_uri = $1
        WHERE batch_id = $2
        "#,
        archive_uri,
        batch_id,
    )
    .execute(pool)
    .await?;

    Ok(())
}
