// Register the modules
pub mod tx;
pub mod batch;
pub mod account; 
pub mod messages;
pub mod hashes;
pub mod merkle_proof;

// Re-export structs for easier use
pub use tx::L2Transaction;
pub use batch::{Batch, ZKProof};
pub use account::AccountState;
pub use messages::{SubmitTxResponse, BatchStatusResponse, TxStatus};
pub use hashes::Hash;
pub use merkle_proof::{MerkleProof, AccountProofResponse};