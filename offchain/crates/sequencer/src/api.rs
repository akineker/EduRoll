use anyhow::{anyhow, Result};
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use sqlx::PgPool;
use tracing::{error, info, warn};
use types::{L2Transaction, L2TransactionStatus, PoseidonHash, SubmitTxResponse};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
}

pub async fn start_api_server(db_pool: PgPool) -> Result<()> {
    let app_state = AppState { db_pool };

    let app = Router::new()
        .route("/submit_tx", post(submit_transaction_handler))
        .route("/accounts",  get(list_accounts_handler))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:9001").await?;
    info!("Sequencer API listening on 0.0.0.0:9001");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn submit_transaction_handler(
    State(state): State<AppState>,
    Json(tx): Json<L2Transaction>,
) -> Result<(StatusCode, Json<SubmitTxResponse>), (StatusCode, String)> {
    if let Err(e) = light_validate(&tx) {
        warn!("Tx rejected (light validation): {e}");
        return Err((StatusCode::BAD_REQUEST, e.to_string()));
    }

    if let Err(e) = validate_signature(&state.db_pool, &tx).await {
        warn!("Tx rejected (signature): {e}");
        return Err((StatusCode::BAD_REQUEST, format!("invalid signature: {e}")));
    }

    if let Err(e) = storage::sequencer::insert_transaction(&state.db_pool, &tx).await {
        error!("Failed to persist tx: {e:#}");
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "DB error".into()));
    }

    info!("Tx accepted: {}", tx.tx_hash);
    Ok((
        StatusCode::CREATED,
        Json(SubmitTxResponse {
            tx_hash: tx.tx_hash,
            status: L2TransactionStatus::Pending,
        }),
    ))
}

#[derive(Serialize)]
struct AccountInfo {
    owner_address: String,  // hex, no 0x
    l2_pubkey_x:   String,  // hex, no 0x
    l2_pubkey_y:   String,  // hex, no 0x
    balance:       String,  // decimal
    nonce:         i64,
    leaf_index:    i64,
}

async fn list_accounts_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<AccountInfo>>, (StatusCode, String)> {
    let rows = sqlx::query!(
        r#"
        SELECT owner_address, l2_pubkey_x, l2_pubkey_y,
               balance::TEXT as "balance!", nonce, leaf_index
        FROM accounts
        ORDER BY leaf_index ASC
        "#
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let out = rows.into_iter().map(|r| AccountInfo {
        owner_address: hex::encode(&r.owner_address),
        l2_pubkey_x:   hex::encode(&r.l2_pubkey_x),
        l2_pubkey_y:   hex::encode(&r.l2_pubkey_y),
        balance:       r.balance,
        nonce:         r.nonce,
        leaf_index:    r.leaf_index,
    }).collect();

    Ok(Json(out))
}

// Stateless validation.
fn light_validate(tx: &L2Transaction) -> Result<()> {
    if tx.amount == 0 {
        return Err(anyhow!("amount must be > 0"));
    }
    if tx.tx_hash == PoseidonHash::default() {
        return Err(anyhow!("tx_hash must be non-zero"));
    }
    Ok(())
}

async fn validate_signature(pool: &PgPool, tx: &L2Transaction) -> Result<()> {
    let sender_bytes = tx.sender.0;
    let recipient_bytes = tx.recipient.0;

    let (sender_pkx, sender_pky) = fetch_pubkey(pool, &sender_bytes).await?
        .ok_or_else(|| anyhow!("sender {:?} not registered", sender_bytes))?;
    let (recv_pkx, recv_pky) = fetch_pubkey(pool, &recipient_bytes).await?
        .ok_or_else(|| anyhow!("recipient {:?} not registered", recipient_bytes))?;

    let sender_l2 = accounts::compute_l2_address(&sender_pkx, &sender_pky);
    let recv_l2   = accounts::compute_l2_address(&recv_pkx, &recv_pky);

    let ok = accounts::verify_tx(
        &sender_pkx, &sender_pky,
        &sender_l2, &recv_l2,
        tx.amount, tx.nonce,
        &tx.sig_r8_x, &tx.sig_r8_y, &tx.sig_s,
    );
    if !ok {
        return Err(anyhow!("EdDSA-Poseidon signature did not verify"));
    }
    Ok(())
}

async fn fetch_pubkey(pool: &PgPool, owner: &[u8; 20]) -> Result<Option<([u8; 32], [u8; 32])>> {
    let row = sqlx::query!(
        "SELECT l2_pubkey_x, l2_pubkey_y FROM accounts WHERE owner_address = $1",
        owner.as_slice() as &[u8],
    )
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else { return Ok(None) };
    let mut pkx = [0u8; 32];
    let mut pky = [0u8; 32];
    if row.l2_pubkey_x.len() == 32 { pkx.copy_from_slice(&row.l2_pubkey_x); }
    if row.l2_pubkey_y.len() == 32 { pky.copy_from_slice(&row.l2_pubkey_y); }
    Ok(Some((pkx, pky)))
}
