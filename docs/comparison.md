# Comparison

## vs TypeChain

[TypeChain](https://github.com/dethcrypto/TypeChain) is the most popular ABI codegen tool. Key differences:

| | abi-typegen | TypeChain |
|---|---|---|
| Runtime | Native Rust binary | Node.js |
| Speed | ~25ms | ~763ms |
| Targets | 11 (7 languages) | 4 (TypeScript only) |
| ethers v6 | yes | yes |
| ethers v5 | yes | yes |
| viem | yes | no |
| wagmi hooks | yes | no |
| web3.js | yes | yes |
| Python | yes | no |
| Go | yes | no |
| Rust | yes | no |
| Swift | yes | no |
| C# | yes | no |
| Kotlin | yes | no |
| Requires Node.js | no | yes |
| Foundry support | yes | via plugin |
| Hardhat support | yes | yes |
| Named multi-returns | yes | yes |
| Signature-based overloads | yes | no |
| `--check` for CI | yes | no |
| `--exclude` patterns | yes | no |
| Watch mode | yes | no |

### Ethers Type Surface

abi-typegen's `ethers` target is checked against TypeChain's `ethers-v6` output on the e2e contracts. The `ethers5` target is validated against ethers v5 ABI decoding behavior. It intentionally matches SDK runtime behavior where that matters:

- Function inputs use ethers-compatible input aliases: `BigNumberish` for integers and `BytesLike` for bytes. Ethers v6 address inputs use `AddressLike`.
- Ethers v6 integer outputs are `bigint`; ethers v5 integer outputs are `number` for <= 48-bit values and `BigNumber` for wider values.
- Fixed-size arrays are emitted as tuple types, e.g. `uint256[3]` becomes `[bigint, bigint, bigint]`.
- Named tuple and multi-return outputs expose tuple positions plus named object fields.

abi-typegen keeps a smaller wrapper surface than TypeChain: it does not generate factories, `BaseContract` subclasses, or encode/decode helper overloads. It improves the callable method surface with signature-based overload names such as `depositUint256Address`, avoiding quoted full-signature property names for overloaded functions.

## vs abigen (Go)

[abigen](https://geth.ethereum.org/docs/tools/abigen) is geth's built-in Go binding generator. It generates Go only. abi-typegen generates Go bindings plus 10 other targets from the same artifacts.

## vs wagmi CLI

[wagmi CLI](https://wagmi.sh/cli) generates TypeScript types and React hooks from contract configs. It fetches ABIs from Etherscan or reads Foundry/Hardhat artifacts. abi-typegen is faster (native binary vs Node.js) and supports more targets, but wagmi CLI has deeper wagmi/viem integration.

## vs ABIType

[ABIType](https://abitype.dev/) is a TypeScript type-level library — no code generation. It infers types from `as const` ABI objects at compile time. abi-typegen generates those `as const` ABI objects. The two are complementary: abi-typegen outputs the ABI file, ABIType/viem infers types from it.

## vs forge bind

[forge bind](https://book.getfoundry.sh/reference/forge/forge-bind) generates Rust/Alloy bindings from Foundry artifacts. It's built into Foundry and produces more complete Rust bindings (with contract call methods). abi-typegen generates struct types for Rust plus 10 other language targets.

## vs web3j / Nethereum

[web3j](https://docs.web3j.io/) (Java/Kotlin) and [Nethereum](https://nethereum.com/) (C#) are full SDKs with built-in codegen. They generate complete contract wrappers with RPC methods. abi-typegen generates typed structs and interfaces — lighter output, but covers all languages from one tool.
