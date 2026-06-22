use anyhow::{anyhow, Context, Result};
use ethers::{
    abi::Address,
    middleware::SignerMiddleware,
    prelude::*,
    providers::{Http, Provider},
    signers::{LocalWallet, Signer},
    types::{U256, U64},
};
use serde::Deserialize;
use std::{env, sync::Arc, time::Duration};
use tokio::time::sleep;
use tracing::{error, info};

use storage::connection::establish_connection;

const POLL_INTERVAL_SECS: u64 = 10;

const DEFAULT_RPC_URL: &str = "http://anvil:8545";
const DEFAULT_CHAIN_ID: u64 = 1337;

abigen!(
    Rollup,
    r#"[
        struct PublicInputs { bytes32 oldRoot; bytes32 newRoot; bytes32 withdrawalsRoot; bytes32 depositsRoot; bytes32 batchDataHash; uint64 batchNumber; uint64 l1BlockNumber; uint32 circuitVersion; }
        function submitBatch(uint256[2] a, uint256[2][2] b, uint256[2] c, PublicInputs input, bytes batchData)
        function stateRoot() view returns (bytes32)
        function batchNumber() view returns (uint64)
    ]"#
);

#[derive(Debug, Deserialize)]
struct ProofData {
    a: [String; 2],
    b: [[String; 2]; 2],
    c: [String; 2],
    public_signals: [String; 3],
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = env::var("DATABASE_URL")
        .context("DATABASE_URL not set")?;
    let rpc_url = env::var("L1_RPC_URL")
        .unwrap_or_else(|_| DEFAULT_RPC_URL.into());
    let chain_id: u64 = env::var("L1_CHAIN_ID")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_CHAIN_ID);
    let private_key = env::var("L1_PRIVATE_KEY")
        .context("L1_PRIVATE_KEY not set (load from .env)")?;
    let rollup_address: Address = env::var("ROLLUP_ADDRESS")
        .context("ROLLUP_ADDRESS not set")?
        .parse()
        .context("ROLLUP_ADDRESS must be a valid hex address")?;

    let pool = establish_connection(&database_url).await?;
    info!("Connected to Postgres");

    let provider = Provider::<Http>::try_from(rpc_url.clone())?
        .interval(Duration::from_millis(2_000));
    let wallet: LocalWallet = private_key.parse::<LocalWallet>()?
        .with_chain_id(chain_id);
    let signer_addr = wallet.address();
    let client = Arc::new(SignerMiddleware::new(provider, wallet));
    let rollup = Rollup::new(rollup_address, client.clone());

    info!(
        "Submitter started — RPC={rpc_url} chain={chain_id} \
         signer={signer_addr:?} rollup={rollup_address:?}"
    );

    loop {
        match process_one_proven_batch(&pool, &rollup).await {
            Ok(true)  => info!("Batch submitted, checking for next"),
            Ok(false) => {
                info!("No PROVEN batches. Sleeping {POLL_INTERVAL_SECS}s");
                sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
            }
            Err(e) => {
                error!("Submission error: {e:#}");
                sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
            }
        }
    }
}

async fn process_one_proven_batch<M: Middleware + 'static>(
    pool: &sqlx::PgPool,
    rollup: &Rollup<M>,
) -> Result<bool> {
    let Some(batch) = storage::submitter::fetch_proven_batch(pool).await? else {
        return Ok(false);
    };

    let zk_proof_bytes = batch.zk_proof
        .as_ref()
        .ok_or_else(|| anyhow!("Batch #{} marked PROVEN but has no proof bytes", batch.batch_id))?;

    let proof: ProofData = serde_json::from_slice(zk_proof_bytes)
        .context("Failed to parse stored proof JSON")?;

    info!("Submitting batch #{}", batch.batch_id);

    let a = [u256_from_dec(&proof.a[0])?, u256_from_dec(&proof.a[1])?];
    let b = [
        [u256_from_dec(&proof.b[0][0])?, u256_from_dec(&proof.b[0][1])?],
        [u256_from_dec(&proof.b[1][0])?, u256_from_dec(&proof.b[1][1])?],
    ];
    let c = [u256_from_dec(&proof.c[0])?, u256_from_dec(&proof.c[1])?];

    let deposits_root = {
        let mut bytes = [0u8; 32];
        u256_from_dec(&proof.public_signals[2])?.to_big_endian(&mut bytes);
        bytes
    };

    let current_batch_number: u64 = rollup
        .batch_number()
        .call()
        .await
        .context("read Rollup.batchNumber()")?;
    let next_batch_number = current_batch_number + 1;

    let da_txs = storage::prover::fetch_batch_transactions(pool, batch.batch_id)
        .await
        .context("fetch batch transactions for DA")?;
    let mut batch_data: Vec<u8> = Vec::with_capacity(4 + da_txs.len() * 160);
    batch_data.extend_from_slice(&(da_txs.len() as u32).to_be_bytes());
    for tx in &da_txs {
        batch_data.extend_from_slice(&tx.sender.0);
        batch_data.extend_from_slice(&tx.recipient.0);
        batch_data.extend_from_slice(&tx.amount.to_be_bytes());
        batch_data.extend_from_slice(&tx.nonce.to_be_bytes());
        batch_data.extend_from_slice(&tx.sig_r8_x);
        batch_data.extend_from_slice(&tx.sig_r8_y);
        batch_data.extend_from_slice(&tx.sig_s);
    }
    let batch_data_hash = ethers::utils::keccak256(&batch_data);
    info!(
        "DA: posting {} tx(s) ({} bytes) as L1 calldata for batch #{}",
        da_txs.len(), batch_data.len(), batch.batch_id
    );

    let public_inputs = PublicInputs {
        old_root:          batch.pre_state_root_poseidon.0,
        new_root:          batch.post_state_root_poseidon.0,
        withdrawals_root:  [0u8; 32],
        deposits_root:     deposits_root,
        batch_data_hash:   batch_data_hash,
        batch_number:      next_batch_number,
        l_1_block_number:  0u64,
        circuit_version:   1u32,
    };

    let call = rollup.submit_batch(a, b, c, public_inputs, batch_data.into());
    let pending = call
        .send()
        .await
        .context("submitBatch tx send failed")?;
    let tx_hash = pending.tx_hash();
    info!("submitBatch tx sent: {:?}", tx_hash);

    let receipt = pending
        .await
        .context("Awaiting tx receipt failed")?
        .ok_or_else(|| anyhow!("submitBatch tx receipt was None"))?;

    let block_number: i64 = receipt.block_number
        .unwrap_or(U64::zero())
        .as_u64() as i64;

    storage::submitter::update_batch_submitted(
        pool,
        batch.batch_id,
        &tx_hash.to_fixed_bytes(),
    ).await?;

    storage::submitter::update_batch_finalized(
        pool,
        batch.batch_id,
        block_number,
    ).await?;

    info!("Batch #{} finalized at L1 block {}", batch.batch_id, block_number);
    Ok(true)
}

fn u256_from_dec(s: &str) -> Result<U256> {
    U256::from_dec_str(s).map_err(|e| anyhow!("Invalid decimal U256 '{s}': {e}"))
}
