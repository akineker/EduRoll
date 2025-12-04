use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TxStatus {
    // Matches your SQL constraints
    #[serde(rename = "ACCEPTED_ON_L2")]
    AcceptedOnL2, 
    #[serde(rename = "ACCEPTED_ON_L1")]
    AcceptedOnL1,
    #[serde(rename = "FINALIZED")]
    Finalized,
}

// Implement Display for easy conversion to String for the DB
impl std::fmt::Display for TxStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TxStatus::AcceptedOnL2 => write!(f, "ACCEPTED_ON_L2"),
            TxStatus::AcceptedOnL1 => write!(f, "ACCEPTED_ON_L1"),
            TxStatus::Finalized => write!(f, "FINALIZED"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitTxResponse {
    pub tx_hash: String,
    pub status: TxStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchStatusResponse {
    pub batch_id: u64,
    pub status: TxStatus,
}