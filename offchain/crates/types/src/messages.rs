use serde::{Serialize, Deserialize};
use crate::L2Transaction;

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitTxResponse {
    pub tx_hash: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchStatusResponse {
    pub batch_id: u64,
    pub status: String,
}