// SPDX-License-Identifier: MIT

pragma solidity ^0.8.20;

import {Script, console} from "forge-std/Script.sol";
import {IRollup} from "../src/interfaces/IRollup.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

/// @title Withdraw script
contract Withdraw is Script {
    function run() external {

        //Load Variables from .env
        uint256 userPrivateKey = vm.envUint("L1_PRIVATE_KEY");
        address rollupAddress = vm.envAddress("ROLLUP_ADDRESS");
        address tokenAddress = vm.envAddress("TOKEN_ADDRESS");

        // Example withdrawal data
        uint256 amountToWithdraw = 5 ether;

        // Prepare the Merkle Proof
        // TODO: Fetch this from your Archiver API when offchain completed.
        bytes32[] memory merkleProof = new bytes32[](1);
        merkleProof[0] = bytes32(0); // Dummy hash

        //Nonce generation
        uint256 nonce = IRollup(rollupAddress).nonces(msg.sender);
        
        // Execute Withdrawal
        vm.startBroadcast(userPrivateKey);

        // The Rollup verifies the proof and commands the Bridge to pay.
        IRollup(rollupAddress).withdrawFunds(
            tokenAddress,
            amountToWithdraw,
            nonce,
            merkleProof
        );

        console.log("Withdrawal initiated for", amountToWithdraw, "tokens");

        vm.stopBroadcast();
    }
}
