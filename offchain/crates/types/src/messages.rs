use serde::{Serialize, Deserialize};
use crate::L2Transaction;

// Submitter: Sent message back to test client
#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitTxResponse {
    pub tx_hash: String,
    pub status: String,
}

// status:{SEQUENCED (Soft finality), PROVEN, FINALISED (Hard finality)}
//  Multiple containers
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchStatusResponse {
    pub batch_id: u64,
    pub status: String,
}