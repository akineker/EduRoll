pragma circom 2.0.0;

// Circom Circuit Design
/* EDDSA signatures with 100 transactions state transition

    Proves 100 valid L2 transfers using EdDSA (Baby Jubjub) signatures over a Sparse Merkle Tree of depth 20.
    
    Public inputs  : old_root, new_root
    
    Private witness: transactions, account states, Merkle proofs, signatures
*/
include "node_modules/circomlib/circuits/eddsaPoseidon.circom";
include "node_modules/circomlib/circuits/poseidon.circom";
include "node_modules/circomlib/circuits/comparators.circom";
include "node_modules/circomlib/circuits/mux1.circom";
include "node_modules/circomlib/circuits/bitify.circom";

// COMPONENTS
// Component1:Merkle Proof - Verify old leaf
template MerkleProof(depth){
    signal output root;
    signal input leaf;
    signal input pathElements[depth];
    signal input pathIndices[depth];

    component left_sel[depth];
    component right_sel[depth];
    component hashers[depth];

    signal hashes[depth+1];
    hashes[0] <== leaf; 

    for (var i = 0; i < depth; i++) {

        // Mux1: selects between two values based on a binary selector
        // LEFT input to Poseidon
        left_sel[i] = Mux1();
        left_sel[i].c[0] <== hashes[i];
        left_sel[i].c[1] <== pathElements[i];
        left_sel[i].s    <== pathIndices[i];

        // RIGHT input to Poseidon
        right_sel[i] = Mux1();
        right_sel[i].c[0] <== pathElements[i];
        right_sel[i].c[1] <== hashes[i];
        right_sel[i].s    <== pathIndices[i];

        // Hash the ordered pair of children to get the parent
        hashers[i] = Poseidon(2);
        hashers[i].inputs[0] <== left_sel[i].out;
        hashers[i].inputs[1] <== right_sel[i].out;

        hashes[i + 1] <== hashers[i].out;
    }

    root <== hashes[depth];
}

//Component2: MerkleUpdate - Compute new root 
template MerkleUpdate(depth) {
    signal input old_leaf;
    signal input new_leaf;
    signal input pathElements[depth];
    signal input pathIndices[depth];
    signal input old_root;
    signal output new_root;

    // Step 1: prove old_leaf is in tree at old_root
    component verify_old = MerkleProof(depth);
    verify_old.leaf <== old_leaf;
    for (var i = 0; i < depth; i++) {
        verify_old.pathElements[i] <== pathElements[i];
        verify_old.pathIndices[i]  <== pathIndices[i];
    }
    verify_old.root === old_root;

    // Step 2: recompute root with new_leaf at the same path
    component compute_new = MerkleProof(depth);
    compute_new.leaf <== new_leaf;
    for (var i = 0; i < depth; i++) {
        compute_new.pathElements[i] <== pathElements[i];
        compute_new.pathIndices[i]  <== pathIndices[i];
    }
    new_root <== compute_new.root;
}

//Component2_1: OptimisedMerkleUpdate - Compute new root 
template OptimizedMerkleUpdate(depth) {
    signal input old_leaf;
    signal input new_leaf;
    signal input pathElements[depth];
    signal input pathIndices[depth];
    signal input old_root;
    signal output new_root;

    // Old root components
    component old_left_sel[depth];
    component old_right_sel[depth];
    component old_hashers[depth];

    // New root components
    component new_left_sel[depth];
    component new_right_sel[depth];
    component new_hashers[depth];

    signal old_hashes[depth + 1];
    signal new_hashes[depth + 1];

    old_hashes[0] <== old_leaf;
    new_hashes[0] <== new_leaf;

    //Check old root and calculate new root
    for (var i = 0; i < depth; i++) {
        
        // Old root
        old_left_sel[i] = Mux1();
        old_left_sel[i].c[0] <== old_hashes[i];
        old_left_sel[i].c[1] <== pathElements[i];
        old_left_sel[i].s    <== pathIndices[i];

        old_right_sel[i] = Mux1();
        old_right_sel[i].c[0] <== pathElements[i];
        old_right_sel[i].c[1] <== old_hashes[i];
        old_right_sel[i].s    <== pathIndices[i];

        old_hashers[i] = Poseidon(2);
        old_hashers[i].inputs[0] <== old_left_sel[i].out;
        old_hashers[i].inputs[1] <== old_right_sel[i].out;
        old_hashes[i + 1] <== old_hashers[i].out;

        // New root
        new_left_sel[i] = Mux1();
        new_left_sel[i].c[0] <== new_hashes[i];
        new_left_sel[i].c[1] <== pathElements[i];
        new_left_sel[i].s    <== pathIndices[i];

        new_right_sel[i] = Mux1();
        new_right_sel[i].c[0] <== pathElements[i];
        new_right_sel[i].c[1] <== new_hashes[i];
        new_right_sel[i].s    <== pathIndices[i];

        new_hashers[i] = Poseidon(2);
        new_hashers[i].inputs[0] <== new_left_sel[i].out;
        new_hashers[i].inputs[1] <== new_right_sel[i].out;
        new_hashes[i + 1] <== new_hashers[i].out;
    }

    // Check old root 
    old_hashes[depth] === old_root;

    // Assign new root
    new_root <== new_hashes[depth];
}

