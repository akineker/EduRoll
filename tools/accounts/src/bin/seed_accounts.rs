use anyhow::{Context, Result};
use bigdecimal::BigDecimal;
use ethers::types::Address;
use merkle::PoseidonMerkleTree;
use sqlx::postgres::PgPoolOptions;
use std::str::FromStr;
use tracing::{info, warn};
use types::AccountState;

const N_ACCOUNTS: usize = 1000;
const INITIAL_BALANCE: u128 = 1_000_000_000;
const TREE_DEPTH: usize = 20;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL must be set")?;

    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .context("connect to Postgres")?;

    // Idempotency check.
    let count_row = sqlx::query!("SELECT COUNT(*)::BIGINT as \"count!\" FROM accounts")
        .fetch_one(&pool)
        .await
        .context("count accounts")?;
    if count_row.count >= N_ACCOUNTS as i64 {
        info!("accounts table already has {} rows — skipping seed", count_row.count);
        let root = compute_genesis_root(&pool).await?;
        println!("GENESIS_ROOT=0x{}", hex::encode(root));
        return Ok(());
    }

    if count_row.count > 0 {
        warn!(
            "accounts table has {} rows (< {}) — wiping for clean seed",
            count_row.count, N_ACCOUNTS
        );
        let mut tx = pool.begin().await?;
        sqlx::query!("DELETE FROM batch_snapshots").execute(&mut *tx).await?;
        sqlx::query!("DELETE FROM transactions").execute(&mut *tx).await?;
        sqlx::query!("DELETE FROM batches").execute(&mut *tx).await?;
        sqlx::query!("DELETE FROM accounts").execute(&mut *tx).await?;
        tx.commit().await?;
    }

    info!("Generating {N_ACCOUNTS} deterministic accounts...");
    let accounts = accounts::deterministic_accounts(N_ACCOUNTS);

    info!("Inserting accounts...");
    let mut tx = pool.begin().await?;
    let balance_bd = BigDecimal::from_str(&INITIAL_BALANCE.to_string())?;
    for acct in &accounts {
        sqlx::query!(
            r#"
            INSERT INTO accounts
                (owner_address, l2_pubkey_x, l2_pubkey_y, balance, nonce, leaf_index)
            VALUES ($1, $2, $3, $4, 0, $5)
            "#,
            acct.owner_address.as_slice() as &[u8],
            acct.pubkey_x.as_slice() as &[u8],
            acct.pubkey_y.as_slice() as &[u8],
            balance_bd,
            acct.index as i64,
        )
        .execute(&mut *tx)
        .await
        .with_context(|| format!("INSERT account {}", acct.index))?;
    }
    tx.commit().await?;
    info!("Inserted {N_ACCOUNTS} accounts");

    let root = compute_genesis_root(&pool).await?;
    println!("GENESIS_ROOT=0x{}", hex::encode(root));
    Ok(())
}

async fn compute_genesis_root(pool: &sqlx::PgPool) -> Result<[u8; 32]> {
    let rows = sqlx::query!(
        r#"
        SELECT owner_address, l2_pubkey_x, l2_pubkey_y,
               balance::TEXT as "balance!", nonce, leaf_index
        FROM accounts
        ORDER BY leaf_index ASC
        "#
    )
    .fetch_all(pool)
    .await?;

    let mut tree = PoseidonMerkleTree::new(TREE_DEPTH);
    for r in rows {
        let mut owner = [0u8; 20];
        if r.owner_address.len() == 20 { owner.copy_from_slice(&r.owner_address); }
        let mut pkx = [0u8; 32];
        let mut pky = [0u8; 32];
        if r.l2_pubkey_x.len() == 32 { pkx.copy_from_slice(&r.l2_pubkey_x); }
        if r.l2_pubkey_y.len() == 32 { pky.copy_from_slice(&r.l2_pubkey_y); }

        let account = AccountState {
            owner_address: Address::from(owner),
            l2_address: Default::default(), // merkle's hash_leaf recomputes
            l2_pubkey_x: pkx,
            l2_pubkey_y: pky,
            balance: r.balance.parse::<u128>().unwrap_or(0),
            nonce: r.nonce as u64,
            leaf_index: r.leaf_index as u64,
        };
        tree.update_leaf_from_account(r.leaf_index as u64, &account);
    }

    Ok(tree.root.0)
}
