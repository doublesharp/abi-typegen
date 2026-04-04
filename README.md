# abi-typegen

<p align="center">
  <img src="docs/abi-typegen.png" alt="abi-typegen" width="360" />
</p>

<p align="center"><strong>Fast typed bindings from Solidity ABI artifacts.</strong></p>
<p align="center">Foundry or Hardhat in, production-ready client bindings out.</p>

`abi-typegen` is a native Rust CLI that reads compiled Solidity artifacts and generates typed bindings for 14 targets across 8 languages. It works with Foundry and Hardhat, supports single-target and multi-target workflows, and includes the command surface you need for local iteration and CI: `generate`, `watch`, `diff`, `json`, `fetch`, `--check`, and `--clean`.

It is designed to be easy to drop into an existing project: point it at your artifacts, pick a target, and generate code that matches the ecosystem you actually use.

## Why abi-typegen

- Native Rust CLI with very low overhead
- Works with Foundry `out/` and Hardhat `artifacts/contracts/`
- Generates bindings for TypeScript, Python, Go, Rust, Swift, C#, Kotlin, and Solidity
- Supports comma-separated multi-target generation into isolated output directories
- CI-friendly stale-output detection with `generate --check`
- Dry-run inspection with `diff` and parsed ABI inspection with `json`
- Fetch verified contract ABIs from any Etherscan-compatible explorer with `fetch`
- Hardhat plugin for automatic generation on compile
- Forge shell integration for `forge typegen`

## Install

### Rust CLI

```sh
cargo install abi-typegen
```

