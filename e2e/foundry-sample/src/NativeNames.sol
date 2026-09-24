// SPDX-License-Identifier: MIT
pragma solidity ^0.8.34;

// Names that must not shadow imported native SDK types or annotations.
interface String {
    function text(string calldata input) external;
}

interface Data {
    function bytesValue(bytes calldata input) external;
}

interface Boolean {
    function flag(bool input) external;
}

interface List {
    function values(uint256[] calldata input) external;
}

interface BigInt {
    function amount(uint256 input) external;
}

interface Uint256 {
    function amount(uint256 input) external;
}

interface NativeNames {
    struct JvmField {
        uint256 amount;
    }

    function annotations(JvmField calldata input) external;
    function underscores(bool __, uint256 ___) external;
}
