// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console} from "forge-std/Test.sol";
import {RollupSP1, PublicValues} from "../src/RollupSP1.sol";
import {ISP1Verifier} from "@sp1-contracts/ISP1Verifier.sol";

/// Accepts every proof — stand-in for the SP1 verifier gateway so the state
/// machine can be tested without a real proof. Real-proof fixtures come from
/// `cargo run --release --bin evm` (see circuits/sp1/TODO.md).
contract AcceptingVerifier is ISP1Verifier {
    function verifyProof(bytes32, bytes calldata, bytes calldata) external pure {}
}

/// Rejects every proof.
contract RejectingVerifier is ISP1Verifier {
    function verifyProof(bytes32, bytes calldata, bytes calldata) external pure {
        revert("invalid proof");
    }
}

/// Tests RollupSP1's state machine with the verifier mocked (no real proof
/// needed). Mirrors the intent of the repo-root Rollup.t.sol.
contract RollupSP1Test is Test {
    address verifier;
    RollupSP1 rollup;

    bytes32 constant VKEY = bytes32(uint256(0x1234));
    bytes32 constant GENESIS_ROOT = bytes32(uint256(0xabc123));
    address constant SUBMITTER = address(0xBEEF);

    function setUp() public {
        verifier = address(new AcceptingVerifier());
        rollup = new RollupSP1(verifier, VKEY, GENESIS_ROOT, SUBMITTER);
    }

    // Build a valid publicValues blob for batch #1 on top of the genesis root.
    function _publicValues() internal pure returns (PublicValues memory pv, bytes memory batchData) {
        batchData = hex"00000001_deadbeef"; // arbitrary DA bytes
        pv = PublicValues({
            oldRoot: GENESIS_ROOT,
            newRoot: bytes32(uint256(0xdef456)),
            depositsRoot: bytes32(0),
            withdrawalsRoot: bytes32(uint256(0x77)),
            batchDataHash: keccak256(batchData),
            batchNumber: 1
        });
    }

    function test_ValidBatchUpdatesState() public {
        (PublicValues memory pv, bytes memory batchData) = _publicValues();

        vm.prank(SUBMITTER);
        rollup.submitBatch(abi.encode(pv), hex"aa", batchData);

        assertEq(rollup.currentStateRoot(), pv.newRoot);
        assertEq(rollup.currentWithdrawalsRoot(), pv.withdrawalsRoot);
        assertEq(rollup.currentDepositsRoot(), pv.depositsRoot);
        assertEq(rollup.currentBatchNumber(), 1);
    }

    function testRevert_NonSubmitter() public {
        (PublicValues memory pv, bytes memory batchData) = _publicValues();

        vm.expectRevert(bytes("Only submitter"));
        rollup.submitBatch(abi.encode(pv), hex"aa", batchData);
    }

    function testRevert_WrongOldRoot() public {
        (PublicValues memory pv, bytes memory batchData) = _publicValues();
        pv.oldRoot = bytes32(uint256(0xbad));

        vm.prank(SUBMITTER);
        vm.expectRevert(bytes("Invalid old root"));
        rollup.submitBatch(abi.encode(pv), hex"aa", batchData);
    }

    function testRevert_WrongBatchNumber() public {
        (PublicValues memory pv, bytes memory batchData) = _publicValues();
        pv.batchNumber = 2; // must be currentBatchNumber + 1 == 1

        vm.prank(SUBMITTER);
        vm.expectRevert(bytes("Batch number is invalid."));
        rollup.submitBatch(abi.encode(pv), hex"aa", batchData);
    }

    function testRevert_BatchDataMismatch() public {
        (PublicValues memory pv,) = _publicValues();

        vm.prank(SUBMITTER);
        vm.expectRevert(bytes("batchData hash mismatch"));
        // Post different DA bytes than the proof committed to.
        rollup.submitBatch(abi.encode(pv), hex"aa", hex"1337");
    }

    function testRevert_InvalidProofPropagates() public {
        (PublicValues memory pv, bytes memory batchData) = _publicValues();
        RollupSP1 strictRollup =
            new RollupSP1(address(new RejectingVerifier()), VKEY, GENESIS_ROOT, SUBMITTER);

        vm.prank(SUBMITTER);
        vm.expectRevert(bytes("invalid proof"));
        strictRollup.submitBatch(abi.encode(pv), hex"aa", batchData);
    }

    function test_BatchNumberChains() public {
        (PublicValues memory pv, bytes memory batchData) = _publicValues();

        vm.prank(SUBMITTER);
        rollup.submitBatch(abi.encode(pv), hex"aa", batchData);

        // Batch #2 must build on batch #1's newRoot.
        PublicValues memory pv2 = PublicValues({
            oldRoot: pv.newRoot,
            newRoot: bytes32(uint256(0xfeed)),
            depositsRoot: bytes32(0),
            withdrawalsRoot: bytes32(0),
            batchDataHash: keccak256(batchData),
            batchNumber: 2
        });
        vm.prank(SUBMITTER);
        rollup.submitBatch(abi.encode(pv2), hex"aa", batchData);
        assertEq(rollup.currentBatchNumber(), 2);
        assertEq(rollup.currentStateRoot(), pv2.newRoot);

        // Replaying batch #2 fails both root and number checks.
        vm.prank(SUBMITTER);
        vm.expectRevert(bytes("Invalid old root"));
        rollup.submitBatch(abi.encode(pv2), hex"aa", batchData);
    }

    function test_SetSubmitter() public {
        rollup.setSubmitter(address(0xCAFE));
        assertEq(rollup.submitter(), address(0xCAFE));

        vm.prank(address(0xCAFE));
        vm.expectRevert(bytes("Only owner"));
        rollup.setSubmitter(address(0xDEAD));
    }

    // ── Realistic gas measurement ───────────────────────────────────────
    //
    // The tests above use an 8-byte placeholder `batchData`, which
    // understates calldata cost. A real batch posts N_TXS=20 transfers at
    // 160 bytes each (see stf-core::encode_batch_data and
    // offchain/crates/submitter/src/main.rs) — 3,204 bytes total. This test
    // reproduces that exact byte layout and length so the measured gas is
    // comparable to what the real submitter actually pays for DA + state
    // update. Only the SP1 verifier itself is mocked (O(1) gas here); the
    // real SP1 Groth16-wrapped on-chain verify is a separate, DOCUMENTED
    // (not measured) ~270-300k gas — see circuits/sp1/README.md and the
    // root README's proving-backend comparison table.

    /// The submitter's canonical DA encoding: 4-byte BE tx count, then per
    /// tx (160 bytes): sender(20) | recipient(20) | amount(16) | nonce(8) |
    /// sig_R8x(32) | sig_R8y(32) | sig_S(32). Bytes are synthetic (this test
    /// only needs the right LENGTH and shape for a realistic gas number —
    /// validity is irrelevant since the verifier is mocked).
    function _realisticBatchData() internal pure returns (bytes memory da) {
        da = abi.encodePacked(uint32(20));
        for (uint256 i = 0; i < 20; i++) {
            da = bytes.concat(
                da,
                bytes20(keccak256(abi.encode("sender", i))),
                bytes20(keccak256(abi.encode("recipient", i))),
                abi.encodePacked(uint128(100), uint64(i)), // amount, nonce
                keccak256(abi.encode("r8x", i)),
                keccak256(abi.encode("r8y", i)),
                keccak256(abi.encode("s", i))
            );
        }
    }

    /// A Groth16-wrapped SP1 proof is documented at ~260 bytes on-chain
    /// (4-byte selector + 8 field elements) — this placeholder matches that
    /// LENGTH for a realistic calldata-gas measurement. Its bytes are not a
    /// real proof; the verifier is mocked, so validity doesn't matter here.
    function _placeholderProof() internal pure returns (bytes memory p) {
        p = bytes.concat(bytes4(0x11b6a09d));
        for (uint256 i = 0; i < 8; i++) p = bytes.concat(p, keccak256(abi.encode("proof", i)));
    }

    function test_GasWithRealisticDataAvailability() public {
        bytes memory da = _realisticBatchData();
        assertEq(da.length, 3204, "DA payload should match the submitter's real 20-tx encoding");

        PublicValues memory pv = PublicValues({
            oldRoot: GENESIS_ROOT,
            newRoot: bytes32(uint256(0xdef456)),
            depositsRoot: bytes32(0),
            withdrawalsRoot: bytes32(0),
            batchDataHash: keccak256(da),
            batchNumber: 1
        });
        bytes memory proof = _placeholderProof();
        bytes memory pvb = abi.encode(pv);

        // Intrinsic tx cost: 21,000 base + 4 gas/zero-byte + 16 gas/non-zero
        // byte of calldata (EIP-2028) — this is what a REAL top-level
        // transaction pays that an internal Solidity call (below) does not.
        bytes memory callCalldata = abi.encodeCall(rollup.submitBatch, (pvb, proof, da));
        uint256 zeroBytes;
        for (uint256 i = 0; i < callCalldata.length; i++) {
            if (callCalldata[i] == 0) zeroBytes++;
        }
        uint256 intrinsic = 21_000 + 4 * zeroBytes + 16 * (callCalldata.length - zeroBytes);

        vm.prank(SUBMITTER);
        uint256 gasBefore = gasleft();
        rollup.submitBatch(pvb, proof, da);
        uint256 executionGas = gasBefore - gasleft();

        console.log(string.concat("calldata bytes: ", vm.toString(callCalldata.length)));
        console.log(string.concat("intrinsic (21k + calldata): ", vm.toString(intrinsic)));
        console.log(string.concat("execution gas (verifier mocked): ", vm.toString(executionGas)));
        console.log(string.concat("total EXCLUDING the real SP1 verifier: ", vm.toString(intrinsic + executionGas)));

        // Regression guard, not a precision claim — see the console output
        // above (and the number quoted in the root README) for the actual
        // measured figure at commit time.
        assertLt(intrinsic + executionGas, 300_000, "unexpectedly high for a mocked verifier");
    }
}
