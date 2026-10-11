pub mod merkle_tree;
pub mod stf;
pub mod types;

pub use merkle_tree::PoseidonMerkleTree;
pub use stf::{encode_batch_data, execute_batch};
pub use types::{Account, AccountWitness, BatchInput, StfOutput, Tx};

pub const DEPTH: usize = 20;

pub const N_TXS: usize = 20;

pub const N_DEPOSITS: usize = 4;
