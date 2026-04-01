// SPDX-License-Identifier: MIT
pragma solidity ^0.8.34;

import {IEdgeCases} from "../src/generated-solidity/IEdgeCases.sol";
import {IExchange} from "../src/generated-solidity/IExchange.sol";
import {IRegistry} from "../src/generated-solidity/IRegistry.sol";
import {IToken} from "../src/generated-solidity/IToken.sol";
import {IVault} from "../src/generated-solidity/IVault.sol";

contract GeneratedInterfacesCheck {
    function tokenAllowance(
        IToken token,
        address owner,
        address spender
    ) external view returns (uint256) {
        return token.allowance(owner, spender);
    }

    function vaultShares(IVault vault, address user) external view returns (uint256) {
        IVault.Position memory position = vault.getPosition(user);
        return position.shares;
    }

    function selectors() external pure returns (bytes4, bytes4) {
        return (
            IToken.InsufficientBalance.selector,
            IVault.InsufficientShares.selector
        );
    }

    function touchOtherInterfaces(
        IExchange exchange,
        IRegistry registry,
        IEdgeCases edgeCases
    ) external pure returns (address, address, address) {
        return (address(exchange), address(registry), address(edgeCases));
    }
}