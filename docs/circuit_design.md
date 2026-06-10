# EduRollup Circuit Architecture

This file presents information regarding the circuit design for EduRollup, considering the required system parameters and their value assignments for our system at hand.

## System Constraints & Parameter Selection
In a ZK-Rollup architecture, there is a trade-off between the system's throughput capacity and the hardware constraints of the prover. To ensure that the EduRollup can be operated by our system, the following analysis was conducted to determine the optimal parameter values.

### Constraint Arithmetisation
An R1CS *constraint* is a single equation of the form `A × B = C`, where `A`, `B` and `C` are each linear combinations of the circuit's signals. The circuit compiles into a system of such equations, and a valid proof certifies that the prover knows an assignment of all signals (the *witness*) that satisfies every constraint at once. The constraint count therefore measures circuit size and, directly, proving cost and memory.

The total number of Rank-1 Constraint System (R1CS) constraints is a function of the batch size $N$. For every single sender-to-receiver transfer, the circuit verifies the signature, proves the existence of two distinct accounts (sender and receiver), and computes two state updates.

The constraint formula is defined as:
$$C = N \times (C_{Signature} + C_{merkle} + C_{checks})$$

#### 1. Signature scheme
Two signature schemes were considered for EduRoll: EdDSA and ECDSA. Because of their underlying field arithmetic, EdDSA is more convenient for arithmetic circuits, whereas ECDSA is better suited to bitwise (CPU) operations and requires circuit-level optimisations to reduce its constraint count. The constraint counts are given below:

    EdDSA - Baby Jubjub (over BN254)     = ~4K constraints per signature
    ECDSA - secp256k1 (Ethereum native)  = ~1.5M constraints per signature

* **Decision:** EdDSA over the Baby Jubjub curve is preferred for our project because it requires fewer constraints, and therefore less RAM and less proof-generation time per state transition.

#### 2. Merkle Tree Depth
The depth of the fixed-depth Poseidon Merkle tree defines the maximum number of users the rollup can support (accounts occupy leaves by a sequential `leaf_index`), calculated by the formula:
$$NumberOfUsers = 2^{depth}$$

* **Decision:** A depth of `20` can support **1,048,576 user accounts**. For our project this number of users would be enough.

### 3. Prover RAM Comparison & Batch Size Scaling
The memory required to construct the polynomial matrices for the Groth16 witness and proof scales as the batch size ($N$) increases. Additionally, the choice of proving engine (the Node.js-based `snarkjs` versus the C++-based `rapidsnark`) impacts RAM consumption during the new state root generation process.

Furthermore, during the generation of the key and verifier contract, the Powers of Tau ceremony must be run with a capacity that satisfies the following condition against the total constraints ($C$):
$$2^{tau} \ge C$$

#### Key Generation RAM Formula
In a Groth16 ZK-Rollup, memory consumption is divided into two distinct lifecycle phases: the **Key Generation (`setup`) phase** and the **Proof Generation (`prover`) phase**.

The hardware estimates below are calculated against the parameters of the system:
* **Batch Size ($N$):** `20` Transactions
* **Tree Depth:** `20` (Supports 1,048,576 accounts)
* **Signature Scheme:** EdDSA over **BN128** (BN254)
* **Total Constraints ($C$):** `1,172,112`
* **Required Powers of Tau:** `21` ($2^{21}$ capacity)
* **`.ptau` File Size:** `2.42 GB`

#### Phase A: Key Generation RAM (The Setup Bottleneck)
The highest memory spike occurs during this phase. During the `snarkjs groth16 setup` command, the Node.js V8 engine must load both the `.ptau` and `.r1cs` files simultaneously into active memory to compute the matrix transpositions and Fast Fourier Transforms (FFTs) required to generate the final `.zkey`.

**Setup RAM Approximation Formula:**
$$RAM_{Setup} \approx (4 \times \text{Size}_{ptau}) + (2 \times \text{Size}_{r1cs})$$

* **Estimated RAM Required:** **~10.0 GB to ~12.0 GB**
* **System Impact:** This phase is heavily resource-intensive but acts as a one-time compilation cost. As it is executed once during system deployment, it does not impact the hardware constraints of the components during the simulation.

#### Phase B: Proof Generation RAM (The Operational Load)
Once the trusted setup is complete and the `.zkey` is distributed, the system can enter the operational phase. Generating the proofs requires less RAM, particularly when transitioning away from the default Node.js environment (snarkjs) to an optimised C++ execution engine (rapidsnark).

**Prover RAM Approximation Formulas:**
* `snarkjs` (Node.js): $\approx 4$ to $6\text{ GB}$ per $10^6\text{ constraints}$.
* `rapidsnark` (C++): $\approx 1.5$ to $2.5\text{ GB}$ per $10^6\text{ constraints}$.

