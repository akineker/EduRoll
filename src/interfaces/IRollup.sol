// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title IRollup
interface IRollup {
    struct PublicInputs {
        bytes32 oldRoot;
        bytes32 newRoot;
        bytes32 withdrawalsRoot;  // Merkle root over L2 withdrawal messages
        bytes32 depositsRoot;     // Merkle root over L1 deposits consumed
        bytes32 batchDataHash;    // keccak256 over canonical batch bytes
        uint64  batchNumber;
        uint64  l1BlockNumber;    // anchor for DA
        uint32  circuitVersion;
    }

    event BatchSubmitted(
        uint64 indexed batchNumber,
        bytes32 newRoot
    );

    // Read-only state accessors
    function stateRoot() external view returns (bytes32);
    function batchNumber() external view returns (uint64);

    //Incrementative nonce
    function nonces(address user) external view returns (uint256);

    // ZK Function: Submits a proof to update state
    function submitBatch(
            uint256[2] calldata a,
            uint256[2][2] calldata b,
            uint256[2] calldata c,
            PublicInputs calldata input
    ) external;

    /// User function to withdraw funds: Requires Merkle proof
    /// @notice  Nonce is added to mitigate Replay attacks as without a nonce value, a user only withdraw once for the same amount.
    function withdrawFunds(
        address token,
        uint256 amount,
        uint256 nonce,
        bytes32[] calldata merkleProof
    ) external;

}