use serde::{Serialize, Deserialize};
use ethers::types::{Address, Signature, U256};
use crate::Hash;
use crate::messages::TxStatus; 

/// FIXME: If multiple tokens used, add token_id below
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2Transaction {
    pub tx_hash: Hash,
    pub sender: Address,
    pub recipient: Address,
    pub amount: U256,
    pub nonce: u64,
    pub fee: U256,
    pub signature: Signature,
    pub status: Option<TxStatus>,
}