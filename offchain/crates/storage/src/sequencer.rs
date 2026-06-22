use anyhow::Result;
use bigdecimal::BigDecimal;
use sqlx::PgPool;
use std::str::FromStr;
use types::{L2Transaction, L2TransactionStatus};

// Inserts a new pending transaction submitted by a user
pub async fn insert_transaction(pool: &PgPool, tx: &L2Transaction) -> Result<()> {
    let sig = {
        let mut buf = Vec::with_capacity(96);
        buf.extend_from_slice(&tx.sig_r8_x);
        buf.extend_from_slice(&tx.sig_r8_y);
        buf.extend_from_slice(&tx.sig_s);
        buf
    };

    let amount_bd = BigDecimal::from_str(&tx.amount.to_string())?;
    let fee_bd    = BigDecimal::from_str(&tx.fee.to_string())?;

    sqlx::query!(
        r#"
        INSERT INTO transactions
            (tx_hash, sender_address, recipient_address, amount, nonce, fee, signature, status)
        VALUES ($1, $2, $3, $4, $5, $6, $7, 'PENDING')
        "#,
        tx.tx_hash.as_ref() as &[u8],
        tx.sender.as_ref() as &[u8],
        tx.recipient.as_ref() as &[u8],
        amount_bd,
        tx.nonce as i64,
        fee_bd,
        sig.as_slice(),
    )
    .execute(pool)
    .await?;

    Ok(())
}

// Fetches the oldest N transactions that have not yet been batched, ordered FIFO
pub async fn fetch_pending_transactions(pool: &PgPool, limit: i64) -> Result<Vec<L2Transaction>> {
    let rows = sqlx::query!(
        r#"
        SELECT tx_hash, sender_address, recipient_address,
               amount::TEXT as "amount!", nonce, fee::TEXT as "fee!", signature
        FROM transactions
        WHERE status = 'PENDING' AND batch_id IS NULL
        ORDER BY created_at ASC
        LIMIT $1
        "#,
        limit
    )
    .fetch_all(pool)
    .await?;

    let txs = rows
        .into_iter()
        .map(|row| {
            let sig = row.signature;
            let mut r8x = [0u8; 32];
            let mut r8y = [0u8; 32];
            let mut s = [0u8; 32];
            if sig.len() == 96 {
                r8x.copy_from_slice(&sig[0..32]);
                r8y.copy_from_slice(&sig[32..64]);
                s.copy_from_slice(&sig[64..96]);
            }

            let mut sender = [0u8; 20];
            let mut recipient = [0u8; 20];
            if row.sender_address.len() == 20 {
                sender.copy_from_slice(&row.sender_address);
            }
            if row.recipient_address.len() == 20 {
                recipient.copy_from_slice(&row.recipient_address);
            }

            let mut tx_hash_bytes = [0u8; 32];
            if row.tx_hash.len() == 32 {
                tx_hash_bytes.copy_from_slice(&row.tx_hash);
            }

            L2Transaction {
                tx_hash: tx_hash_bytes.into(),
                sender: sender.into(),
                recipient: recipient.into(),
                amount: row.amount.to_string().parse::<u128>().unwrap_or(0),
                fee: row.fee.to_string().parse::<u128>().unwrap_or(0),
                nonce: row.nonce as u64,
                status: L2TransactionStatus::Pending,
                batch_id: None,
                sig_r8_x: r8x,
                sig_r8_y: r8y,
                sig_s: s,
            }
        })
        .collect();

    Ok(txs)
}

pub async fn write_batch_snapshot(
    db_tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    batch_id: i32,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO batch_snapshots
            (batch_id, leaf_index, owner_address, l2_pubkey_x, l2_pubkey_y,
             pre_balance, pre_nonce)
        SELECT $1, leaf_index, owner_address, l2_pubkey_x, l2_pubkey_y,
               balance, nonce
        FROM accounts
        "#,
        batch_id
    )
    .execute(&mut **db_tx)
    .await?;
    Ok(())
}
