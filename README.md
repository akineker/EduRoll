# EduRoll
EduRoll: End-to-End Ethereum zk-Rollup (L1 Contracts, L2 Components and Circuits) Built From Scratch

> **Research / educational prototype — NOT production-ready, NOT audited.**
> EduRoll demonstrates the end-to-end ZK-rollup proving pipeline. In its current version it omits security and infrastructure required for a real deployment: trustless L1 and L2 bridging, a multi-party trusted setup in Phase 2, decentralised sequencing, forced-inclusion/exit, rate-limiting, fees, and more.

## What it is

**EduRoll** is an end-to-end zk-rollup prototype built entirely from scratch:
- L1 smart contracts (`Rollup.sol`, `BridgeERC20.sol`, `Verifier.sol`)
- L2 sequencer, prover, submitter, and archiver (Rust + Docker)
- ZK transfer circuit (Groth16, Circom)
- PostgreSQL data layer and a test client (Dockerised)

It demonstrates the full lifecycle of a zk-rollup on Ethereum: **state execution -> batching -> proving -> L1 verification -> state finalisation** with each batch's transactions posted to L1 as **calldata** for data availability.

**Design decisions, component internals, the trusted-setup details, and the full list of limitations are in [`docs/README.md`](./docs/README.md).**

### EduRoll's Scope
**Implemented & working:** 
  1. EdDSA-signed L2 transfers
  2. Batching
  3. A witness built from the PostgreSQL batch data
  4. Groth16 validity proof
  5. On-chain verification
  6. State-root finalisation with calldata DA
  7. Single ERC-20 asset
  8. Fixed 20 tx/batch
  9. Authorised-submitter access control

**Illustrative and not yet wired end-to-end:**
1. The L1 and L2 **bridge** (deposits queue on L1 but are not credited on L2 and withdrawals are not claimable yet)
2. Live deposit ingestion
---

## System Architecture (high-level)

<a href="./docs/assets/diagrams/SystemOverview.png">
  <img src="./docs/assets/diagrams/SystemOverview.png" width="600">
</a>

- **L1 (Ethereum):**
  - `Rollup.sol:` Verifies the batch proof and DA commitment, holds the state root
  - `Verifier.sol:` snarkjs Groth16 verifier
  - `BridgeERC20.sol:` L1 custody
- **L2 (off-chain, Rust):**
  - **sequencer:** Validates, orders and executes txs from test_client into batches 
  - **prover:** Generates one Groth16 proof per batch
  - **submitter:** Posts proven batches and calldata DA to L1
  - **archiver:** Indexes for auditing purposes
- **PostgreSQL PersistentDB:** Acts as the async coordination hub between dockerised L2 components
- **test_client:** Generates EdDSA-signed transactions

See [`docs/README.md`](./docs/README.md) for the detailed component breakdown and operational flow.

---

## Test Environment & Running

**Development machine:** MacBook Pro, M1 Pro (10 Cores), 16 GB RAM.

[Colima](https://github.com/abiosoft/colima),a lightweight Docker alternative for low-resource systems, is used and pre-wired in the scripts. To use plain Docker, edit the scripts first.

```bash
# 1. Build the ZK artifacts (Compile circuit -> trusted setup -> export Verifier.sol)
bash ./simulation-scripts/build-circuit.sh

# 2. Fresh simulation start (Compile the project and docker containers, seed accounts, deploy L1 contracts, start all docker services)
bash ./simulation-scripts/run-fresh.sh

# 3. Pause the simulation
bash ./simulation-scripts/run-pause.sh

# 4. Resume (Keeps the DB + L1 state)
bash ./simulation-scripts/run-resume.sh

# 5. Remove (All docker containers, Rust and Forge compiles)
bash ./simulation-scripts/remove.sh
```

Inspect a service's logs:
```bash
docker-compose logs -f # all services
docker-compose logs -f CONTAINER_NAME # i.e: sequencer / prover / submitter / archiver / test_client / anvil 
```

---

## Results
  > *Measured locally on Anvil + M1 Pro(10 cores) with 16GB RAM*

Local runs of the 20-tx circuit (20 transfers + 4 deposit slots).

| Metric | EduRoll | Notes |
|---|---|---|
| Circuit constraints | ~628k | 20-tx circuit (transfers + deposits) |
| Witness generation | ~1.8 s | |
| Proof generation | ~80 s | snarkjs Groth16 |
| Trusted setup (Phase 2) | ~3 min | One-off `groth16 setup` and contribution |
| Proof Size | ~256 bytes | Groth16(A,B,C), constant |
| DA Calldata per Batch | ~3.12 KB | 20 txs posted to L1 |
| Gas cost submitBatch() including DA | ~327,418 | |
| Amortised L1 gas per Tx | ~16,371 | |


### L1 deployment gas (one-time, measured on Anvil)
| Contract | Gas |
|---|---|
| MockToken (test ERC-20) | 563,807 |
| Groth16Verifier | 366,287 |
| BridgeERC20 | 341,027 |
| Rollup | 859,975 |
---

## Future Work
1. **Decentralised sequencer network:** A PoS or PoA sequencer set removes the single point of failure for liveness and censorship.
2. **Recursive proof aggregation:** Multi-tier proving to compress larger tx sets into one L1 submission.
3. **Alternative data availability:** EIP-4844 blobs or modular DA (Celestia / EigenDA) to reduce DA cost.
4. **Trustless bridge:** Proof-bound deposits and withdrawals.
5. **Larger batch size:** C++ witness generation and an optimized prover (e.g., Rapidsnark) to manage increased constraint sizes and memory footprint.