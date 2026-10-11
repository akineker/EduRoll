//! Generate an EVM-verifiable (Groth16/PLONK-wrapped) proof of the EduRoll
//! STF and write a Solidity test fixture.
//!
//! ```shell
//! RUST_LOG=info cargo run --release --bin evm -- --system groth16
//! RUST_LOG=info cargo run --release --bin evm -- --system plonk
//! ```
//!
//! NOTE (16 GB machines): the Groth16/PLONK wrap is the RAM-heavy step. If it
//! OOMs locally, set SP1_PROVER=network + NETWORK_PRIVATE_KEY to use the
//! Succinct Prover Network instead — see circuits/sp1/TODO.md.

use alloy_sol_types::SolType;
use clap::{Parser, ValueEnum};
use eduroll_lib::PublicValuesStruct;
use serde::{Deserialize, Serialize};
use sp1_sdk::{
    blocking::{ProveRequest, Prover, ProverClient},
    include_elf, Elf, HashableKey, ProvingKey, SP1ProofWithPublicValues, SP1Stdin, SP1VerifyingKey,
};
use std::path::PathBuf;

#[path = "../demo_batch.rs"]
mod demo_batch;

const EDUROLL_ELF: Elf = include_elf!("eduroll-program");

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct EVMArgs {
    #[arg(long, default_value = "20")]
    txs: usize,
    #[arg(long, default_value = "32")]
    accounts: usize,
    #[arg(long, value_enum, default_value = "groth16")]
    system: ProofSystem,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum ProofSystem {
    Plonk,
    Groth16,
}

/// Fixture consumed by contracts/test — mirrors the RollupSP1 public values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SP1EduRollProofFixture {
    old_root: String,
    new_root: String,
    deposits_root: String,
    withdrawals_root: String,
    batch_data_hash: String,
    batch_number: u64,
    vkey: String,
    public_values: String,
    proof: String,
}

fn main() {
    sp1_sdk::utils::setup_logger();
    dotenv::dotenv().ok();

    let args = EVMArgs::parse();
    let client = ProverClient::from_env();
    let pk = client.setup(EDUROLL_ELF).expect("failed to setup elf");

    let input = demo_batch::build(args.accounts, args.txs);
    let mut stdin = SP1Stdin::new();
    stdin.write(&input);

    println!("batch: {} txs over {} accounts", args.txs, args.accounts);
    println!("Proof System: {:?}", args.system);

    let proof = match args.system {
        ProofSystem::Plonk => client.prove(&pk, stdin).plonk().run(),
        ProofSystem::Groth16 => client.prove(&pk, stdin).groth16().run(),
    }
    .expect("failed to generate proof");

    create_proof_fixture(&proof, pk.verifying_key(), args.system);
}

fn create_proof_fixture(
    proof: &SP1ProofWithPublicValues,
    vk: &SP1VerifyingKey,
    system: ProofSystem,
) {
    let bytes = proof.public_values.as_slice();
    let pv = PublicValuesStruct::abi_decode(bytes).unwrap();

    let fixture = SP1EduRollProofFixture {
        old_root: format!("0x{}", hex::encode(pv.oldRoot)),
        new_root: format!("0x{}", hex::encode(pv.newRoot)),
        deposits_root: format!("0x{}", hex::encode(pv.depositsRoot)),
        withdrawals_root: format!("0x{}", hex::encode(pv.withdrawalsRoot)),
        batch_data_hash: format!("0x{}", hex::encode(pv.batchDataHash)),
        batch_number: pv.batchNumber,
        vkey: vk.bytes32().to_string(),
        public_values: format!("0x{}", hex::encode(bytes)),
        proof: format!("0x{}", hex::encode(proof.bytes())),
    };

    println!("Verification Key: {}", fixture.vkey);
    println!("Public Values: {}", fixture.public_values);
    println!("Proof Bytes: {}", fixture.proof);

    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../contracts/src/fixtures");
    std::fs::create_dir_all(&fixture_path).expect("failed to create fixture path");
    std::fs::write(
        fixture_path.join(format!("rollup-{:?}-fixture.json", system).to_lowercase()),
        serde_json::to_string_pretty(&fixture).unwrap(),
    )
    .expect("failed to write fixture");
}
