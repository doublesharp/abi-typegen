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
| Named multi-returns | yes | no |
| Signature-based overloads | yes | no |
| `--check` for CI | yes | no |
| `--exclude` patterns | yes | no |
| Watch mode | yes | no |

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
