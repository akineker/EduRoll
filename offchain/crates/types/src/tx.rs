use serde::{Serialize, Deserialize};
use ethers::types::{Address, Signature};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2Transaction {
    pub tx_hash: [u8; 32],
    pub sender: Address,
    pub recipient: Address,
    pub amount: u64,
    pub nonce: u64,
    pub signature: Signature,
}