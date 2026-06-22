// src/Rollup.sol

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;
import "./interfaces/IVerifier.sol";
import "./interfaces/IBridge.sol";

import "./interfaces/IRollup.sol";
import {MerkleProof} from "@openzeppelin/contracts/utils/cryptography/MerkleProof.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

contract Rollup is IRollup, ReentrancyGuard {
    // State variables
    IVerifier public immutable verifier;
    IBridge public immutable bridge;

    // Access control- only `submitter` may post batches and `owner` may rotate it.
    address public owner;
    address public submitter;

    bytes32 public currentStateRoot;
    bytes32 public currentWithdrawalsRoot;
    bytes32 public currentDepositsRoot;
    uint64 public currentBatchNumber;

    mapping (bytes32=>bool) public processedWithdrawals;
    mapping(address => uint256) public nonces;

    event SubmitterUpdated(address indexed newSubmitter);

    modifier onlySubmitter() {
        require(msg.sender == submitter, "Only submitter");
        _;
    }

    constructor(address _verifier, address _bridge, bytes32 _initialStateRoot, address _submitter) {
        require(_submitter != address(0), "Zero submitter");
        verifier = IVerifier(_verifier);
        bridge = IBridge(_bridge);
        currentStateRoot = _initialStateRoot;
        owner = msg.sender;
        submitter = _submitter;
    }

    // @notice Rotate the authorised batch submitter. Owner-only.
    function setSubmitter(address _submitter) external {
        require(msg.sender == owner, "Only owner");
        require(_submitter != address(0), "Zero submitter");
        submitter = _submitter;
        emit SubmitterUpdated(_submitter);
    }

    function submitBatch(
            uint256[2] calldata a,
            uint256[2][2] calldata b,
            uint256[2] calldata c,
            PublicInputs calldata input,
            bytes calldata batchData
    )external override onlySubmitter {
        // CHECKS
        require(input.oldRoot == currentStateRoot, "Invalid old root");
        require(input.batchNumber == currentBatchNumber + 1, "Batch number is invalid.");

        // DA
        require(keccak256(batchData) == input.batchDataHash, "batchData hash mismatch");

        // Pass the three signals the circuit exposes as public
        uint256[3] memory pubSignals;
        pubSignals[0] = uint256(input.oldRoot);
        pubSignals[1] = uint256(input.newRoot);
        pubSignals[2] = uint256(input.depositsRoot);

        //Verify the proof
        bool validProof = verifier.verifyProof(a, b, c, pubSignals);
        require(validProof, "ZK Proof verification failed.");

        // Update state
        currentStateRoot       = input.newRoot;
        currentWithdrawalsRoot = input.withdrawalsRoot;
        currentDepositsRoot    = input.depositsRoot;
        currentBatchNumber     = input.batchNumber;

        emit BatchSubmitted(currentBatchNumber, currentStateRoot);
        // Publish the batch data for DA (available in calldata + this log).
        emit BatchDataPosted(currentBatchNumber, input.batchDataHash, batchData);

    }

    // Withdraw
    function withdrawFunds(
            address token,
            uint256 amount,
            uint256 nonce,
            bytes32[] calldata merkleProof
        ) external override nonReentrant {
            // Prevent withdrawals before any batch has been submitted
            require(currentBatchNumber > 0, "No batches submitted yet");

            // Reject zero-value withdrawals
            require(amount > 0, "Invalid amount");

            // Check nonce order
            require(nonce == nonces[msg.sender],"Invalid nonce");

            // Hash the request
            bytes32 leaf = keccak256(abi.encodePacked(token, msg.sender, amount, nonce));

            //Check if already withdrawn
            require(!processedWithdrawals[leaf], "Withdrawal already processed!");

            //Verify Merkle proof against the withdrawals root committed in the last batch
            require(MerkleProof.verify(merkleProof, currentWithdrawalsRoot, leaf), "Invalid withdrawal proof");

            // Mark as processed
            processedWithdrawals[leaf] = true;
            //Increment nonce
            nonces[msg.sender]++;

            //Send funds
            bridge.releaseFunds(msg.sender, amount);

        }

    function stateRoot() external view returns (bytes32){
        return currentStateRoot;
    }
    function batchNumber() external view returns (uint64){
        return currentBatchNumber;
    }
}