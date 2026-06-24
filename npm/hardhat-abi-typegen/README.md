<p align="center">
  <img src="https://raw.githubusercontent.com/doublesharp/abi-typegen/main/docs/abi-typegen.png" alt="abi-typegen" width="360" />
</p>

<p align="center"><strong>Fast typed bindings from Solidity ABI artifacts.</strong></p>
<p align="center">Hardhat plugin &middot; auto-generate on compile &middot; ~30x faster than TypeChain</p>

# @0xdoublesharp/hardhat-abi-typegen

Hardhat plugin for [abi-typegen](https://github.com/doublesharp/abi-typegen). Generates typed bindings from Solidity ABI artifacts on every `hardhat compile`. Downloads the pre-built `abi-typegen` binary for your platform automatically.

## Install

```sh
npm install -D @0xdoublesharp/hardhat-abi-typegen
yarn add -D @0xdoublesharp/hardhat-abi-typegen
pnpm add -D @0xdoublesharp/hardhat-abi-typegen
```

## Setup

### Hardhat 3

Add the plugin to `plugins` in `hardhat.config.ts`:

```typescript
import { defineConfig } from "hardhat/config";
import abiTypegen from "@0xdoublesharp/hardhat-abi-typegen";

export default defineConfig({
  plugins: [abiTypegen],
  solidity: "0.8.34",
  typegen: {
    out: "src/generated",       // output directory (default: "src/generated")
    target: "viem",             // target name or comma-separated targets
    wrappers: true,             // emit typed wrappers (default: true)
    contracts: ["Token"],       // optional - limit to named contracts
    exclude: ["*Test", "*Mock"],// optional - exclude by glob pattern
  },
});
```

### Hardhat 2

Hardhat 2 projects can keep the side-effect import:

```typescript
import "@0xdoublesharp/hardhat-abi-typegen";

const config: HardhatUserConfig = {
  solidity: "0.8.34",
  typegen: {
    out: "src/generated",       // output directory (default: "src/generated")
    target: "viem",             // target name or comma-separated targets
    wrappers: true,             // emit typed wrappers (default: true)
    contracts: ["Token"],       // optional - limit to named contracts
    exclude: ["*Test", "*Mock"],// optional - exclude by glob pattern
  },
};

export default config;
```

The package uses conditional exports: ESM imports get the Hardhat 3 plugin object, and CommonJS/Hardhat 2 imports get the legacy task-hook plugin. The explicit fallback subpath `@0xdoublesharp/hardhat-abi-typegen/hardhat2` is also available for Hardhat 2 projects that want to pin the legacy adapter.

## Usage

Types are generated automatically after every compile:

```sh
npx hardhat compile
# → abi-typegen: generated 5 contract(s) → src/generated
```

Or generate directly:

```sh
npx abi-typegen generate --hardhat --target viem
```

## Configuration

| Option | Type | Default | Description |
|---|---|---|---|
| `out` | `string` | `"src/generated"` | Output directory for generated files |
| `target` | `string \| string[]` | `"viem"` | Target name, comma-separated targets, or array of targets (see below) |
| `wrappers` | `boolean` | `true` | Emit typed wrapper files when supported |
| `contracts` | `string[]` | `[]` | Limit to named contracts (empty = all) |
| `exclude` | `string[]` | `[]` | Exclude contracts matching glob patterns |

## Targets

| Target | Flag | Language | Ecosystem |
|---|---|---|---|
| viem | `viem` | TypeScript | [viem](https://viem.sh/) |
| zod | `zod` | TypeScript | [Zod](https://zod.dev/) 4 |
| wagmi | `wagmi` | TypeScript | [wagmi](https://wagmi.sh/) v2 |
| ethers v6 | `ethers` | TypeScript | [ethers](https://docs.ethers.org/v6/) v6 |
| ethers v5 | `ethers5` | TypeScript | ethers v5 |
| web3.js | `web3js` | TypeScript | [web3.js](https://docs.web3js.org/) v4 |
| Python | `python` | Python | [web3.py](https://web3py.readthedocs.io/) |
| Go | `go` | Go | [go-ethereum](https://geth.ethereum.org/) |
| Rust | `rust` | Rust | [alloy](https://alloy.rs/) |
| Swift | `swift` | Swift | [web3swift](https://github.com/web3swift-team/web3swift) |
| C# | `csharp` | C# | [Nethereum](https://nethereum.com/) |
| Kotlin | `kotlin` | Kotlin | [web3j](https://docs.web3j.io/) |
| Solidity | `solidity` | Solidity | External interfaces |
| YAML | `yaml` | YAML | Human-readable ABI descriptions |

Target aliases: `ethers6` → ethers, `web3` → web3js, `cs` → csharp, `kt` → kotlin, `sol` → solidity, `yml` → yaml

Multi-target examples — each target gets its own output subdirectory:

```typescript
target: "viem,python,rust"          // comma-separated string
target: ["viem", "python", "rust"]  // array of strings
```

## Notes

- When using `target: "zod"`, install the latest `zod` package in the consuming project
- Use comma-separated multi-target generation instead of the removed `all` and `all-ts` aliases

See [github.com/doublesharp/abi-typegen](https://github.com/doublesharp/abi-typegen) for full documentation.
