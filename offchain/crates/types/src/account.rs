use serde::{Serialize, Deserialize};
use ethers::types::Address;
use crate::PoseidonHash;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct AccountState {
    pub owner_address: Address,
    pub l2_address: PoseidonHash,
    pub l2_pubkey_x: [u8; 32], 
    pub l2_pubkey_y: [u8; 32],
    pub balance: u128,
    pub nonce: u64,
    pub leaf_index: u64,

}