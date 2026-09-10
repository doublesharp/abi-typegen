// SPDX-License-Identifier: MIT
pragma solidity ^0.8.34;

contract Token {
    event Transfer(address indexed from, address indexed to, uint256 value);

    mapping(address => uint256) public balanceOf;

    error InsufficientBalance(address owner, uint256 available, uint256 requested);

    constructor(uint256 initialSupply) {
        balanceOf[msg.sender] = initialSupply;
    }

    function transfer(address to, uint256 value) external returns (bool) {
        uint256 senderBalance = balanceOf[msg.sender];
        if (senderBalance < value) {
            revert InsufficientBalance(msg.sender, senderBalance, value);
        }

        unchecked {
            balanceOf[msg.sender] = senderBalance - value;
            balanceOf[to] += value;
        }

        emit Transfer(msg.sender, to, value);
        return true;
    }
}
