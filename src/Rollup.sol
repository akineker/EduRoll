// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;
import "./interfaces/IVerifier.sol";
import "./interfaces/IBridge.sol";

import "./interfaces/IRollup.sol";
import {MerkleProof} from "@openzeppelin/contracts/utils/cryptography/MerkleProof.sol";

contract Rollup is IRollup{
    // State variables
    IVerifier public immutable verifier;
    IBridge public immutable bridge;

    bytes32 public currentStateRoot;
    uint64 public currentBatchNumber;

    mapping (bytes32=>bool) public processedWithdrawals;
    mapping(address => uint256) public nonces;

    constructor(address _verifier, address _bridge, bytes32 _initialStateRoot) {
        verifier = IVerifier(_verifier);
        bridge = IBridge(_bridge);
        currentStateRoot = _initialStateRoot;
    }

    function submitBatch(
            uint256[2] calldata a,
            uint256[2][2] calldata b,
            uint256[2] calldata c,
            PublicInputs calldata input
    )external override{
        require(input.oldRoot == currentStateRoot, "Invalid old root");
        require(input.batchNumber == currentBatchNumber + 1, "Batch number is invalid.");

        //TODO: Change here if you are using a different circuit size
        uint256[] memory pubSignals = new uint256[](8); 
        pubSignals[0] = uint256(input.oldRoot);
        pubSignals[1] = uint256(input.newRoot);
        pubSignals[2] = uint256(input.withdrawalsRoot);
        pubSignals[3] = uint256(input.depositsRoot);
        pubSignals[4] = uint256(input.batchDataHash);
        pubSignals[5] = uint256(input.batchNumber);
        pubSignals[6] = uint256(input.l1BlockNumber);
        pubSignals[7] = uint256(input.circuitVersion);

        bool validProof = verifier.verifyProof(a, b,c, pubSignals);
        require(validProof, "ZK Proof verification failed.");

        currentStateRoot = input.newRoot;
        currentBatchNumber = input.batchNumber;

        emit BatchSubmitted(currentBatchNumber, currentStateRoot);

    }

    function withdrawFunds(
            address token,
            uint256 amount,
            uint256 nonce,
            bytes32[] calldata merkleProof
        ) external override{
            // Check nonce order
            require(nonce == nonces[msg.sender],"Invalid nonce");

            // Hash the request
            bytes32 leaf = keccak256(abi.encodePacked(token, msg.sender, amount, nonce));

            //Check if already withdrawn
            require(!processedWithdrawals[leaf], "Withdrawal already processed!");

            //Verify Merkle proof against the withdrawals root
            //TODO: Edit the code below to change the currentStateRoot with the withdrawals root
            require(MerkleProof.verify(merkleProof, currentStateRoot,leaf));


            processedWithdrawals[leaf] = true;
            nonces[msg.sender]++;
            bridge.releaseFunds(msg.sender, amount);

        }

    function stateRoot() external view returns (bytes32){
        return currentStateRoot;
    }
    function batchNumber() external view returns (uint64){
        return currentBatchNumber;
    }
}