// SPDX-License-Identifier: MIT

pragma solidity ^0.8.20;

import {Script, console} from "forge-std/Script.sol";

import {MockToken} from "../src/mocks/MockToken.sol";
import {Rollup} from "../src/Rollup.sol";
import {BridgeERC20} from "../src/BridgeERC20.sol";
import {Verifier} from "../src/Verifier.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

/// @title Deploy script

contract Deploy is Script {
    function run() external {
        //Load Variables from .env
        uint256 deployerPrivateKey = vm.envUint("L1_PRIVATE_KEY");
        address deployerAddress = vm.addr(deployerPrivateKey);
    
        
        vm.startBroadcast(deployerPrivateKey);

        // Deploy Mock Tokens
        MockToken token = new MockToken();
        console.log("TOKEN_ADDRESS=", address(token));
        address tokenAddress = address(token);

        // Deploy Helper Contracts
        Verifier verifier = new Verifier();
        console.log("Verifier deployed at:", address(verifier));

        // Pre-compute the Rollup Address
        uint256 deployerNonce = vm.getNonce(deployerAddress);
        address precomputedRollupAddress = computeCreateAddress(deployerAddress, deployerNonce + 1);
        
        console.log("Pre-computed Rollup Address:", precomputedRollupAddress);

        // Deploy Bridge
        BridgeERC20 bridge = new BridgeERC20(tokenAddress, precomputedRollupAddress);
        console.log("Bridge deployed at:", address(bridge));

        //Deploy Rollup
        bytes32 initialRoot = bytes32(0); 
        Rollup rollup = new Rollup(address(verifier), address(bridge), initialRoot);
        console.log("Rollup deployed at:", address(rollup));


        console.log("!!! DONT FORGET TO UPDATE THE .env FILE !!!");

        // Verify the Prediction
        require(address(rollup) == precomputedRollupAddress, "Address mismatch! Deployment failed.");

        vm.stopBroadcast();
    }
}
