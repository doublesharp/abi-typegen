// SPDX-License-Identifier: MIT
pragma solidity ^0.8.34;

/// @title Simple ERC20-like token
/// @notice A minimal token for testing abi-typegen output
contract Token {
    string public name;
    string public symbol;
    uint8 public decimals;
    uint256 public totalSupply;

    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;

    event Transfer(address indexed from, address indexed to, uint256 amount);
    event Approval(address indexed owner, address indexed spender, uint256 amount);

    error InsufficientBalance(address account, uint256 available, uint256 required);
    error InvalidRecipient();

    constructor(string memory _name, string memory _symbol, uint8 _decimals) {
        name = _name;
        symbol = _symbol;
        decimals = _decimals;
    }

    /// @notice Transfer tokens to a recipient
    /// @param to Recipient address
    /// @param amount Amount to transfer
    /// @return success Whether the transfer succeeded
    function transfer(address to, uint256 amount) external returns (bool success) {
        if (balanceOf[msg.sender] < amount) {
            revert InsufficientBalance(msg.sender, balanceOf[msg.sender], amount);
        }
        if (to == address(0)) revert InvalidRecipient();
        balanceOf[msg.sender] -= amount;
        balanceOf[to] += amount;
        emit Transfer(msg.sender, to, amount);
        return true;
    }

    /// @notice Approve spender to transfer tokens
    /// @param spender Spender address
    /// @param amount Allowance amount
    function approve(address spender, uint256 amount) external returns (bool) {
        allowance[msg.sender][spender] = amount;
        emit Approval(msg.sender, spender, amount);
        return true;
    }

    function mint(address to, uint256 amount) external {
        totalSupply += amount;
        balanceOf[to] += amount;
        emit Transfer(address(0), to, amount);
    }
}
