// SPDX-License-Identifier: MIT
pragma solidity ^0.8.34;

/// @title Multi-asset vault with structs, overloads, and arrays
/// @notice Exercises tuple outputs, fixed arrays, overloaded functions, and receive
contract Vault {
    struct Position {
        uint256 shares;
        uint64 depositedAt;
        address token;
    }

    mapping(address => Position) public positions;

    event Deposited(address indexed user, uint256 amount);
    event BatchProcessed(uint256 count);

    error InsufficientShares(uint256 available, uint256 requested);
    error ZeroAmount();

    receive() external payable {
        emit Deposited(msg.sender, msg.value);
    }

    /// @notice Deposit ETH
    /// @param amount Amount to deposit
    function deposit(uint256 amount) external payable {
        if (amount == 0) revert ZeroAmount();
        positions[msg.sender].shares += amount;
        positions[msg.sender].depositedAt = uint64(block.timestamp);
        emit Deposited(msg.sender, amount);
    }

    /// @notice Deposit on behalf of another user
    /// @param amount Amount to deposit
    /// @param recipient Recipient of the deposit
    function deposit(uint256 amount, address recipient) external payable {
        if (amount == 0) revert ZeroAmount();
        positions[recipient].shares += amount;
        positions[recipient].depositedAt = uint64(block.timestamp);
        emit Deposited(recipient, amount);
    }

    /// @notice Get a user's position
    /// @param user User address
    /// @return position The user's full position struct
    function getPosition(address user) external view returns (Position memory position) {
        return positions[user];
    }

    /// @notice Get balances for multiple users
    function getBalances(address[] calldata users) external view returns (uint256[] memory) {
        uint256[] memory result = new uint256[](users.length);
        for (uint256 i = 0; i < users.length; i++) {
            result[i] = positions[users[i]].shares;
        }
        return result;
    }

    /// @notice Returns a fixed-size array
    function getMatrix() external pure returns (uint256[3] memory) {
        return [uint256(1), 2, 3];
    }
}
