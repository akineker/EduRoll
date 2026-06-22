/*  
    
*/

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {IBridge} from "./interfaces/IBridge.sol";

// @title BridgerERC20
contract BridgeERC20 is IBridge{
    using SafeERC20 for IERC20;

    address public immutable rollupContract;
    IERC20  public immutable token;

    // L1 deposit queue
    struct PendingDeposit { uint256 l2PubX; uint256 l2PubY; uint256 amount; }
    PendingDeposit[] public pendingDeposits;

    // @dev Restrict calls
    modifier onlyRollup {
        require(msg.sender == rollupContract, "Only rollup");
        _;
    }

    // @param _token Address of the ERC20 being bridged
    // @param _rollup Address of the authorised rollup contract
    constructor(address _token, address _rollup) {
        require(_token != address(0) && _rollup != address(0), "Zero address");
        token = IERC20(_token);
        rollupContract = _rollup;
    }

    // @param amount Amount of tokens to deposit.
    // @param l2PubX BabyJubJub public key X of the L2 recipient
    // @param l2PubY BabyJubJub public key Y of the L2 recipient
    function deposit(uint256 amount, uint256 l2PubX, uint256 l2PubY) external {
        require(amount > 0, "Invalid amount");
        token.safeTransferFrom(msg.sender, address(this), amount);
        pendingDeposits.push(PendingDeposit(l2PubX, l2PubY, amount));
        emit DepositQueued(pendingDeposits.length - 1, msg.sender, l2PubX, l2PubY, amount);
    }

    // @notice Total deposits ever queued
    function pendingDepositCount() external view returns (uint256) {
        return pendingDeposits.length;
    }

    // @param recipient Receiver address.
    // @param amount Amount to release.
    function releaseFunds(address recipient, uint256 amount) external onlyRollup override {
        require(amount > 0, "Invalid amount");
        token.safeTransfer(recipient, amount);
        emit WithdrawalReleased(recipient, amount);
    }

    // @notice Events emitted by the contract
    event DepositQueued(uint256 indexed index, address indexed from, uint256 l2PubX, uint256 l2PubY, uint256 amount);
    event WithdrawalReleased(address indexed recipient, uint256 amount);
}