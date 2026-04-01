// SPDX-License-Identifier: MIT
pragma solidity ^0.8.34;

/// @title Edge case contract for type generation validation
/// @dev Exercises: all integer widths, bytesN variants, nested arrays,
///      fixed arrays, anonymous events, fallback, payable constructor,
///      no-param functions, multi-output returns, and empty-input write functions.
contract EdgeCases {
    // ── Integer width boundaries ────────────────────────────────────────
    // uint8 (smallest) through uint256 (largest), including the 48-bit boundary
    // where viem switches from `number` to `bigint`.

    function smallInts() external pure returns (uint8 a, uint16 b, uint32 c, uint48 d) {
        return (1, 2, 3, 4);
    }

    function largeInts() external pure returns (uint64 a, uint128 b, uint256 c) {
        return (1, 2, 3);
    }

    function signedInts() external pure returns (int8 a, int48 b, int256 c) {
        return (-1, -2, -3);
    }

    // ── Bytes variants ─────────────────────────────────────────────────
    function fixedBytes() external pure returns (bytes1 a, bytes16 b, bytes32 c) {
        return (0x01, bytes16(0), bytes32(0));
    }

    function dynamicBytes() external pure returns (bytes memory) {
        return hex"deadbeef";
    }

    // ── Nested and multi-dimensional arrays ────────────────────────────
    function nestedArray() external pure returns (uint256[][] memory) {
        uint256[][] memory arr = new uint256[][](1);
        arr[0] = new uint256[](2);
        arr[0][0] = 1;
        arr[0][1] = 2;
        return arr;
    }

    function fixedArrayOfAddresses() external pure returns (address[3] memory) {
        return [address(1), address(2), address(3)];
    }

    // ── Multiple named return values ───────────────────────────────────
    /// @notice Returns multiple named values
    /// @return count The count
    /// @return total The total
    /// @return flag A flag
    function multiReturn() external pure returns (uint256 count, uint256 total, bool flag) {
        return (10, 100, true);
    }

    // ── Zero-param write function ──────────────────────────────────────
    function reset() external {
        // no-op for type testing
    }

    // ── Payable function ───────────────────────────────────────────────
    function fund() external payable {
        // accepts ETH
    }

    // ── Anonymous event ────────────────────────────────────────────────
    event DebugLog(string message) anonymous;

    // ── Regular events with mixed indexed ──────────────────────────────
    event ItemCreated(uint256 indexed id, address creator, string name);
    event Transfer(address indexed from, address indexed to, uint256 amount);

    // ── Custom errors ──────────────────────────────────────────────────
    error Overflow(uint256 max, uint256 actual);
    error Unauthorized();
    error InvalidInput(string reason, bytes data);

    // ── Fallback + receive ─────────────────────────────────────────────
    fallback() external payable {}
    receive() external payable {}

    // ── Struct with all types ──────────────────────────────────────────
    struct ComplexStruct {
        uint256 id;
        address owner;
        bytes32 hash;
        bool active;
        string name;
        uint64 timestamp;
    }

    function getComplex() external pure returns (ComplexStruct memory) {
        return ComplexStruct({
            id: 1,
            owner: address(0x1),
            hash: bytes32(0),
            active: true,
            name: "test",
            timestamp: 1234567890
        });
    }

    function processComplex(ComplexStruct calldata input) external pure returns (bytes32) {
        return keccak256(abi.encode(input.id, input.owner));
    }
}
