# @0xdoublesharp/hardhat-abi-typegen

Hardhat plugin for [abi-typegen](https://github.com/doublesharp/abi-typegen). Generates typed bindings from Solidity ABI artifacts on every `hardhat compile`. ~30x faster than TypeChain.

Downloads the pre-built `abi-typegen` binary for your platform automatically.

## Install

```sh
pnpm add -D @0xdoublesharp/hardhat-abi-typegen
```

## Setup

Add to `hardhat.config.ts`:

```typescript
import "@0xdoublesharp/hardhat-abi-typegen";

const config: HardhatUserConfig = {
  solidity: "0.8.34",
  typegen: {
    out: "src/generated",   // output directory (default: "src/generated")
    target: "viem",         // single target or comma-separated targets
    wrappers: true,          // emit typed wrappers (default: true)
    contracts: ["Token"],   // optional contract allowlist
    exclude: ["*Test"],     // optional contract denylist globs
  },
};
```

## Usage

Types are generated automatically after every compile:

```sh
npx hardhat compile
# → abi-typegen: generated 5 contract(s) → src/generated
```

Or run directly:

```sh
npx abi-typegen generate --hardhat
```

## Configuration

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `out` | `string` | `"src/generated"` | Output directory |
| `target` | `string` | `"viem"` | Single target or comma-separated targets |
| `wrappers` | `boolean` | `true` | Emit typed wrapper files when supported |
| `contracts` | `string[]` | `[]` | Limit to named contracts (empty = all) |
| `exclude` | `string[]` | `[]` | Exclude contracts matching glob patterns |

Use comma-separated multi-target generation instead of the removed `all` and `all-ts` aliases.

When using `target: "zod"`, install the latest `zod` package in the consuming project.

See [github.com/doublesharp/abi-typegen](https://github.com/doublesharp/abi-typegen) for full documentation.
