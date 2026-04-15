// Circom Circuit Design
/* ECDSA signatures with 100 transactions state transition

    Proves 100 valid L2 transfers using ECDSA signatures over a Sparse Merkle Tree of depth 20 (up to 1,048,576 accounts).
    
    Public inputs  : old_root, new_root
    
    Private witness: transactions, account states, Merkle proofs, signatures
*/