use anyhow::{Context, Result};
use std::{env, time::Duration};
use tokio::time::sleep;
use tracing::{error, info};

use storage::connection::establish_connection;

const POLL_INTERVAL_SECS: u64 = 30;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = env::var("DATABASE_URL").context("DATABASE_URL not set")?;
    let pool = establish_connection(&database_url).await?;
    info!("Archiver started — polling every {POLL_INTERVAL_SECS}s for FINALIZED batches");

    loop {
        match archive_unprocessed(&pool).await {
            Ok(n) if n > 0 => info!("Archived {n} batch(es) this tick"),
            Ok(_)          => {}
            Err(e)         => error!("Archive tick failed: {e:#}"),
        }
        sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
    }
}

async fn archive_unprocessed(pool: &sqlx::PgPool) -> Result<usize> {
    let batch_ids = storage::archiver::fetch_unarchived_finalized_batches(pool).await?;
    let mut count = 0;

    for batch_id in batch_ids {
        let tx_hashes = storage::archiver::fetch_batch_transaction_hashes(pool, batch_id).await?;
        let proof    = storage::archiver::fetch_batch_proof_bytes(pool, batch_id).await?;

        info!(
            "Archiving batch #{batch_id}: {} txs, proof_bytes={}",
            tx_hashes.len(),
            proof.as_ref().map(|p| p.len()).unwrap_or(0)
        );

        // Show the first few tx hashes for a quick sanity check.
        for (i, h) in tx_hashes.iter().take(3).enumerate() {
            info!("  tx[{i}] = 0x{}", hex::encode(h));
        }

        let uri = format!("log://archived/batch/{batch_id}");
        storage::archiver::mark_batch_archived(pool, batch_id, &uri).await?;
        count += 1;
    }

    Ok(count)
}
