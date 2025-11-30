// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console2} from "forge-std/Test.sol";
import {BridgeERC20} from "../src/BridgeERC20.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

// Create a Mock Token
contract MockToken is ERC20 {
    constructor() ERC20("Mock Token", "MCK") {
        _mint(msg.sender, 1000000 * 10**18);
    }
}
contract BridgeTest is Test {
    BridgeERC20 public bridge;
    MockToken public token;

    // Define test actors
    address public user = address(1);
    address public rollup = address(2);

    // 'setUp' function runs before every single test
    function setUp() public {
        // Deploy the token
        token = new MockToken();

        // Deploy the Bridge
        bridge = new BridgeERC20(address(token), rollup);

        // Fund the user with 100 tokens
        token.transfer(user, 100 ether);

        // Act as the user to approve the bridge
        vm.startPrank(user);
        token.approve(address(bridge), type(uint256).max);
        vm.stopPrank();
    }

    // Test the Deposit Logic
    function test_Deposit() public {
        uint256 amount = 10 ether;

        // Check starting state
        assertEq(token.balanceOf(address(bridge)), 0);
        assertEq(token.balanceOf(user), 100 ether);

        vm.startPrank(user); // Become the user

        vm.expectEmit(true, false, false, true);
        emit BridgeERC20.DepositReceived(user, amount);

        bridge.deposit(amount);
        
        vm.stopPrank();

        // Check final state
        assertEq(token.balanceOf(address(bridge)), amount, "Bridge should hold the tokens");
        assertEq(token.balanceOf(user), 90 ether, "User balance should decrease");
    }

    // Test Failure Case
        function test_FailDepositZero() public {
        vm.startPrank(user);
        vm.expectRevert("Invalid amount");
        bridge.deposit(0);
        vm.stopPrank();
    }
}