use serde::{Serialize, Deserialize};
use crate::L2Transaction;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Batch {
    pub batch_id: u64,
    pub transactions: Vec<L2Transaction>,
    pub pre_state_root: [u8; 32], 
    pub post_state_root: [u8; 32],
    pub proof: Option<ZKProof>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZKProof {
    pub a: [u8; 32],
    pub b: [[u8; 32]; 2],
    pub c: [u8; 32],
}