use types::{KeccakHash, crypto_utils};

pub const BATCH_SIZE: usize = 20;
#[derive(Debug, Clone)]
pub struct KeccakMerkleTree {
    pub leaves: Vec<KeccakHash>,
    pub layers: Vec<Vec<KeccakHash>>, 
}

impl KeccakMerkleTree {
    pub fn new(transactions: Vec<Vec<u8>>) -> Self {
        assert_eq!(
            transactions.len(),
            BATCH_SIZE,
            "Batch size must be exactly {} but received {}.",
            BATCH_SIZE,
            transactions.len()
        );

        let leaves: Vec<KeccakHash> = transactions
            .iter()
            .map(|tx| crypto_utils::calculate_keccak_hash(tx))
            .collect();

        let mut tree = Self {
            leaves: leaves.clone(),
            layers: vec![leaves],
        };
        tree.build_tree();
        tree
    }

    fn build_tree(&mut self) {
        let mut current_layer = self.layers[0].clone();

        while current_layer.len() > 1 {
            let mut next_layer = Vec::new();

            if current_layer.len() % 2 != 0 {
                let last = current_layer.last().unwrap().clone();
                current_layer.push(last);
            }

            for i in (0..current_layer.len()).step_by(2) {
                let left = current_layer[i].as_ref();
                let right = current_layer[i + 1].as_ref();
                
                // Concatenate the two 32-byte hashes into a 64-byte buffer
                let mut combined = Vec::with_capacity(64);
                combined.extend_from_slice(left);
                combined.extend_from_slice(right);
                
                // Hash the pair to move up to the next layer
                next_layer.push(crypto_utils::calculate_keccak_hash(&combined));
            }
            self.layers.push(next_layer.clone());
            current_layer = next_layer;
        }
    }

    pub fn get_root(&self) -> KeccakHash {
        self.layers.last().expect("Tree structure is empty")[0].clone()
    }

    pub fn get_proof(&self, index: usize) -> Vec<KeccakHash> {
        let mut proof = Vec::new();
        let mut current_index = index;

        for i in 0..self.layers.len() - 1 {
            let layer = &self.layers[i];
            let is_right_node = current_index % 2 != 0;
            let sibling_index = if is_right_node { current_index - 1 } else { current_index + 1 };

            if sibling_index < layer.len() {
                proof.push(layer[sibling_index].clone());
            }
            current_index /= 2;
        }
        proof
    }
}