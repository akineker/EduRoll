use serde::{Serialize, Deserialize};

use crate::tx::L2TransactionStatus;
use crate::batch::BatchStatus;
use crate::hashes::PoseidonHash;

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitTxResponse {
    pub tx_hash: PoseidonHash,
    pub status: L2TransactionStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchStatusResponse {
    pub batch_id: i32,
    pub status: BatchStatus,
}
