// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

contract NamingCases {
    struct Item {
        uint256 class;
        bool _class;
    }

    function run(uint256 class, bool _class) external returns (uint256, bool) {
        return (class, _class);
    }

    function foo_bar(uint256 amount) external returns (uint256) {
        return amount;
    }

    function fooBar(bool enabled) external returns (bool) {
        return enabled;
    }

    function read() external pure returns (Item memory item) {
        return Item({class: 7, _class: true});
    }

    function store(Item calldata item) external returns (uint256, bool) {
        return (item.class, item._class);
    }
}