##### Theoretical RAM Projections for Various Batch Sizes
Applying these standard baseline formulas, the following table projects the theoretical RAM requirements across various batch sizes (assuming a Merkle tree depth of 20). 

| Batch Size ($N$) | Total Constraints | Required Tau ($2^{tau}$) | Est. Setup RAM | `snarkjs` Prover RAM | `rapidsnark` Prover RAM |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **20 tx** | ~1,172,000 | 21 | ~10 to 12 GB | ~5 to 7 GB | ~2 to 3 GB |
| **100 tx** | ~5,354,600 | 23 | ~40 to 50 GB | ~21 to 32 GB | ~8 to 13 GB |
| **200 tx** | ~10,709,200 | 24 | ~80 to 100 GB | ~42 to 64 GB | ~16 to 26 GB |
| **500 tx** | ~26,773,000 | 25 | ~160 to 200 GB | ~107 to 160 GB | ~40 to 66 GB |
| **1,000 tx** | ~53,546,000 | 26 | ~320 to 400 GB | ~214 to 321 GB | ~80 to 134 GB |

*(Note: These figures are theoretical approximations based on the mathematical scaling of the Groth16 proving system. The 20-tx constraint count is the **measured** value (transfers + 4 deposit slots). The larger-batch rows are linear transfer-only projections, since the fixed ~100k-constraint deposit overhead does not scale with batch size.)*

**Architectural Decision:** Restricting the batch size to 20 transactions keeps proving within a 16 GB machine. The prover used is **`snarkjs` (Node.js)**, which needs roughly **~5 to 7 GB** for a 20-tx batch (see the table above). Switching to the **`rapidsnark` (C++)** engine would compress this to **~2 to 3 GB** and is the recommended optimisation for future versions of the project.


### 4. Financial Viability & Amortised Costs
While the batch size is heavily restricted by the prover's RAM limits, the primary economic driver of a ZK-Rollup is the amortisation of the L1 verification cost. A Groth16 proof verifies on L1 for a roughly fixed gas cost (~210-230k for `verifyProof`), independent of the number of transactions it proves. Larger batch sizes therefore reduce the per-transaction cost heavily, since this fixed verification cost is shared across more users. The downside is that larger batches require significantly more RAM to prove, so the batch size is a direct trade-off between per-transaction cost and proving hardware.

#### Decision
A batch size of **20 transactions** was chosen because it is the most our system can run while the whole simulation is live, because the simulation is not only the prover, but also the other containers (sequencer, submitter, archiver, PostgreSQL, test client) and the Anvil L1 node, all sharing a single 16 GB machine. The prover alone needs only ~5 to 7 GB with `snarkjs` (and ~2 to 3 GB with `rapidsnark`), but it shares that 16 GB with every other service, so larger batches risk exhausting memory during proof generation. Although switching the prover to `rapidsnark` (C++) would free headroom for larger batches, rapidsnark can only be used during proof generation, and the circuit compilation process for larger batch sizes would still have failed on our hardware.

---
## 5. Circom Circuit Component Architecture
The system is constructed using Circom to enforce strict state transition rules. Defined components are:

* **`MerkleProof(depth)`** *(reference — not compiled into the live circuit)*: Cryptographically verifies the existence of an old leaf within the current state root.
* **`MerkleUpdate(depth)`** *(reference — not compiled into the live circuit)*: Confirms the `old_root` matches the current state, and recomputes the root following a leaf change.

