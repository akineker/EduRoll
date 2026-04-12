 # EduRoll
EduRoll: End-to-End Ethereum zk-Rollup (L1 Contracts, Sequencer, Circuits) Built From Scratch

**EduRoll** is an end-to-end zk-rollup prototype built entirely from scratch, including:
- L1 smart contracts (`Rollup.sol`, `Bridge.sol`, `Verifier.sol`)
- L2 sequencer, prover, submitter, and archiver (Rust + Docker)
- ZK circuit (Groth16, Circom)
- PostgreSQL data layer
- Full Dockerised microservice environment

EduRoll demonstrates the architecture of a zk-rollup deployed on Ethereum:
state execution → batching → proving → L1 verification → state finalisation.

> NOTE: Currently in active development. Local testing uses an Anvil environment. 
> Testnet deployment will follow once core components are complete.

---

## 📚 System Architecture (High-Level)


<a href="./docs/assets/diagrams/SystemOverview.png">
  <img src="./docs/assets/diagrams/SystemOverview.png" width="600">
</a>



---

## System Components (Overview)

See **[`/docs`](./docs/README.md)** for the full explanation.

### **Layer 1 (Ethereum)**  
- **Rollup.sol** — Stores canonical L2 state root and validates batch proofs  
- **Verifier.sol** — Auto-generated zkSNARK verifier contract  
- **Bridge.sol** — Handles deposits and withdrawals between L1 ↔ L2 
- **`Data Availability (DA)`** — Data availability refers to the guarantee that transaction data published by the sequencer is accessible to anyone who needs to verify or reconstruct L2 state. There are multiple approaches:
  - **Ethereum calldata** — Posting data directly to L1 as calldata is the most secure option but is the most expensive.
  - **EIP-4844 blobs** — Cheaper data channel for rollups via blob-carrying transactions. However, the data is pruned later on.
  - **External DA layers** (e.g. Celestia, EigenDA) — Offload DA entirely offchain for maximum cost reduction, at the cost of weaker security assumptions.


### **Layer 2 (Off-Chain Services)**  
- **Sequencer** — Validates, orders, and executes L2 transactions; creates batches  
- **Prover** — Generates Groth16 proofs for each batch  (A batch is accepted to consist of 100 transactions)
- **Submitter** — Submits proven batches to `Rollup.sol`  
- **Archiver** — Indexes txs and batches for external queries  
- **Persistent DB (PostgreSQL)** — Central coordination layer for all off-chain components  

- **`Decentralisation`** — The single Sequencer is a centralisation risk as it represents a single point of failure for both liveness and censorship. Decentralised Sequencer Sets coordinated via PoS or PoA committees can address this concern. However, this project intentionally uses a single sequencer as a simplification appropriate for a local research prototype.

### **Client**
- **Test Client** — Generates signed L2 transactions and sends them to the Sequencer RPC

---

## Running the Project
**Note:** A Makefile will be added once the L1 and L2 development is complete.  
During development, you can use the commands below to run the environment.

Full setup instructions (Docker, Rust services, circuits, keys, and L1 deployment)  
will be added as the implementation progresses.

#### Standard Docker
```bash
    docker-compose up --build -d
```
#### Using Colima (Lightweight Docker Alternative)
If you have [colima](https://github.com/abiosoft/colima),  a lightweight Docker alternative suitable for low-resource systems, start it first:

```bash
    colima start
```

OR to further limit CPU and memory usage (optional) :
```bash
    colima start --cpu 6 --memory 10 --runtime docker
```

Then run the project:

```bash
    COMPOSE_PARALLEL_LIMIT=1 docker-compose up --build -d
```
#### Checking the system


```bash
  docker-compose logs submitter
```

Expected output:

```bash
  Submitter is here!
```

---

## Performance & Scalability Analysis
The performance of a ZK-Rollup is defined by the trade-off between **Prover Latency** (proof generation time) and **Amortised L1 Gas Costs** (cost per transaction on Ethereum). Detailed architectural reasoning and academic justifications are provided in the [Full Documentation](./docs/README.md).

`Batch Size Strategy`
EduRoll is configured with a default batch size of 100 transactions.

**L2 Prover Latency:** A batch size of 100 allows for proof generation without excessive memory overhead.

**L1 Amortisation:** While larger batches increase prover time, they significantly reduce the cost per user.

---

## Benchmarks *(Results Pending — Testnet Deployment)*
The following table defines the technical performance metrics that will be evaluated upon testnet deployment.

| Metric | EduRoll | Metric Description |
|---|---|---|
| Circuit Constraints | [Pending] | Total R1CS constraints for the `transfer` circuit logic. |
| Witness Generation | [Pending] | Time to calculate the witness for a batch. |
| Proof Generation Time | [Pending] | Total time for the `Prover` to generate a Groth16 ZK-SNARK. |
| Verification Gas Cost | [Pending] | The cost of running the `verifyProof()` function on L1. |
| Finality (Soft) | [Pending] | The time for the `Sequencer` to acknowledge and store the batch. |
| Finality (Hard) | [Pending] | Time until the proof is verified and the batch is finalised on-chain (Ethereum). |
