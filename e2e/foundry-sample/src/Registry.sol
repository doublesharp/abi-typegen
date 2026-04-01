// SPDX-License-Identifier: MIT
pragma solidity ^0.8.34;

/// @title Registry with nested structs, bytes types, and pure functions
/// @notice Exercises bytesN, nested tuples, pure/view, and multiple return values
contract Registry {
    struct Entry {
        bytes32 id;
        string label;
        Metadata meta;
    }

    struct Metadata {
        uint64 createdAt;
        address owner;
        bool active;
    }

    mapping(bytes32 => Entry) internal entries;

    event Registered(bytes32 indexed id, address indexed owner);

    error AlreadyRegistered(bytes32 id);
    error NotFound(bytes32 id);

    /// @notice Register a new entry
    /// @param id Unique identifier
    /// @param label Human-readable label
    function register(bytes32 id, string calldata label) external {
        if (entries[id].meta.owner != address(0)) revert AlreadyRegistered(id);
        entries[id] = Entry({
            id: id,
            label: label,
            meta: Metadata({
                createdAt: uint64(block.timestamp),
                owner: msg.sender,
                active: true
            })
        });
        emit Registered(id, msg.sender);
    }

    /// @notice Look up an entry by ID
    /// @param id Entry identifier
    /// @return entry The full entry with nested metadata
    function getEntry(bytes32 id) external view returns (Entry memory entry) {
        if (entries[id].meta.owner == address(0)) revert NotFound(id);
        return entries[id];
    }

    /// @notice Check multiple IDs at once
    /// @return ids The IDs queried
    /// @return owners The owners of each entry
    function batchLookup(bytes32[] calldata ids)
        external
        view
        returns (bytes32[] memory, address[] memory)
    {
        bytes32[] memory outIds = new bytes32[](ids.length);
        address[] memory owners = new address[](ids.length);
        for (uint256 i = 0; i < ids.length; i++) {
            outIds[i] = ids[i];
            owners[i] = entries[ids[i]].meta.owner;
        }
        return (outIds, owners);
    }

    /// @notice Pure utility — hash a label
    function hashLabel(string calldata label) external pure returns (bytes32) {
        return keccak256(abi.encodePacked(label));
    }

    /// @notice Pure utility — encode a uint and address
    function encode(uint256 value, address addr) external pure returns (bytes memory) {
        return abi.encode(value, addr);
    }
}
