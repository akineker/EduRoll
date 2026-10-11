//! Print the EduRoll guest program's verification key hash — the `stfVKey`
//! constructor argument for RollupSP1.sol. Changes whenever the guest program
//! (or its dependencies) change.

use sp1_sdk::{blocking::MockProver, blocking::Prover, include_elf, Elf, HashableKey, ProvingKey};

const EDUROLL_ELF: Elf = include_elf!("eduroll-program");

fn main() {
    let prover = MockProver::new();
    let pk = prover.setup(EDUROLL_ELF).expect("failed to setup elf");
    println!("{}", pk.verifying_key().bytes32());
}
