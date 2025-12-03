use serde::{Serialize, Deserialize};
use crate::L2Transaction;
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TxStatus {
    ACCEPTED_ON_L2,
    ACCEPTED_ON_L1,
    FINALIZED,
}

// Submitter: Sent message back to test client
#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitTxResponse {
    pub tx_hash: String,
    pub status: TxStatus,
}

// status:{ACCEPTED_ON_L2, ACCEPTED_ON_L1, FINALIZED}
//  Multiple containers
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchStatusResponse {
    pub batch_id: u64,
    pub status: TxStatus,
}