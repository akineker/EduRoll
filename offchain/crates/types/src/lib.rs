// Register the modules
pub mod tx;
pub mod batch;
pub mod account; 

// Re-export structs for easier use
pub use tx::L2Transaction;
pub use batch::{Batch, ZKProof};
pub use account::AccountState;