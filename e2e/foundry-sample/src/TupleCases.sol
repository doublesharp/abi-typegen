// SPDX-License-Identifier: MIT
pragma solidity ^0.8.34;

library TupleAmount {
    struct Position { uint256 amount; }
}

library TupleAccount {
    struct Position { address account; }
}

interface TupleCases {
    event Moved(uint256 amount, address indexed from, bytes32 memo, address indexed to);

    function setMatrix(uint256[][] calldata matrix) external;
    function setRows(bool[2][] calldata rows) external;
    function deposit(TupleAmount.Position calldata position) external;
    function deposit(TupleAccount.Position calldata position) external;
}
