// SPDX-License-Identifier: MIT
pragma solidity ^0.8.34;

/// @notice Shapes the native-language targets must handle: arrays and bytes
/// inside structs, nested tuples, inherited-getter field names, acronyms,
/// unnamed parameters, and names shared by a function and an event.
interface NativeCases {
    struct Call3Value {
        address target;
        bool allowFailure;
        uint256 value;
        bytes callData;
    }

    struct Grid {
        uint256[][] rows;
        bool[2] flags;
        bytes32[] tags;
    }

    struct Batch {
        Call3Value[] calls;
        Grid grid;
        uint24 fee;
        int128 delta;
    }

    event Transfer(address indexed from, address indexed to, uint256 value);
    event Logged(bytes data) anonymous;

    error Failed(uint256 index, bytes reason);

    function transfer(address to, uint256 value) external returns (bool);
    function tokenURI(uint256 id) external view returns (string memory);
    function aggregate(Batch calldata batch) external payable returns (bytes[] memory);
    function record(address, address, uint256) external;
    function PREMIUM_PERIOD() external view returns (uint256);
    function premiumPeriod() external view returns (uint256);
}
