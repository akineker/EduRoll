use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    pub owner_address: [u8; 20],
    pub l2_pubkey_x: [u8; 32],
    pub l2_pubkey_y: [u8; 32],
    pub balance: u128,
    pub nonce: u64,
    pub leaf_index: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tx {
    pub sender: [u8; 20],
    pub recipient: [u8; 20],
    pub amount: u128,
    pub nonce: u64,
    pub sig_r8_x: [u8; 32],
    pub sig_r8_y: [u8; 32],
    pub sig_s: [u8; 32],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountWitness {
    pub account: Account,
    pub siblings: Vec<[u8; 32]>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchInput {
    pub batch_number: u64,
    pub old_root: [u8; 32],
    pub accounts: Vec<AccountWitness>,
    pub txs: Vec<Tx>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StfOutput {
    pub old_root: [u8; 32],
    pub new_root: [u8; 32],
    pub deposits_root: [u8; 32],
    pub withdrawals_root: [u8; 32],
    pub batch_data_hash: [u8; 32],
}