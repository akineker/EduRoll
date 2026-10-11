use alloy_sol_types::sol;

sol! {
    struct PublicValuesStruct {
        bytes32 oldRoot;
        bytes32 newRoot;
        bytes32 depositsRoot;
        bytes32 withdrawalsRoot;
        bytes32 batchDataHash;
        uint64 batchNumber;
    }
}
