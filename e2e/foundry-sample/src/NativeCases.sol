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

    struct PackedMarket {
        bytes32 first;
        bytes32 second;
        uint256 price;
    }

    function getMarket() external view returns (PackedMarket memory);

    struct Amounts {
        uint256[] amounts;
    }

    function getAmounts() external view returns (Amounts memory);

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

    /// Named like SDK types (`Data` in Swift and Foundation).
    struct Data {
        bytes payload;
    }

    event IndexedReferences(string indexed label, bytes indexed payload, uint256[] indexed values);
    event IndexedFixed(uint256[2] indexed pair);
    event Changed(uint256 indexed self, bool self_);
    event Positional(uint256 indexed, bool _0);
    function positional(uint256, bool _0) external;

    event Transfer(address indexed from, address indexed to, uint256 value);
    event Logged(bytes data) anonymous;

    error Failed(uint256 index, bytes reason);

    /// @notice Returns   ``owner``'s balance across
    ///         multiple    aligned words and `inline_code`.
    function balanceOf(address owner) external view returns (uint256);
    function manyArguments(uint256 a, uint256 b, uint256 c, uint256 d, uint256 e, uint256 f, uint256 g, uint256 h, uint256 i) external;

    function transfer(address to, uint256 value) external returns (bool);
    function tokenURI(uint256 id) external view returns (string memory);
    function aggregate(Batch calldata batch) external payable returns (bytes[] memory);
    function record(address, address, uint256) external;
    function store(Data calldata data, uint256[40] calldata window) external;
    function PREMIUM_PERIOD() external view returns (uint256);
    function premiumPeriod() external view returns (uint256);
}

/// A contract whose snake_case module name is a Rust keyword.
interface Override {
    function ping() external;
}

/// A contract whose module filename would otherwise replace the Rust index.
interface Mod {
    function ping() external;
}
