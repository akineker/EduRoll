/*  
    
*/

// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {IBridge} from "./interfaces/IBridge.sol";

/// @title BridgerERC20
contract BridgeERC20 is IBridge{
    using SafeERC20 for IERC20;

    address public immutable rollupContract;
    IERC20  public immutable token;

    /// @dev Restrict calls to the authorised rollup contract.
    modifier onlyRollup {
        require(msg.sender == rollupContract, "Only rollup");
        _;
    }

    /// @param _token Address of the ERC20 being bridged.
    /// @param _rollup Address of the authorised rollup contract.
    constructor(address _token, address _rollup) {
        require(_token != address(0) && _rollup != address(0), "Zero address");
        token = IERC20(_token);
        rollupContract = _rollup;
    }

    /// @notice Deposit tokens into the bridge.
    /// @param amount Amount of tokens to deposit.
    function deposit(uint256 amount) external{
        require(amount > 0, "Invalid amount");
        token.safeTransferFrom(msg.sender, address(this), amount);
        emit DepositReceived(msg.sender, amount);
    }

    /// @notice Release bridged tokens to a recipient. Callable only by rollup.
    /// @param recipient Receiver address.
    /// @param amount Amount to release.

    function releaseFunds(address recipient, uint256 amount) external onlyRollup override {
        require(amount > 0, "Invalid amount");
        token.safeTransfer(recipient, amount);
        emit WithdrawalReleased(recipient, amount);
        // TODO: WITHDRAWAL STRUCT defined in IBRIGE use it here
    }

    /// @notice Events happen in the contract
    event DepositReceived(address indexed user, uint256 amount);
    event WithdrawalReleased(address indexed recipient, uint256 amount);
}