// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Script, console} from "forge-std/Script.sol";
import {BridgeERC20} from "../src/BridgeERC20.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

// @title Deposit script

contract Deposit is Script {
    function run() external {
        //Load Variables from .env
        uint256 userPrivateKey = vm.envUint("L1_PRIVATE_KEY");
        address bridgeAddress = vm.envAddress("BRIDGE_ADDRESS");
        address tokenAddress = vm.envAddress("TOKEN_ADDRESS");

        uint256 amountToDeposit = 10 ether; // Example amount (10 tokens)
        // L2 recipient BabyJubJub pubkey
        uint256 l2PubX = vm.envOr("L2_PUBKEY_X", uint256(1));
        uint256 l2PubY = vm.envOr("L2_PUBKEY_Y", uint256(2));

        // 2. Start signing transactions as the user
        vm.startBroadcast(userPrivateKey);

        // 3. Approve the Bridge
        IERC20(tokenAddress).approve(bridgeAddress, amountToDeposit);
        console.log("Approved Bridge to spend tokens");

        // 4. Deposit
        BridgeERC20(bridgeAddress).deposit(amountToDeposit, l2PubX, l2PubY);
        console.log("Deposited", amountToDeposit, "tokens into Bridge");

        // 5. Stop signing
        vm.stopBroadcast();
        
    }
}
