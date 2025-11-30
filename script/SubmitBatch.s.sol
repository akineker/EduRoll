// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Script, console} from "forge-std/Script.sol";
import {IRollup} from "../src/interfaces/IRollup.sol";

import {Rollup} from "../src/Rollup.sol";

/// @title Submitbatch script
contract SubmitBatch is Script{
    function run() external{
        //Load Variables from .env
        uint256 deployerPrivateKey = vm.envUint("L1_PRIVATE_KEY");
        address rollupAddress = vm.envAddress("ROLLUP_ADDRESS");

        bytes32 currentRoot = Rollup(rollupAddress).stateRoot();
        uint256 nextBatchNum = Rollup(rollupAddress).batchNumber () + 1;

        //Broadcast Txs
        vm.startBroadcast(deployerPrivateKey);

        //Prepare Proof
        // TODO: Edit for Rust Prover - Currently dummy data for test purposes
        uint256[2] memory a = [uint256(1), uint256(2)];
        uint256[2][2] memory b = [[uint256(3), uint256(4)], [uint256(5), uint256(6)]];
        uint256[2] memory c = [uint256(7), uint256(8)];

        //Prepare Public Inputs - MOCK
        //TODO: Edit below for the ZKRollup tests
        IRollup.PublicInputs memory pubInputs = IRollup.PublicInputs({
            oldRoot: currentRoot,
            newRoot: bytes32(uint256(currentRoot)+1),
            withdrawalsRoot: bytes32(0),
            depositsRoot: bytes32(0),
            batchDataHash: bytes32(0),
            batchNumber: uint64(nextBatchNum),  
            l1BlockNumber: uint64(block.number),
            circuitVersion: 1
        });

        // Call Rollup contract
        Rollup(rollupAddress).submitBatch(a, b, c, pubInputs);

        console.log("Batch submitted");

        vm.stopBroadcast();
    }
}