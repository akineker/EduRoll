use anyhow::Result;
use sqlx::PgPool;
use std::time::Duration;
use tokio::time;
use tracing::{error, info, warn};

use crate::state_loader::{load_full_state, persist_state_changes};
use execution::execute_batch;

const BATCH_SIZE: i64 = 20;
const POLL_INTERVAL_SECS: u64 = 10;

pub async fn start_batch_processor(pool: PgPool) -> Result<()> {
    let mut tick = time::interval(Duration::from_secs(POLL_INTERVAL_SECS));
    info!("Batch processor started. Polling every {POLL_INTERVAL_SECS}s for {BATCH_SIZE} pending txs");

    loop {
        tick.tick().await;
        if let Err(e) = try_close_batch(&pool).await {
            error!("Batch close failed: {e:#}");
        }
    }
}

async fn try_close_batch(pool: &PgPool) -> Result<()> {
    let txs = storage::sequencer::fetch_pending_transactions(pool, BATCH_SIZE).await?;
    if (txs.len() as i64) < BATCH_SIZE {
        return Ok(()); // not enough yet
    }

    info!("Closing batch with {} txs", txs.len());

    let state = load_full_state(pool).await?;
    let pre_state_root_poseidon = state.tree.root;

    let (post_state_root_poseidon, updated_state) = match execute_batch(state, txs.clone()) {
        Ok(out) => out,
        Err(e) => {
            warn!("Batch execution failed — leaving txs pending: {e:#}");
            return Ok(());
        }
    };

    let mut db_tx = pool.begin().await?;

    let tx_hashes: Vec<[u8; 32]> = txs.iter().map(|t| t.tx_hash.0).collect();

    let batch_row = sqlx::query!(
        r#"
        INSERT INTO batches
            (pre_state_root_poseidon, post_state_root_poseidon, status)
        VALUES ($1, $2, 'PENDING_PROOF')
        RETURNING batch_id
        "#,
        &pre_state_root_poseidon.0  as &[u8],
        &post_state_root_poseidon.0 as &[u8],
    )
    .fetch_one(&mut *db_tx)
    .await?;
    let batch_id = batch_row.batch_id;

    for hash in &tx_hashes {
        sqlx::query!(
            r#"
            UPDATE transactions
            SET batch_id = $1, status = 'ACCEPTED_ON_L2'
            WHERE tx_hash = $2
            "#,
            batch_id,
            hash as &[u8],
        )
        .execute(&mut *db_tx)
        .await?;
    }
    
    storage::sequencer::write_batch_snapshot(&mut db_tx, batch_id).await?;

    persist_state_changes(pool, &mut db_tx, &updated_state).await?;
    db_tx.commit().await?;

    info!("Batch #{batch_id} closed and marked PENDING_PROOF (snapshot stored)");
    Ok(())
}
