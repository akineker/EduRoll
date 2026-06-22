use crate::state::L2State;
use types::{L2Transaction, PoseidonHash};
use anyhow::{Result, anyhow};
use tracing::info;

pub fn execute_batch(
    mut state: L2State,
    txs: Vec<L2Transaction>,
) -> Result<(PoseidonHash, L2State)> {
    info!("Executing batch of {} transactions", txs.len());

    for tx in txs {
        apply_transaction(&mut state, &tx)?;
    }

    let new_root = state.tree.root;
    Ok((new_root, state))
}

fn apply_transaction(state: &mut L2State, tx: &L2Transaction) -> Result<()> {
    let sender_addr: [u8; 20] = tx.sender.0;
    let recipient_addr: [u8; 20] = tx.recipient.0;

    let sender_idx = state
        .account_indices
        .get(&sender_addr)
        .copied()
        .ok_or_else(|| anyhow!("Unknown sender {:?}", sender_addr))?;

    let sender = state
        .account_states
        .get_mut(&sender_addr)
        .ok_or_else(|| anyhow!("Missing state for sender {:?}", sender_addr))?;

    if sender.nonce != tx.nonce {
        return Err(anyhow!(
            "Nonce mismatch for sender: expected {}, got {}",
            sender.nonce, tx.nonce
        ));
    }
    if sender.balance < tx.amount {
        return Err(anyhow!(
            "Insufficient balance: sender has {}, tx needs {}",
            sender.balance, tx.amount
        ));
    }

    sender.balance -= tx.amount;
    sender.nonce   += 1;

    let updated_sender = sender.clone();
    state.tree.update_leaf_from_account(sender_idx, &updated_sender);

    let recipient_idx = state
        .account_indices
        .get(&recipient_addr)
        .copied()
        .ok_or_else(|| anyhow!(
            "Unknown recipient {:?} — recipient must be registered via deposit first",
            recipient_addr
        ))?;

    let recipient = state
        .account_states
        .get_mut(&recipient_addr)
        .ok_or_else(|| anyhow!("Missing state for recipient {:?}", recipient_addr))?;

    recipient.balance += tx.amount;

    let updated_recipient = recipient.clone();
    state.tree.update_leaf_from_account(recipient_idx, &updated_recipient);

    Ok(())
}
