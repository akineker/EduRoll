use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info};

use prover::{build_witness_from_db, prove_batch, ProofData, ProveTimings};
use storage::{
    connection::establish_connection,
    prover::{fetch_pending_batch, update_batch_to_proven},
};

const POLL_INTERVAL_SECS: u64 = 5;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL environment variable not set")?;

    let proving_key_path = PathBuf::from(
        std::env::var("ZK_PROVING_KEY_PATH")
            .unwrap_or_else(|_| "/app/circuits/build/transfer_final.zkey".into()),
    );
    let wasm_path = PathBuf::from(
        std::env::var("ZK_WASM_PATH").unwrap_or_else(|_| {
            "/app/circuits/build/transfer_eddsa_20_js/transfer_eddsa_20.wasm".into()
        }),
    );
    let wasm_js_path = PathBuf::from(
        std::env::var("ZK_WASM_JS_PATH").unwrap_or_else(|_| {
            "/app/circuits/build/transfer_eddsa_20_js/generate_witness.js".into()
        }),
    );

    let pool = establish_connection(&database_url)
        .await
        .context("Failed to connect to database")?;

    info!("Prover started (db mode) — polling every {POLL_INTERVAL_SECS}s");

    loop {
        let result =
            process_next_batch(&pool, &proving_key_path, &wasm_path, &wasm_js_path).await;

        match result {
            Ok(true)  => info!("Batch proven. Checking for next batch..."),
            Ok(false) => {
                info!("No pending batches. Sleeping for {POLL_INTERVAL_SECS}s...");
                sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
            }
            Err(e) => {
                error!("Proving error: {e:#}");
                sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
            }
        }
    }
}

async fn process_next_batch(
    pool: &storage::PgPool,
    proving_key_path: &Path,
    wasm_path: &Path,
    wasm_js_path: &Path,
) -> Result<bool> {
    let Some(batch) = fetch_pending_batch(pool).await? else {
        return Ok(false);
    };
    info!("Found pending batch #{} (DB mode)", batch.batch_id);
    
    let tmp = tempfile::tempdir().context("temp dir")?;
    let input_path = build_witness_from_db(pool, batch.batch_id, tmp.path()).await
        .context("build_witness_from_db")?;
    info!("  Witness assembled at {}", input_path.display());

    info!("  Generating ZK proof for batch #{}...", batch.batch_id);
    let (proof, timings): (ProofData, ProveTimings) = {
        let input = input_path.clone();
        let pk = proving_key_path.to_path_buf();
        let wasm = wasm_path.to_path_buf();
        let js = wasm_js_path.to_path_buf();
        tokio::task::spawn_blocking(move || prove_batch(&input, &pk, &wasm, &js))
            .await
            .context("Proving thread panicked")??
    };
    log_and_store(pool, batch.batch_id, proof, timings).await
}

async fn log_and_store(
    pool: &storage::PgPool,
    batch_id: i32,
    proof: ProofData,
    timings: ProveTimings,
) -> Result<bool> {
    info!(
        "  Batch #{} proof done — witness {} ms, proof {} ms (total {} ms)",
        batch_id, timings.witness_ms, timings.prove_ms,
        timings.witness_ms + timings.prove_ms,
    );

    let pre_root = decimal_to_bytes32(&proof.public_signals[0])
        .context("convert old_root")?;
    let post_root = decimal_to_bytes32(&proof.public_signals[1])
        .context("convert new_root")?;
    let proof_bytes = serde_json::to_vec(&proof).context("Failed to serialise proof")?;
    update_batch_to_proven(pool, batch_id, &proof_bytes, &pre_root, &post_root).await?;
    info!("  Batch #{} marked as PROVEN", batch_id);
    Ok(true)
}

fn decimal_to_bytes32(s: &str) -> Result<[u8; 32]> {
    let n = num_bigint::BigUint::parse_bytes(s.as_bytes(), 10)
        .ok_or_else(|| anyhow!("invalid decimal field element: {s}"))?;
    let be = n.to_bytes_be();
    if be.len() > 32 {
        return Err(anyhow!("field element exceeds 32 bytes"));
    }
    let mut out = [0u8; 32];
    out[32 - be.len()..].copy_from_slice(&be);
    Ok(out)
}
