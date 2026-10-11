#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy_sol_types::SolType;
use eduroll_lib::PublicValuesStruct;
use stf_core::{execute_batch, BatchInput};

pub fn main() {
    let input = sp1_zkvm::io::read::<BatchInput>();
    let out = execute_batch(&input);

    let public_values = PublicValuesStruct {
        oldRoot: out.old_root.into(),
        newRoot: out.new_root.into(),
        depositsRoot: out.deposits_root.into(),
        withdrawalsRoot: out.withdrawals_root.into(),
        batchDataHash: out.batch_data_hash.into(),
        batchNumber: input.batch_number,
    };
    sp1_zkvm::io::commit_slice(&PublicValuesStruct::abi_encode(&public_values));
}
