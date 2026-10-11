// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {ISP1Verifier} from "@sp1-contracts/ISP1Verifier.sol";

/// The public values committed by the EduRoll SP1 guest program.
/// Field order MUST match `PublicValuesStruct` in circuits/sp1/lib/src/lib.rs.
struct PublicValues {
    bytes32 oldRoot;
    bytes32 newRoot;
    bytes32 depositsRoot;
    bytes32 withdrawalsRoot;
    bytes32 batchDataHash;
    uint64 batchNumber;
}

/// @title RollupSP1
/// @notice SP1-verified variant of EduRoll's Rollup contract (parallel to the
///         Groth16 `src/Rollup.sol` at the repo root — the Circom pipeline is
///         untouched). One SP1 proof attests the full batch STF: EdDSA
///         signatures, Merkle updates, AND the DA commitment.
///
/// Compared to `src/Rollup.sol`:
///   * verification goes through the audited SP1 verifier gateway
///     (`ISP1Verifier.verifyProof(vkey, publicValues, proofBytes)`) instead of
///     a circuit-specific Groth16 verifier;
///   * `withdrawalsRoot` and `batchDataHash` are read from the PROOF's public
///     values instead of being operator-supplied — closing the
///     "DA not proof-bound" and "withdrawalsRoot operator-attested" gaps;
///   * `batchNumber` is part of the proven statement.
contract RollupSP1 {
    /// @notice The SP1 verifier (gateway or a specific version). Deployments:
    ///         https://github.com/succinctlabs/sp1-contracts/tree/main/contracts/deployments
    address public immutable verifier;

    /// @notice Verification key hash of the EduRoll STF guest program
    ///         (print with `cargo run --release --bin vkey`).
    bytes32 public immutable stfVKey;

    // Access control: mirrors src/Rollup.sol.
    address public owner;
    address public submitter;

    bytes32 public currentStateRoot;
    bytes32 public currentWithdrawalsRoot;
    bytes32 public currentDepositsRoot;
    uint64 public currentBatchNumber;

    event BatchSubmitted(uint64 indexed batchNumber, bytes32 newRoot);
    event BatchDataPosted(uint64 indexed batchNumber, bytes32 batchDataHash, bytes batchData);
    event SubmitterUpdated(address indexed newSubmitter);

    modifier onlySubmitter() {
        require(msg.sender == submitter, "Only submitter");
        _;
    }

    constructor(address _verifier, bytes32 _stfVKey, bytes32 _initialStateRoot, address _submitter) {
        require(_verifier != address(0), "Zero verifier");
        require(_submitter != address(0), "Zero submitter");
        verifier = _verifier;
        stfVKey = _stfVKey;
        currentStateRoot = _initialStateRoot;
        owner = msg.sender;
        submitter = _submitter;
        // currentBatchNumber defaults to 0; the first batch must carry 1.
    }

    /// @notice Rotate the authorised batch submitter. Owner-only.
    function setSubmitter(address _submitter) external {
        require(msg.sender == owner, "Only owner");
        require(_submitter != address(0), "Zero submitter");
        submitter = _submitter;
        emit SubmitterUpdated(_submitter);
    }

    /// @notice Verify an SP1 proof of one batch STF and finalise the batch.
    /// @param publicValues ABI-encoded `PublicValues` committed by the guest.
    /// @param proofBytes   The SP1 proof (Groth16- or PLONK-wrapped).
    /// @param batchData    The batch's transactions, posted as calldata DA.
    ///                     Bound INSIDE the proof via `batchDataHash` — an
    ///                     adversarial submitter can no longer post data that
    ///                     differs from what was proven.
    function submitBatch(
        bytes calldata publicValues,
        bytes calldata proofBytes,
        bytes calldata batchData
    ) external onlySubmitter {
        PublicValues memory pv = abi.decode(publicValues, (PublicValues));

        // CHECKS — same state-machine rules as src/Rollup.sol.
        require(pv.oldRoot == currentStateRoot, "Invalid old root");
        require(pv.batchNumber == currentBatchNumber + 1, "Batch number is invalid.");

        // DATA AVAILABILITY: the posted calldata must hash to the commitment
        // the GUEST computed inside the proof (not an operator-supplied one).
        require(keccak256(batchData) == pv.batchDataHash, "batchData hash mismatch");

        // Verify the proof of the whole batch STF. Reverts on failure.
        ISP1Verifier(verifier).verifyProof(stfVKey, publicValues, proofBytes);

        // EFFECTS
        currentStateRoot = pv.newRoot;
        currentWithdrawalsRoot = pv.withdrawalsRoot;
        currentDepositsRoot = pv.depositsRoot;
        currentBatchNumber = pv.batchNumber;

        emit BatchSubmitted(currentBatchNumber, currentStateRoot);
        emit BatchDataPosted(currentBatchNumber, pv.batchDataHash, batchData);
    }

    function stateRoot() external view returns (bytes32) {
        return currentStateRoot;
    }

    function batchNumber() external view returns (uint64) {
        return currentBatchNumber;
    }
}
