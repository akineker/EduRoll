/* 

*/

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IVerifier} from "./interfaces/IVerifier.sol";

/// @title Mockup Verifier
contract Verifier is IVerifier{

    // Mock Verifier
    // TODO: Generate Verifier.sol using snarkjs and  /circuits/build/transfer.r1cs 
    function verifyProof(
        uint256[2] calldata a,
        uint256[2][2] calldata b,
        uint256[2] calldata c,
        uint256[] calldata publicInputs
    ) external view returns (bool){
        return true;
    }
}