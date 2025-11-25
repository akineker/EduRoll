/* 

*/

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title IBridge
interface IBridge {
  function releaseFunds(
    address recipient,
    uint256 amount
  ) external;
}