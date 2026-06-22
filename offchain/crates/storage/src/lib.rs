pub mod connection;
pub mod prover;
pub mod sequencer;
pub mod submitter;
pub mod archiver;

// Re-export the database pool type for other crates' use 
pub use sqlx::PgPool;