use crate::hashes::{KeccakHash, PoseidonHash};

/// Cheap conversions from standard array
/// Keccak
impl From<[u8; 32]> for KeccakHash {
    fn from(bytes: [u8; 32]) -> Self {
        KeccakHash(bytes)
    }
}

impl AsRef<[u8; 32]> for KeccakHash {
    fn as_ref(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Poseidon
impl From<[u8; 32]> for PoseidonHash {
    fn from(bytes: [u8; 32]) -> Self {
        PoseidonHash(bytes)
    }
}

impl AsRef<[u8; 32]> for PoseidonHash {
    fn as_ref(&self) -> &[u8; 32] {
        &self.0
    }
}

/// TODO: Calculate the Keccak hash of the input data.
pub fn calculate_keccak_hash(data: &[u8]) -> KeccakHash {
    KeccakHash([0u8; 32])
}

/// TODO: Calculate the Poseidon hash of the input data.
pub fn calculate_poseidon_hash(data: &[u8]) -> PoseidonHash {
    PoseidonHash([0u8; 32]) 
}