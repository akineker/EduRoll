use clap::Parser;
use rand::Rng;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use tracing::{error, info, warn};
use types::{L2Transaction, L2TransactionStatus, PoseidonHash};

// CLI
#[derive(Parser, Debug)]
#[command(
    name = "test_client",
    about = "Sends real-EdDSA-signed L2 transactions to the EduRoll sequencer"
)]
struct Args {
    /// Number of transactions to send (ignored when --continuous is set)
    #[arg(short, long, default_value_t = 20)]
    count: u64,

    /// Run forever: 20 txs back-to-back, then sleep a random 10-30s, repeat.
    #[arg(long, default_value_t = false)]
    continuous: bool,

    /// Sequencer HTTP endpoint
    #[arg(short, long, default_value = "http://localhost:9001")]
    url: String,

    /// Fixed delay between requests in non-continuous mode (ms)
    #[arg(long, default_value_t = 0)]
    delay_ms: u64,

    /// Transfer amount per transaction (max 2^128 - 1 enforced by circuit)
    #[arg(short, long, default_value_t = 100)]
    amount: u128,

    /// Number of accounts to load locally (must equal the seed_accounts count)
    #[arg(long, default_value_t = 1000)]
    accounts: usize,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct AccountInfo {
    owner_address: String,  // hex
    nonce:         i64,
    leaf_index:    i64,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    let endpoint = format!("{}/submit_tx", args.url.trim_end_matches('/'));
    let accounts_endpoint = format!("{}/accounts", args.url.trim_end_matches('/'));
    let client = Client::new();

    info!("Loading {} deterministic accounts...", args.accounts);
    let local_accounts = accounts::deterministic_accounts(args.accounts);
    info!("Local accounts ready");

    info!("Fetching current nonces from {accounts_endpoint}");
    let mut nonces: HashMap<[u8; 20], u64> = match fetch_nonces(&client, &accounts_endpoint).await {
        Ok(map) => {
            info!("Fetched nonces for {} accounts", map.len());
            map
        }
        Err(e) => {
            warn!("Couldn't fetch nonces ({e}); defaulting all to 0");
            local_accounts.iter().map(|a| (a.owner_address, 0u64)).collect()
        }
    };

    for a in &local_accounts {
        nonces.entry(a.owner_address).or_insert(0);
    }

    if args.continuous {
        info!(url = %endpoint, amount = args.amount, "Starting test client (continuous mode)");
        let mut cycle = 0u64;
        loop {
            cycle += 1;
            info!(cycle, "sending a batch of 20 transactions");
            run_one_cycle(&client, &endpoint, &accounts_endpoint, &local_accounts, &mut nonces, args.amount).await;
            let sleep_secs = rand::thread_rng().gen_range(10..=30);
            info!(cycle, sleep_secs, "20 txs done — sleeping {sleep_secs}s");
            tokio::time::sleep(tokio::time::Duration::from_secs(sleep_secs)).await;
        }
    } else {
        info!(
            count = args.count, url = %endpoint, amount = args.amount,
            "Starting test client (fixed-count mode)"
        );
        let mut ok = 0u64;
        let mut err = 0u64;
        for _ in 0..args.count {
            if send_random_tx(&client, &endpoint, &local_accounts, &mut nonces, args.amount).await.is_ok() {
                ok += 1;
            } else {
                err += 1;
            }
            if args.delay_ms > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(args.delay_ms)).await;
            }
        }
        info!(sent = args.count, ok, errors = err, "Done");
    }
}

async fn run_one_cycle(
    client: &Client,
    endpoint: &str,
    accounts_endpoint: &str,
    local: &[accounts::Account],
    nonces: &mut HashMap<[u8; 20], u64>,
    amount: u128,
) {
    if let Ok(fresh) = fetch_nonces(client, accounts_endpoint).await {
        for (addr, n) in fresh { nonces.insert(addr, n); }
    }
    for _ in 0..20 {
        let _ = send_random_tx(client, endpoint, local, nonces, amount).await;
    }
}

async fn send_random_tx(
    client: &Client,
    endpoint: &str,
    local: &[accounts::Account],
    nonces: &mut HashMap<[u8; 20], u64>,
    amount: u128,
) -> Result<(), ()> {
    let mut rng = rand::thread_rng();
    let n = local.len();
    let s_idx = rng.gen_range(0..n);
    let mut r_idx = rng.gen_range(0..n);
    while r_idx == s_idx { r_idx = rng.gen_range(0..n); }
    drop(rng);

    let sender = &local[s_idx];
    let recipient = &local[r_idx];
    let nonce = *nonces.get(&sender.owner_address).unwrap_or(&0);

    let tx = build_signed_tx(sender, recipient, amount, nonce).map_err(|e| {
        error!("signing failed: {e}");
    })?;
    let tx_hash_display = tx.tx_hash.to_string();

    match client.post(endpoint).json(&tx).send().await {
        Ok(resp) if resp.status().is_success() => {
            info!(
                sender = %hex::encode(sender.owner_address),
                nonce, tx_hash = %tx_hash_display,
                "tx accepted"
            );
            nonces.insert(sender.owner_address, nonce + 1);
            Ok(())
        }
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            warn!(
                sender = %hex::encode(sender.owner_address),
                nonce, %status, %body, "tx rejected"
            );
            Err(())
        }
        Err(e) => {
            error!(nonce, error = %e, "request failed");
            Err(())
        }
    }
}

fn build_signed_tx(
    sender: &accounts::Account,
    recipient: &accounts::Account,
    amount: u128,
    nonce: u64,
) -> anyhow::Result<L2Transaction> {
    let (r8x, r8y, s) = accounts::sign_tx(
        sender,
        &sender.l2_address,
        &recipient.l2_address,
        amount, nonce,
    )?;

    let tx_hash_bytes = accounts::compute_msg_hash(
        &sender.l2_address, &recipient.l2_address, amount, nonce,
    );

    Ok(L2Transaction {
        tx_hash:   PoseidonHash(tx_hash_bytes),
        sender:    sender.owner_address.into(),
        recipient: recipient.owner_address.into(),
        amount,
        fee: 0,
        nonce,
        status: L2TransactionStatus::Pending,
        batch_id: None,
        sig_r8_x: r8x,
        sig_r8_y: r8y,
        sig_s:    s,
    })
}

async fn fetch_nonces(
    client: &Client,
    accounts_endpoint: &str,
) -> anyhow::Result<HashMap<[u8; 20], u64>> {
    let resp = client.get(accounts_endpoint).send().await?
        .error_for_status()?
        .json::<Vec<AccountInfo>>().await?;

    let mut out = HashMap::with_capacity(resp.len());
    for r in resp {
        let bytes = hex::decode(&r.owner_address)?;
        if bytes.len() == 20 {
            let mut addr = [0u8; 20];
            addr.copy_from_slice(&bytes);
            out.insert(addr, r.nonce as u64);
        }
    }
    Ok(out)
}
