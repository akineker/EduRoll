// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Script, console} from "forge-std/Script.sol";
import {BridgeERC20} from "../src/BridgeERC20.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

/// @title Deposit script

contract Deposit is Script {
    function run() external {
        //Load Variables from .env
        uint256 userPrivateKey = vm.envUint("L1_PRIVATE_KEY");
        address bridgeAddress = vm.envAddress("BRIDGE_ADDRESS");
        address tokenAddress = vm.envAddress("TOKEN_ADDRESS");

        uint256 amountToDeposit = 10 ether;

        // Start signing transactions as the bridge
        vm.startBroadcast(userPrivateKey);

        // Approve the Bridge
        // The Bridge needs permission to 'pull' tokens from wallet=
        IERC20(tokenAddress).approve(bridgeAddress, amountToDeposit);
        console.log("Approved Bridge to spend tokens");

        // Deposit
        BridgeERC20(bridgeAddress).deposit(amountToDeposit);
        console.log("Deposited", amountToDeposit, "tokens into Bridge");

        // Stop signing
        vm.stopBroadcast();
        
    }
}
