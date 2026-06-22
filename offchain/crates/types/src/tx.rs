use serde::{Serialize, Deserialize};
use ethers::types::{Address};
use crate::PoseidonHash;

#[derive(Debug, Clone, sqlx::Type, Serialize, Deserialize, PartialEq)]
#[sqlx(type_name = "VARCHAR", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum L2TransactionStatus {
    Pending,
    AcceptedOnL2,
    Finalized
}
// FIXME: If multiple tokens used, add token_id below
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2Transaction {
    pub tx_hash:PoseidonHash,
    pub sender: Address,
    pub recipient: Address,

    pub amount: u128, 
    pub fee: u128,
    pub nonce: u64,
    pub status: L2TransactionStatus,
    pub batch_id: Option<i32>,

    pub sig_r8_x: [u8; 32],
    pub sig_r8_y: [u8; 32],
    pub sig_s: [u8; 32],
}