Pre-built binaries are available on [GitHub Releases](https://github.com/doublesharp/abi-typegen/releases).

### Hardhat plugin

```sh
pnpm add -D @0xdoublesharp/hardhat-abi-typegen
```

If you only want the packaged binary in a Node project, you can install:

```sh
pnpm add -D @0xdoublesharp/abi-typegen
```

## Quick Start

### Foundry

Build your contracts, then generate bindings:

```sh
forge build
abi-typegen generate
```

Minimal `foundry.toml` configuration:

```toml
[abi-typegen]
out = "src/generated"
target = "viem"            # or "viem,python" or ["viem", "python"]
```

Watch mode is useful while iterating:

```sh
abi-typegen watch
```

For Zod output, install the latest `zod` package in the consuming project.

### Hardhat

```ts
import "@0xdoublesharp/hardhat-abi-typegen";

const config: HardhatUserConfig = {
  solidity: "0.8.34",
  typegen: {
    out: "src/generated",
    target: "viem",
    contracts: ["Token"],
    exclude: ["*Test"],
  },
};

export default config;
```

Bindings are generated automatically on every compile:

```sh
npx hardhat compile
```

### Multi-target generation

Multiple targets can be specified in the config file or on the command line:

```toml
# foundry.toml
[abi-typegen]
target = ["viem", "python", "rust"]   # also accepts "viem,python,rust"
```

```sh
# CLI
abi-typegen generate --target viem,python,rust
```

Multi-target output is written to one subdirectory per target under the configured output path:

```text
src/generated/
  viem/
  python/
  rust/
```

## Fetch and generate from a block explorer

`fetch` pulls a verified ABI, saves it as a local artifact, and immediately generates typed bindings — all in one command:

```sh
abi-typegen fetch --name WETH --network mainnet \
  0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2
```

Output (using configured target, defaults to `viem`):

```text
out/WETH.sol/WETH.json      ← saved artifact
src/generated/WETH.abi.ts
src/generated/WETH.viem.ts
src/generated/index.ts
```

### From a local ABI file

If you already have a raw ABI JSON file (either a bare array `[...]` or a Foundry/Hardhat artifact `{"abi": [...]}`), use `--file` to skip the network request entirely:

```sh
abi-typegen fetch --name WETH --file ./WETH.abi.json
```

### API key

Most explorers require an API key. Set it once in `.env` in the working directory or in the environment:

```sh
# .env
ETHERSCAN_API_KEY=your_key_here
```

Or pass it directly:

```sh
abi-typegen fetch --name WETH --network mainnet --api-key $KEY 0xc02aaa...
```

### Supported networks

Built-in shortcuts for 80+ networks via the `--network` flag:

| Group | Names |
|---|---|
| Ethereum | `mainnet`, `sepolia`, `holesky`, `hoodi` |
| OP Stack | `optimism`, `base`, `blast`, `fraxtal`, `worldchain`, `unichain` |
| Arbitrum | `arbitrum`, `arbitrum-nova`, `arbitrum-sepolia` |
| Polygon | `polygon`, `polygon-amoy` |
| BNB Chain | `bsc`, `opbnb` |
| Avalanche | `avalanche`, `fuji` |
| Other L2s | `linea`, `scroll`, `zksync`, `mantle`, `sonic`, `taiko` |
| Alt L1s | `gnosis`, `moonbeam`, `moonriver`, `celo`, `fantom`, `cronos`, `berachain`, `sei` |
| Newer chains | `hyperevm`, `abstract`, `monad`, `megaeth`, `apechain`, `katana` |

Pass `--url` to use any explorer not in the list:

```sh
abi-typegen fetch --name MyToken --url https://api.sonicscan.org/api 0xabc...
```

All networks listed at [docs.etherscan.io/supported-chains](https://docs.etherscan.io/supported-chains) are supported via the Etherscan V2 unified endpoint.

## Targets

| Target | Flag | Language | Primary ecosystem |
|---|---|---|---|
| viem | `--target viem` | TypeScript | [viem](https://viem.sh/) contract helpers |
| zod | `--target zod` | TypeScript | [Zod](https://zod.dev/) 4 validation schemas |
| wagmi | `--target wagmi` | TypeScript | [wagmi](https://wagmi.sh/) v2 React hooks |
| ethers v6 | `--target ethers` | TypeScript | [ethers](https://docs.ethers.org/v6/) v6 |
| ethers v5 | `--target ethers5` | TypeScript | ethers v5 |
| web3.js | `--target web3js` | TypeScript | [web3.js](https://docs.web3js.org/) v4 |
| Python | `--target python` | Python | [web3.py](https://web3py.readthedocs.io/) |
| Go | `--target go` | Go | [go-ethereum](https://geth.ethereum.org/) |
| Rust | `--target rust` | Rust | [alloy](https://alloy.rs/) |
| Swift | `--target swift` | Swift | [web3swift](https://github.com/web3swift-team/web3swift) |
| C# | `--target csharp` | C# | [Nethereum](https://nethereum.com/) |
| Kotlin | `--target kotlin` | Kotlin | [web3j](https://docs.web3j.io/) |
| Solidity interfaces | `--target solidity` | Solidity | External contract interfaces |
| YAML | `--target yaml` | YAML | Human-readable ABI descriptions |

## Configuration

### Foundry (`foundry.toml`)

```toml
[abi-typegen]
out       = "src/generated"          # output directory
target    = "viem"                   # string, "a,b,c", or ["a", "b", "c"]
wrappers  = true                     # emit typed wrapper files when supported
contracts = []                       # [] = all; or ["MyToken", "Vault"]
exclude   = []                       # glob patterns: ["*Test", "*Mock", "I*"]
```

### Hardhat (`hardhat.config.ts`)

```ts
typegen: {
  out: "src/generated",
  target: "viem",              // string, "a,b,c", or ["a", "b", "c"]
  wrappers: true,
  contracts: [],
  exclude: [],
}
```

### CLI overrides

```sh
abi-typegen generate \
  --artifacts ./out \
  --out ./types \
  --target viem \
  --contracts Token,Vault \
  --exclude "*Test,*Mock" \
  --no-wrappers \
  --clean
```

## Commands

```sh
abi-typegen generate             # write generated bindings
abi-typegen generate --hardhat   # use Hardhat artifact layout
abi-typegen generate --check     # fail if output is stale
abi-typegen generate --clean     # remove stale generated files
abi-typegen diff                 # show what would change without writing
abi-typegen json --pretty        # dump parsed ABI summary as JSON
abi-typegen watch                # watch artifacts and regenerate on change
abi-typegen fetch --name <NAME> --network <NETWORK> <ADDRESS>
                                 # fetch ABI from a block explorer and generate bindings
abi-typegen fetch --name <NAME> --file <ABI.json>
                                 # import a local ABI file and generate bindings
abi-typegen init                 # scaffold [abi-typegen] in foundry.toml
abi-typegen forge-install        # install Forge shell integration
```

## What Gets Generated

For each contract, `abi-typegen` generates files based on the selected target.

- TypeScript wrapper targets emit `<Name>.abi.ts` plus a target-specific wrapper file such as `<Name>.viem.ts` or `<Name>.ethers.ts`
- `zod` emits `<Name>.abi.ts` plus `<Name>.zod.ts` with schemas targeting the current Zod 4 API (`import * as z from 'zod'`)
- `solidity` emits `I<Name>.sol` with interface declarations, events, custom errors, and reconstructed structs from ABI tuples
- Non-TypeScript targets emit one primary file per contract such as `.py`, `.go`, `.rs`, `.swift`, `.cs`, or `.kt`
- Multi-target runs keep each target isolated in its own output directory

Overloaded functions get signature-based names so the output stays unambiguous:

- `deposit(uint256)` -> `depositUint`
- `deposit(uint256,address)` -> `depositUintAddress`

### Example output

**viem**

```ts
export function getTokenContract(address: Address, client: Client) {
  return getContract({ address, abi: TokenAbi, client });
}

export type TokenTransferParams = {
  to: `0x${string}`;
  amount: bigint;
};
```

**Rust**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenTransferParams {
    pub to: Address,
    pub amount: U256,
}
```

For deeper output examples across all targets, see [docs/generated-output.md](docs/generated-output.md).

## Performance

On the sample benchmark in this repo:

```text
abi-typegen:  25 ms
TypeChain:   763 ms
Speedup:     ~30x
```

See [full comparison](docs/comparison.md) with TypeChain, wagmi CLI, abigen, and others.

## CI

### Foundry

```yaml
- run: forge build
- run: abi-typegen generate --check
```

### Hardhat

```yaml
- run: npx hardhat compile
- run: git diff --exit-code src/generated/
```

## Docs

- [docs/configuration.md](docs/configuration.md) for configuration details and target selection
- [docs/generated-output.md](docs/generated-output.md) for target-by-target generated output details
- [docs/forge-integration.md](docs/forge-integration.md) for `forge typegen` shell integration

## Contributing

Issues and PRs are welcome. If you change behavior, include a regression test with the change.
