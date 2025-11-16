# EduRoll
EduRoll: End-to-End Ethereum zk-Rollup (L1 Contracts, Sequencer, Circuits) Built From Scratch

**EduRoll** is an end-to-end zk-rollup prototype built entirely from scratch, including:
- L1 smart contracts (`Rollup.sol`, `Bridge.sol`, `Verifier.sol`)
- L2 sequencer, prover, submitter, and archiver (Rust + Docker)
- ZK circuit (Groth16, Circom/Noir)
- PostgreSQL data layer
- Full Dockerised microservice environment

EduRoll demonstrates the architecture of modern zk-rollups deployed on Ethereum:
state execution → batching → proving → L1 verification → state finalisation.

---

## 📚 System Architecture (High-Level)


<a href="./docs/assets/diagrams/SystemOverview.png">
  <img src="./docs/assets/diagrams/SystemOverview.png" width="600">
</a>

---

## System Components (Overview)

See **[`/docs`](./docs/README.md)** for the full technical architecture.

### **Layer 1 (Ethereum)**  
- **`Rollup.sol`** — Stores canonical L2 state root and validates batch proofs  
- **`Verifier.sol`** — Auto-generated zkSNARK verifier contract  
- **`Bridge.sol`** — Handles deposits and withdrawals between L1 ↔ L2  


### **Layer 2 (Off-Chain Services)**  
- **Sequencer** — Validates, orders, and executes L2 transactions; creates batches  
- **Prover** — Generates Groth16 proofs for each batch  
- **Submitter** — Submits proven batches to `Rollup.sol`  
- **Archiver** — Indexes txs and batches for external queries  
- **Persistent DB (PostgreSQL)** — Central coordination layer for all off-chain components  



### **Client**
- **Test Client** — Generates signed L2 transactions and sends them to the Sequencer RPC

---

## Running the Project

Full setup instructions (Docker, Rust services, circuits, keys, L1 deployment)  
will be added as the implementation progresses.

```bash
    # Coming soon:
