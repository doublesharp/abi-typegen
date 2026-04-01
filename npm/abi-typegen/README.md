# @0xdoublesharp/abi-typegen

Pre-built binary for [abi-typegen](https://github.com/doublesharp/abi-typegen). Generates typed bindings from Solidity ABI artifacts for 13 targets across 8 languages.

## Install

```sh
pnpm add -D @0xdoublesharp/abi-typegen
```

## Usage

```sh
npx abi-typegen generate --target viem
npx abi-typegen generate --target zod
npx abi-typegen generate --hardhat --target ethers
npx abi-typegen generate --target python,go,rust
```

When using the `zod` target, install the latest `zod` package in the consuming project.

For Hardhat auto-generation on compile, use [`@0xdoublesharp/hardhat-abi-typegen`](https://www.npmjs.com/package/@0xdoublesharp/hardhat-abi-typegen).

See [github.com/doublesharp/abi-typegen](https://github.com/doublesharp/abi-typegen) for full documentation.
