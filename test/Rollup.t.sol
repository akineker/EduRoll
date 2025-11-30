// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console} from "forge-std/Test.sol";
import {Rollup} from "../src/Rollup.sol";
import {Verifier} from "../src/Verifier.sol";
import {BridgeERC20} from "../src/BridgeERC20.sol";
import {IRollup} from "../src/interfaces/IRollup.sol";
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

// Simple Mock Token for the Bridge
contract MockToken is ERC20 {
    constructor() ERC20("Mock", "MCK") {
        _mint(msg.sender, 1_000_000 ether);
    }
}

contract RollupTest is Test {
    Rollup public rollup;
    Verifier public verifier;
    BridgeERC20 public bridge;
    MockToken public token;

    function setUp() public {
        //Deploy helpers
        verifier = new Verifier();
        token = new MockToken();

        // Handle Circular Dependency (Bridge <-> Rollup)
        uint256 deployerNonce = vm.getNonce(address(this));
        address precomputedRollup = computeCreateAddress(address(this), deployerNonce + 1);

        // Deploy Bridge
        bridge = new BridgeERC20(address(token), precomputedRollup);

        // Deploy Rollup
        rollup = new Rollup(address(verifier), address(bridge), bytes32(0));

        // Sanity Check
        assertEq(address(rollup), precomputedRollup, "Deployment address mismatch");
    }

    function test_SubmitBatchPath() public {
        // Prepare Dummy Proof
        uint256[2] memory a = [uint256(1), uint256(2)];
        uint256[2][2] memory b = [[uint256(3), uint256(4)], [uint256(5), uint256(6)]];
        uint256[2] memory c = [uint256(7), uint256(8)];

        //Prepare Public Inputs
        bytes32 newRoot = bytes32(uint256(0x123456789)); 
        
        IRollup.PublicInputs memory inputs = IRollup.PublicInputs({
            oldRoot: bytes32(0), 
            newRoot: newRoot,
            withdrawalsRoot: bytes32(0),
            depositsRoot: bytes32(0),
            batchDataHash: bytes32(0),
            batchNumber: 1, // Must be current + 1
            l1BlockNumber: uint64(block.number),
            circuitVersion: 1
        });

        // Expect the Event
        vm.expectEmit(true, false, false, true);
        emit IRollup.BatchSubmitted(1, newRoot);

        // 4. Call submitBatch
        rollup.submitBatch(a, b, c, inputs);

        // 5. Assertions: Verify State Updates
        assertEq(rollup.stateRoot(), newRoot, "State root should update to newRoot");
        assertEq(rollup.batchNumber(), 1, "Batch number should increment");
    }

    function test_RevertIf_WrongOldRoot() public {
        // This test ensures no one can force an invalid state transition
        uint256[2] memory a = [uint256(1), uint256(2)];
        uint256[2][2] memory b = [[uint256(3), uint256(4)], [uint256(5), uint256(6)]];
        uint256[2] memory c = [uint256(7), uint256(8)];

        IRollup.PublicInputs memory inputs = IRollup.PublicInputs({
            oldRoot: bytes32(uint256(0xDEADBEEF)), // Intentional wrong oldRoot to test
            newRoot: bytes32(uint256(0x123456)),
            withdrawalsRoot: bytes32(0),
            depositsRoot: bytes32(0),
            batchDataHash: bytes32(0),
            batchNumber: 1,
            l1BlockNumber: uint64(block.number),
            circuitVersion: 1
        });

        vm.expectRevert("Invalid old root");
        
        rollup.submitBatch(a, b, c, inputs);
    }
}