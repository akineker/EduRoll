//! Execute or prove the EduRoll STF guest on a demo batch.
//!
//! ```shell
//! # Run WITHOUT proving — reports the cycle count (the zkVM "cost dial"):
//! RUST_LOG=info cargo run --release -- --execute
//!
//! # Generate + verify a core proof locally:
//! RUST_LOG=info cargo run --release -- --prove
//!
//! # Batch shape (defaults: 20 txs over 32 accounts, like the simulation):
//! RUST_LOG=info cargo run --release -- --execute --txs 20 --accounts 32
//! ```

use alloy_sol_types::SolType;
use clap::Parser;
use eduroll_lib::PublicValuesStruct;
use sp1_sdk::{
    blocking::{ProveRequest, Prover, ProverClient},
    include_elf, Elf, ProvingKey, SP1Stdin,
};

#[path = "../demo_batch.rs"]
mod demo_batch;

/// The compiled RISC-V ELF of the EduRoll guest program.
const EDUROLL_ELF: Elf = include_elf!("eduroll-program");

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    execute: bool,

    #[arg(long)]
    prove: bool,

    /// Transfers in the demo batch (max 20 — the circuit-parity batch size).
    #[arg(long, default_value = "20")]
    txs: usize,

    /// Accounts in the demo pre-state.
    #[arg(long, default_value = "32")]
    accounts: usize,
}

fn main() {
    sp1_sdk::utils::setup_logger();
    dotenv::dotenv().ok();

    let args = Args::parse();
    if args.execute == args.prove {
        eprintln!("Error: You must specify either --execute or --prove");
        std::process::exit(1);
    }

    let client = ProverClient::from_env();

    // Build a demo batch with REAL EdDSA signatures and write it to stdin.
    let input = demo_batch::build(args.accounts, args.txs);
    let mut stdin = SP1Stdin::new();
    stdin.write(&input);

    println!("batch: {} txs over {} accounts", args.txs, args.accounts);

    if args.execute {
        // Execute only: no proof, but exact cycle accounting — use this to
        // benchmark (e.g. syscall-accelerated Poseidon vs a naive build).
        let (output, report) = client.execute(EDUROLL_ELF, stdin).run().unwrap();
        println!("Program executed successfully.");

        let decoded = PublicValuesStruct::abi_decode(output.as_slice()).unwrap();
        println!("oldRoot:         0x{}", hex::encode(decoded.oldRoot));
        println!("newRoot:         0x{}", hex::encode(decoded.newRoot));
        println!("depositsRoot:    0x{}", hex::encode(decoded.depositsRoot));
        println!("withdrawalsRoot: 0x{}", hex::encode(decoded.withdrawalsRoot));
        println!("batchDataHash:   0x{}", hex::encode(decoded.batchDataHash));
        println!("batchNumber:     {}", decoded.batchNumber);

        // Sanity: the committed values must equal a native re-execution.
        let expected = stf_core::execute_batch(&input);
        assert_eq!(decoded.oldRoot.0, expected.old_root);
        assert_eq!(decoded.newRoot.0, expected.new_root);
        assert_eq!(decoded.batchDataHash.0, expected.batch_data_hash);
        println!("Guest output matches native re-execution.");

        println!("Total cycles: {}", report.total_instruction_count());
    } else {
        // Core proof: fast, local, NOT EVM-verifiable (use bin/evm for that).
        let pk = client.setup(EDUROLL_ELF).expect("failed to setup elf");
        let proof = client
            .prove(&pk, stdin)
            .run()
            .expect("failed to generate proof");
        println!("Successfully generated proof!");

        client
            .verify(&proof, pk.verifying_key(), None)
            .expect("failed to verify proof");
        println!("Successfully verified proof!");
    }
}
