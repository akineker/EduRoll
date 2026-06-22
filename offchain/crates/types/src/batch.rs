use serde::{Serialize, Deserialize};
use ark_serialize::{CanonicalSerialize, CanonicalDeserialize};
use crate::{L2Transaction, KeccakHash, PoseidonHash};

use ark_bn254::{G1Affine, G2Affine};

#[derive(Debug, Clone, sqlx::Type, Serialize, Deserialize, PartialEq)]
#[sqlx(type_name = "VARCHAR", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BatchStatus{
    PendingProof,
    Proven,
    SubmittedToL1,
    Finalized,
}
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Batch {
    pub batch_id: i32,

    #[sqlx(skip)] 
    pub transactions: Vec<L2Transaction>,
    pub pre_state_root_poseidon: PoseidonHash,
    pub post_state_root_poseidon: PoseidonHash,

    pub zk_proof: Option<Vec<u8>>,
    pub l1_tx_hash: Option<KeccakHash>,
    pub l1_block_number: Option<i64>,
    
    pub status: BatchStatus, 
}
#[derive(Debug, Clone, CanonicalSerialize, CanonicalDeserialize)]
pub struct ZKProof {
    pub a: G1Affine,
    pub b: G2Affine,
    pub c: G1Affine,
}