//Component3: SingleStateTransition
template StateTransition(depth) {
    signal input current_root;
    signal output new_root;

    // Transaction data
    signal input tx_sender_address;
    signal input tx_receiver_address;
    signal input tx_amount;
    signal input tx_nonce;

    // Sender account state
    signal input sender_balance;
    signal input sender_nonce;
    signal input sender_path_elements[depth];
    signal input sender_path_indices[depth];

    // Receiver account state
    signal input receiver_balance;
    signal input receiver_nonce;
    signal input receiver_path_elements[depth];
    signal input receiver_path_indices[depth];

    // EdDSA signature (Baby Jubjub curve)
    signal input sig_R8x;
    signal input sig_R8y;
    signal input sig_S;
    signal input sender_pub_key_x;
    signal input sender_pub_key_y;


    // Step 1: Verify sender EDDSA signature
    component msg_hash = Poseidon(4);
    msg_hash.inputs[0] <== tx_sender_address;
    msg_hash.inputs[1] <== tx_receiver_address;
    msg_hash.inputs[2] <== tx_amount;
    msg_hash.inputs[3] <== tx_nonce;

    component eddsa = EdDSAPoseidonVerifier();
    eddsa.enabled <== 1;
    eddsa.Ax  <== sender_pub_key_x;
    eddsa.Ay  <== sender_pub_key_y;
    eddsa.R8x <== sig_R8x;
    eddsa.R8y <== sig_R8y;
    eddsa.S   <== sig_S;
    eddsa.M   <== msg_hash.out;

    // Security Check: Ensure key belongs to the transaction owner
    component pub_key_hasher = Poseidon(2);
    pub_key_hasher.inputs[0] <== sender_pub_key_x;
    pub_key_hasher.inputs[1] <== sender_pub_key_y;
    tx_sender_address === pub_key_hasher.out;

    // Step 2: Verify sender exists in Merkle tree
    component old_sender_leaf = Poseidon(3);
    old_sender_leaf.inputs[0] <== tx_sender_address;
    old_sender_leaf.inputs[1] <== sender_balance;
    old_sender_leaf.inputs[2] <== sender_nonce;

    //Security Check: Field Over/Underflow
    component amount_bounds_check = Num2Bits(128);
    amount_bounds_check.in <== tx_amount;

    //Security Check: Ensure that sender and receiver's are different
    component is_self_transfer = IsZero();
    is_self_transfer.in <== tx_sender_address - tx_receiver_address;
    is_self_transfer.out === 0;

    // Step 3: Verify sender has sufficient balance : 128bit maximum balance
    component sufficient_balance = GreaterEqThan(128);
    sufficient_balance.in[0] <== sender_balance;
    sufficient_balance.in[1] <== tx_amount;
    sufficient_balance.out === 1;

    // Step 4: Verify sender nonce is correct
    tx_nonce === sender_nonce;

    // Step 5: Verify receiver exists in Merkle tree
    component old_receiver_leaf = Poseidon(3);
    old_receiver_leaf.inputs[0] <== tx_receiver_address;
    old_receiver_leaf.inputs[1] <== receiver_balance;
    old_receiver_leaf.inputs[2] <== receiver_nonce;

    // Step 6: Update sender leaf
    component new_sender_leaf = Poseidon(3);
    new_sender_leaf.inputs[0] <== tx_sender_address;
    new_sender_leaf.inputs[1] <== sender_balance - tx_amount;  // deduct
    new_sender_leaf.inputs[2] <== sender_nonce + 1;            // increment nonce

    signal intermediate_root;  // root after sender update, before receiver update

    // component sender_update = MerkleUpdate(depth);
    component sender_update = OptimizedMerkleUpdate(depth);
    sender_update.old_leaf  <== old_sender_leaf.out;
    sender_update.new_leaf  <== new_sender_leaf.out;
    sender_update.old_root  <== current_root;
    for (var i = 0; i < depth; i++) {
        sender_update.pathElements[i] <== sender_path_elements[i];
        sender_update.pathIndices[i]  <== sender_path_indices[i];
    }
    intermediate_root <== sender_update.new_root;

    // Step 7: Update receiver leaf
    component new_receiver_leaf = Poseidon(3);
    new_receiver_leaf.inputs[0] <== tx_receiver_address;
    new_receiver_leaf.inputs[1] <== receiver_balance + tx_amount;  // credit
    new_receiver_leaf.inputs[2] <== receiver_nonce;                 // unchanged


    // Step 8: Compute new Merkle root
    // component receiver_update = MerkleUpdate(depth);
    component receiver_update = OptimizedMerkleUpdate(depth);
    receiver_update.old_leaf  <== old_receiver_leaf.out;
    receiver_update.new_leaf  <== new_receiver_leaf.out;
    receiver_update.old_root  <== intermediate_root;
    for (var i = 0; i < depth; i++) {
        receiver_update.pathElements[i] <== receiver_path_elements[i];
        receiver_update.pathIndices[i]  <== receiver_path_indices[i];
    }
    new_root <== receiver_update.new_root;
}

