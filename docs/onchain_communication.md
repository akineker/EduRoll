# On-Chain (Ethereum) Communication Flow

This document explains the roles of the Layer 1 smart contracts and how they interact to ensure the security and finality of the EduRoll ZK-Rollup. The simulation runs on a local Anvil test network.

The project follows a standard Foundry directory structure (created using the command below), separating core logic (`src/`), deployment scripts (`script/`), and tests (`test/`).

```bash
forge init PROJECT_NAME
```

### Folder Structure (`src/` and `script/`)

| Folder/File        | Path |  Responsibility & Purpose |
|--------------------|----------|------------------|
| `Deploy.s.sol`     | `script/` | Deploys all core contracts (`MockToken`, `Verifier`, `BridgeERC20`, `Rollup`) and configures their initial linkage. |
| `Deposit.s.sol`    | `script/` | Simulates user interaction for depositing tokens into the `BridgeERC20.sol` contract. |
| `SubmitBatch.s.sol`| `script/` | Simulates the L2 Submitter service by calling the `Rollup.sol` contract's state update function. |
| `Withdraw.s.sol`   | `script/` | Simulates user withdrawals from the `Rollup.sol` contract. |
| `BridgeERC20.sol`  | `src/` | Manages custody of tokens on L1. Includes `deposit()` and secured `releaseFunds()` functions. |
| `Rollup.sol`       | `src/` | Manages the authoritative L2 `stateRoot` and coordinates ZK proof verification. |
| `Verifier.sol`     | `src/` | Performs fast on-chain cryptographic verification of the ZK-SNARK proof. |
| `IBridge.sol`      | `src/interfaces/` |  Defines the `releaseFunds` function signature used by `Rollup.sol` to command the bridge. |
| `IRollup.sol`      | `src/interfaces/` | Defines the public interface to the Rollup contract for external or off-chain callers. |
| `IVerifier.sol`    | `src/interfaces/` | Defines the **Groth16 verification** function signature used by `Rollup.sol` when interacting with the verifier. |
| `Bridge.t.sol`     | `test/` | Tests deposit and release logic, and access control for the `BridgeERC20.sol` contract. |
| `Rollup.t.sol`     | `test/` | Tests batch submission, proof verification, and `stateRoot` updates for the `Rollup.sol` contract. |

### Dependencies

| Dependency      | Component   | Purpose in the Project |
|-----------------|-------------|-------------------------|
| OpenZeppelin    | `IERC20`    | Provides the standard ERC20 interface used by the bridged token. |
| OpenZeppelin    | `SafeERC20` | Security wrapper used by `BridgeERC20.sol` for safe token transfers (`safeTransferFrom`, `safeTransfer`) and protection against non-standard tokens. |
| Foundry         | `anvil`     | A free, fast Ethereum node for local testing and debugging of L1 contract interactions. |



## Communication (Operational) Flow

The core security of the ZK-Rollup relies on the interaction between the Off-Chain Submitter, the Rollup contract, and the Verifier.

1.  **Submission Trigger:**
    * The **Off-Chain Submitter** calls the **`Rollup.sol`** contract's `submitBatch()` function. Only the **authorised submitter** address may call it.
    * **Data Sent:** the ZK-SNARK proof (`a, b, c`), the three public inputs (`old_root`, `new_root`, `deposits_root`), and the batch's transactions as **L1 calldata** for DA, committed via `batchDataHash`.
    * *Security Consideration:* `batchDataHash` is supplied by the submitter and only checked against the posted blob. It is not one of the circuit's public signals, so the published calldata is not cryptographically bound to the proven state transition. Therefore, if the Submitter is adversarial, it may cause different batch data to be sent as the DA. Due to our hardware constraints, we will address this in the next iterations of the project.

2.  **Verification (The Security Check):**
    * The **`Rollup.sol`** contract checks `old_root == currentStateRoot`, the batch number, and `keccak256(batchData) == batchDataHash`, then calls `verifyProof()` on the **`Verifier.sol`** contract with the three public inputs.
    * The **`Verifier.sol`** runs the cryptographic check against the proof and public inputs.

3.  **State Finalisation:**
    * **If Verified Successfully:**
        * The `Verifier.sol` returns `true`.
        * The `Rollup.sol` updates `currentStateRoot`, `currentWithdrawalsRoot` and `currentDepositsRoot`, advances the batch number, and emits `BatchSubmitted` and the batch data (`BatchDataPosted`). This finalises the L2 batch on Ethereum's ledger.
    * **If Verification Fails:**
        * The `Verifier.sol` returns `false`.
        * The `Rollup.sol` transaction **reverts**, and the L2 state root on L1 remains unchanged, ensuring no invalid state can ever be committed.

4.  **Deposit Flow (L1 → L2):**
    * A **User** calls `deposit(amount, l2PubX, l2PubY)` on the **`BridgeERC20.sol`** contract, which locks the tokens, records a `PendingDeposit`, and emits a **`DepositQueued`** event.
    * The **Off-Chain Sequencer** indexes new deposits up to `pendingDepositCount()`, credits them on L2, and the circuit binds them through the public **`deposits_root`**.
    * **NOTE**: In the current version of the project, the L1 side works, but the honest pipeline does not yet credit deposits on L2, so the deposit path is not wired end-to-end yet. This will be addressed in the next iterations of the project.

5.  **Withdrawal Flow (The Pay-Out):**
    * A **User** calls `withdrawFunds()` on the **`Rollup.sol`** contract.
    * The `Rollup.sol` verifies the user's Merkle proof against the **`withdrawalsRoot`** committed in the last batch, then calls `releaseFunds()` on **`BridgeERC20.sol`** to pay out.
    * **NOTE**: `withdrawalsRoot` is currently operator-attested and not bound by the proof, and the honest pipeline does not yet populate it. Thus, withdrawals are not claimable end-to-end yet. This will be addressed in the next iterations of the project.