use anyhow::Result;
use bigdecimal::BigDecimal;
use ethers::types::Address;
use execution::L2State;
use sqlx::PgPool;
use std::str::FromStr;
use types::AccountState;

pub async fn load_full_state(pool: &PgPool) -> Result<L2State> {
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

    let mut state = L2State::new();

    for row in rows {
        let mut addr = [0u8; 20];
        if row.owner_address.len() == 20 {
            addr.copy_from_slice(&row.owner_address);
        }

        let mut pk_x = [0u8; 32];
        let mut pk_y = [0u8; 32];
        if row.l2_pubkey_x.len() == 32 { pk_x.copy_from_slice(&row.l2_pubkey_x); }
        if row.l2_pubkey_y.len() == 32 { pk_y.copy_from_slice(&row.l2_pubkey_y); }

        let balance: u128 = row.balance.parse().unwrap_or(0);
        let nonce = row.nonce as u64;
        let leaf_index = row.leaf_index as u64;

        let account = AccountState {
            owner_address: Address::from(addr),
            l2_address: Default::default(),
            l2_pubkey_x: pk_x,
            l2_pubkey_y: pk_y,
            balance,
            nonce,
            leaf_index,
        };

        state.account_indices.insert(addr, leaf_index);
        state.tree.update_leaf_from_account(leaf_index, &account);
        state.account_states.insert(addr, account);
    }

    Ok(state)
}

pub async fn persist_state_changes(
    pool: &PgPool,
    db_tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    state: &L2State,
) -> Result<()> {
    let _ = pool;
    for (addr, account) in &state.account_states {
        let balance_bd = BigDecimal::from_str(&account.balance.to_string())?;
        sqlx::query!(
            r#"
            UPDATE accounts
            SET balance = $1,
                nonce   = $2
            WHERE owner_address = $3
            "#,
            balance_bd,
            account.nonce as i64,
            addr as &[u8],
        )
        .execute(&mut **db_tx)
        .await?;
    }
    Ok(())
}
