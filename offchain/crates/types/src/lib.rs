// Register the modules
pub mod tx;
pub mod batch;
pub mod account; 
pub mod messages;
pub mod hashes;
pub mod merkle_proof;
pub mod crypto_utils;
pub mod keys;

// Re-export structs for easier use
pub use tx::{L2Transaction, L2TransactionStatus};
pub use batch::{Batch, BatchStatus, ZKProof};
pub use account::AccountState;
pub use messages::{SubmitTxResponse, BatchStatusResponse};
pub use hashes::{KeccakHash, PoseidonHash};
pub use merkle_proof::{MerkleProof, AccountProofResponse};
pub use keys::L2Keypair;