//Main Circuit
template EduRollup(depth, n_txs) {

    // Public inputs
    signal input old_root;
    signal input new_root;

    // Private: transaction fields
    signal input tx_sender_address[n_txs];
    signal input tx_receiver_address[n_txs];
    signal input tx_amount[n_txs];
    signal input tx_nonce[n_txs];

    // Private: sender account states
    signal input sender_balance[n_txs];
    signal input sender_nonce[n_txs];
    signal input sender_path_elements[n_txs][depth];
    signal input sender_path_indices[n_txs][depth];

    // Private: receiver account states 
    signal input receiver_balance[n_txs];
    signal input receiver_nonce[n_txs];
    signal input receiver_path_elements[n_txs][depth];
    signal input receiver_path_indices[n_txs][depth];

    // Private: EdDSA signatures 
    signal input sig_R8x[n_txs];
    signal input sig_R8y[n_txs];
    signal input sig_S[n_txs];
    signal input sender_pub_key_x[n_txs];
    signal input sender_pub_key_y[n_txs];

    // Intermediate roots chain
    //
    // intermediate_roots[0]     = old_root  (before any tx)
    // intermediate_roots[i+1]   = root after tx i
    // intermediate_roots[n_txs] = final root (must equal new_root)
    signal intermediate_roots[n_txs + 1];
    intermediate_roots[0] <== old_root;

    // Instantiate and chain 100 state transitions
    component transitions[n_txs];

    for (var i = 0; i < n_txs; i++) {
        transitions[i] = StateTransition(depth);

        // Each transition takes the previous output root as its input
        transitions[i].current_root <== intermediate_roots[i];

        // Wire transaction data
        transitions[i].tx_sender_address   <== tx_sender_address[i];
        transitions[i].tx_receiver_address <== tx_receiver_address[i];
        transitions[i].tx_amount           <== tx_amount[i];
        transitions[i].tx_nonce            <== tx_nonce[i];

        // Wire sender account state
        transitions[i].sender_balance <== sender_balance[i];
        transitions[i].sender_nonce   <== sender_nonce[i];
        for (var j = 0; j < depth; j++) {
            transitions[i].sender_path_elements[j] <== sender_path_elements[i][j];
            transitions[i].sender_path_indices[j]  <== sender_path_indices[i][j];
        }

        // Wire receiver account state
        transitions[i].receiver_balance <== receiver_balance[i];
        transitions[i].receiver_nonce   <== receiver_nonce[i];
        for (var j = 0; j < depth; j++) {
            transitions[i].receiver_path_elements[j] <== receiver_path_elements[i][j];
            transitions[i].receiver_path_indices[j]  <== receiver_path_indices[i][j];
        }

        // Wire signature
        transitions[i].sig_R8x          <== sig_R8x[i];
        transitions[i].sig_R8y          <== sig_R8y[i];
        transitions[i].sig_S            <== sig_S[i];
        transitions[i].sender_pub_key_x <== sender_pub_key_x[i];
        transitions[i].sender_pub_key_y <== sender_pub_key_y[i];

        // Store this transition's output root in the chain
        intermediate_roots[i + 1] <== transitions[i].new_root;
    }

    // Final constraint
    intermediate_roots[n_txs] === new_root;
}


//Entry Point
component main {public [old_root, new_root]} = EduRollup(20, 100);