# EduRoll: SP1 Proving Backend

EduRoll's batch state-transition function, re-proven with SP1 to compare two proving paradigms: a general-purpose zkVM (SP1) against EduRoll's system-specific, hand-written circuit (Circom). This backend exists for that comparison, not as a production alternative, and the Circom Prover remains the system's prover. For clarity, the SP1 backend is called the **SP1 Prover** and the existing Circom/snarkjs backend the **Circom Prover**.

## Why compare against a zkVM?

The Circom Prover struggles to scale as the number of transactions in a batch grows, for two reasons:

- Operations that are not native field arithmetic (comparisons, range checks, bit decompositions) each expand into many R1CS constraints.
- Groth16 proving time and memory grow with the total constraint count (large multi-scalar multiplications and FFTs over the 254-bit BN254 scalar field).

On top of that, a Circom circuit is fixed at compile time for one batch size. Supporting more transactions per batch means a larger circuit, a fresh per-circuit trusted setup, and a new verifier contract. SP1 removes that rigidity as the same guest program proves any batch, so a larger batch required more work without a need for a new circuit.

## How SP1 differs?

SP1 takes a different approach. Its prover works over a small 31-bit prime field (KoalaBear, in SP1 Hypercube) that fits in a machine word, so its core field operations are cheaper than the Circom Prover's 254-bit arithmetic. For soundness it draws its random challenges from an extension of that field, which keeps the soundness error negligible. Its commitments are hash-based and transparent, so the core proof does not need trusted setup. However, a trusted setup re-enters only for the final proof, which is wrapped to Groth16 just for cheap on-chain verification.

There is also a conceptual difference in what is proven. The Circom Prover proves that one fixed circuit is satisfied by a witness. The SP1 Prover instead proves that a general-purpose RISC-V virtual machine executed the STF program correctly on the given inputs. Both proving systems attest the same statement: a valid batch state transition. However, the arithmetisation differs.

## What runs inside the zkVM?

The overall system design of EduRoll is unchanged from the system-overview diagram: the sequencer, submitter, archiver, PostgreSQL, and prover containers keep their roles. Only the proving backend changes, from Circom/snarkjs to SP1. In the current state the SP1 backend runs as standalone host scripts. Wiring SP1 into the dockerised prover is deferred, so the Circom path stays primary.

`program/` (the guest) replays one batch through `crates/stf-core`, enforcing the same rules that `circuits/circom/transfer_eddsa_20.circom` enforces: the EdDSA signature verifies, the sender address equals `Poseidon(pubkey)`, the nonce is sequential, the balance is sufficient, no self-transfer, and the Merkle root is chained through each update. The guest receives only the accounts the batch touches, each with its Merkle path against the pre-batch root (a sparse witness). The untouched accounts are represented by the sibling hashes on those paths, so the guest never sees them.

To do that, the guest needs the exact hash and signature scheme the rest of EduRoll already uses: the state tree is Poseidon-based and accounts are authenticated with EdDSA over BabyJubJub. So two syscall-accelerated crypto modules under `crates/zk-precompiles/src/` provide **Poseidon hashing** and **EdDSA verification**. These are because they must reproduce the sequencer's and circuit's primitives exactly, so the SP1 Merkle roots and signature acceptance come out byte-identical to the Circom pipeline.

Implementing these as SP1 AIR chips is an option, but the syscall-accelerated library approach is preferred here for two reasons: an AIR chip needs full proving servers plus rigorous soundness and completeness testing, and neither fits the available hardware. Instead, both modules build on a single, well-tested SP1 precompile (`uint256_mulmod`, exposed via the `sys_bigint` syscall), expressing their field arithmetic in terms of it.

## Advantages of SP1 Proving Backend

- **The DA commitment is bound in the proof.** The guest re-serialises the batch's txs with the submitter's exact encoding and commits `keccak256(batchData)` as a public output (`batchDataHash`), so the L1 calldata is cryptographically tied to the proven transition. Without this, a submitter could pair a valid proof with arbitrary DA bytes: the state root would still advance, but the published data would not match the transactions that produced it, which breaks reconstruction. The Circom Prover does not bind the DA inside the proof (its circuit exposes only `old_root, new_root, deposits_root`), so getting the same guarantee there would require committing the batch data inside the circuit, an expensive in-circuit keccak.
- **No per-circuit trusted setup.** The core proof is transparent. The EVM wrap reuses SP1's universal setup which is generated once, rather than a Groth16 Phase-2 that must be redone for every circuit change.
- **Not tied to a fixed batch size.** Supporting more transactions is a constant change in the guest, not a new circuit and trusted setup as it would be in Circom.
- **Cost is independent of the number of registered users.** The guest verifies only the touched accounts against `old_root` via their Merkle paths, so a batch over 1,000 accounts costs the same as one over a handful. Our measurement show that ~160M cycles for 20 random transfers whether the tree holds 32 or 1000 accounts.

## Bottlenecks of SP1 Proving Backend

- **Proving requires more capable machines:** A 20-tx batch is ~160M RISC-V cycles, which needs an estimated 40 GB or more of RAM for the core proof. So proving does not run on the 16 GB development machine. It needs a cloud/GPU server or the Succinct Prover Network. Only guest execution and the parity/contract tests run locally.

## Committed public values
`oldRoot, newRoot, depositsRoot, withdrawalsRoot, batchDataHash, batchNumber`
