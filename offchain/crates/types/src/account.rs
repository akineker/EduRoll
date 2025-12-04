use serde::{Serialize, Deserialize};
use ethers::types::U256;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct AccountState {
    pub balance: U256,
    pub nonce: u64,
    pub leaf_index: u64,
}