* **`OptimizedMerkleUpdate(depth)`:** *(The Merkle module actually compiled into the circuit.)* It merges the proof and update phases into a single pass to reduce constraint costs. Path indices are constrained to bits here, closing the standard Circom non-boolean-selector soundness gap.
    
    >**Non-boolean-selector soundness gap:** Each Merkle level uses a selector bit (`pathIndex`) to choose, via a `Mux`, whether the running hash is the left or right child. A `Mux` only behaves as a true selector when that bit is exactly 0 or 1. If it is left unconstrained, a malicious prover could assign it any field value, making the `Mux` output an arbitrary blend of the two children and forge a Merkle path. Forcing each selector to be boolean closes the gap.

      (pathIndices[i] * (pathIndices[i] - 1) === 0 satisfiable only when `pathIndices[i] ∈ {0, 1})
      
* **`Deposit(depth)`:** Credits one L1 deposit to an L2 account by onboarding a brand-new account into an empty leaf, or topping up an existing one, and emits a commitment folded into the public `deposits_root`.
* **`StateTransition(depth)`:** The core logic engine for a single transfer. It executes the following algorithmic flow:
  1. **Assert:** The EdDSA signature corresponds to the transaction sender.
  2. **Assert:** The sender exists within the current Merkle tree state.
  3. **Enforce:** The sender balance and transfer amount are range-checked to 128 bits (no overflow or underflow).
  4. **Enforce:** The sender and receiver are distinct accounts (prevents self-transfer exploitation).
  5. **Assert:** The sender maintains a sufficient token balance to execute the transfer.
  6. **Assert:** The sender's nonce is strictly sequential (replay attack protection).
  7. **Assert:** The receiver exists within the Merkle tree.
  8. **Compute:** The updated state leaves for both the sender and the receiver.
  9. **Compute:** The new intermediate Merkle root.
* **`EduRollup(depth, n_txs, n_deposits)`:** The top-level circuit. It first processes `n_deposits` deposit slots, then iterates `StateTransition` over the `n_txs` transfers, chaining the state root and binding the public `deposits_root`.

### Circuit Entry Point
The top-level component exposes the initial root, the final root, and `deposits_root` as public signals, keeping all transaction data private during verification. The entry point declared at the end of the file invokes `EduRollup` with the configured Merkle-tree depth, transfer batch size, and deposit-slot count:

```circom
component main {public [old_root, new_root, deposits_root]} = EduRollup(20, 20, 4); // (depth, n_txs, n_deposits)
```

## 6. Build & Deployment Pipeline
Once the circuit architecture is finalised, the following commands are executed to compile the constraints, perform the Trusted Setup ceremony, and generate the Layer 1 smart contract verifier.

### Option 1: Automatic Build (Recommended)

Run the automated pipeline, which performs every step in Option 2 below in a single command:

```bash
bash simulation-scripts/build-circuit.sh
```

### Option 2: Manual Deployment

Run the steps the script automates individually:

#### 1. Circuit Compilation

Compile the `.circom` file to generate the WebAssembly (WASM) execution environment and the R1CS constraint system.

> **Note for Apple Silicon (ARM64) Users:** The `-c` flag generates C++ witness calculation code for high-performance proving. If you experience `ld: symbol(s) not found` linker errors on macOS, remove the `-c` flag and rely on the WASM target (`transfer_eddsa_20_js`).

```bash
circom ./circuits/circom/transfer_eddsa_20.circom -l ./lib --wasm --r1cs --sym -c -o ./circuits/build
```

#### 2. Trusted Setup (Phase 1 & 2)
##### Option A: Download Precomputed Phase 1 (Recommended)

To bypass the heavily resource-intensive local Phase 1 ceremony, download the Hermez/Aptos Power 21 contribution:

```bash
curl -L https://github.com/aptos-labs/aptos-keyless-trusted-setup-contributions-may-2024/raw/main/powersOfTau28_hez_final_21.ptau \
  -o circuits/build/pot21_final.ptau
```

##### Option B: Local Ceremony Execution

If a deterministic local setup is required, execute the following:

```bash
# Initialise the Power 21 ceremony
snarkjs powersoftau new bn128 21 circuits/build/pot21_0000.ptau -v

# Contribute cryptographic randomness
snarkjs powersoftau contribute circuits/build/pot21_0000.ptau circuits/build/pot21_0001.ptau --name="First contribution" -v

# Prepare the ceremony for Phase 2 circuit-specific evaluations
snarkjs powersoftau prepare phase2 circuits/build/pot21_0001.ptau circuits/build/pot21_final.ptau -v
```

#### 3. Key Generation & Verifier Export
Generate the proving/verification keys and the L1 verifier, linking the compiled circuit to the Phase-1 baseplate.

```bash
# Phase-2 init: bind the circuit to the Phase-1 ptau
snarkjs groth16 setup circuits/build/transfer_eddsa_20.r1cs circuits/build/pot21_final.ptau circuits/build/transfer_eddsa_20_0000.zkey

# Phase-2 contribution
snarkjs zkey contribute circuits/build/transfer_eddsa_20_0000.zkey circuits/build/transfer_final.zkey --name="eduroll" -e="$(head -c 32 /dev/urandom | xxd -p)"

# Export the verification key (used by the off-chain prover)
snarkjs zkey export verificationkey circuits/build/transfer_final.zkey circuits/build/verification_key.json

# Export the L1 verifier FROM THE CONTRIBUTED key -> src/Verifier.sol
snarkjs zkey export solidityverifier circuits/build/transfer_final.zkey src/Verifier.sol
```


## 7. Compilation Results

Based on the selected configuration (**Depth = 20, Batch Size = 20**), the following file sizes and cryptographic parameters are generated upon successful compilation and setup:

| Resource Metric | Value |
| :--- | :--- |
| **Circuit Configuration** | 20 tx Batch, Depth 20 tree |
| **Required Powers of Tau** | `21` |
| **Final `.ptau` File Size** | `2.42 GB` |
| **Generated `.zkey` File Size** | `571.8 MB` |
| **Solidity Verifier Size** | `7 KB` |