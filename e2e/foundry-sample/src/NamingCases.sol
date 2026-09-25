// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

contract NamingCases {
    // These functions intentionally remain nonpayable writes for binding tests.
    uint256 private invocations;

    struct Item {
        uint256 class;
        bool _class;
    }

    function run(uint256 class, bool _class) external returns (uint256, bool) {
        invocations++;
        return (class, _class);
    }

    function foo_bar(uint256 amount) external returns (uint256) {
        invocations++;
        return amount;
    }

    function fooBar(bool enabled) external returns (bool) {
        invocations++;
        return enabled;
    }

    function read() external pure returns (Item memory item) {
        return Item({class: 7, _class: true});
    }

    function store(Item calldata item) external returns (uint256, bool) {
        invocations++;
        return (item.class, item._class);
    }
}
