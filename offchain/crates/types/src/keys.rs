use ark_ec::Group;
use ark_ed_on_bn254::{EdwardsAffine, EdwardsProjective, Fr};
use ark_std::UniformRand;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

pub struct L2Keypair {
    pub secret_key: Fr,
    pub public_key: EdwardsAffine,
}

impl L2Keypair {
    pub fn from_eth_seed(seed: [u8; 32]) -> Self {
        let mut rng = ChaCha20Rng::from_seed(seed);
        let sk = Fr::rand(&mut rng);
        let pk = (EdwardsProjective::generator() * sk).into();
        Self {
            secret_key: sk,
            public_key: pk,
        }
    }
}