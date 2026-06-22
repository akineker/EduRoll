// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IVerifier} from "./interfaces/IVerifier.sol";

/// @title Mockup Verifier — always returns true, used for local testing only
contract VerifierMock is IVerifier{
    function verifyProof(
        uint256[2] calldata _pA,
        uint256[2][2] calldata _pB,
        uint256[2] calldata _pC,
        uint256[3] calldata _pubSignals
    ) external pure returns (bool){
        return true;
    }
}