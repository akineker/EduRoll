use anyhow::{Context, Result};
use std::env;
use tracing::info;

use sequencer::{start_api_server, start_batch_processor};
use storage::connection::establish_connection;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = env::var("DATABASE_URL")
        .context("DATABASE_URL must be set")?;

    let pool = establish_connection(&database_url)
        .await
        .context("Failed to connect to Postgres")?;
    info!("Sequencer connected to Postgres");

    let batcher_pool = pool.clone();
    tokio::spawn(async move {
        if let Err(e) = start_batch_processor(batcher_pool).await {
            tracing::error!("Batch processor exited: {e:#}");
        }
    });

    start_api_server(pool).await?;
    Ok(())
}
