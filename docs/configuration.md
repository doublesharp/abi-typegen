# Configuration

## Foundry (`foundry.toml`)

`abi-typegen` reads configuration from the `[abi-typegen]` section. Run `abi-typegen init` to scaffold it.

```toml
[abi-typegen]
out       = "src/generated"
target    = "viem"
wrappers  = true
contracts = []
exclude   = []
```

### Options

#### `out`

**Type:** path — **Default:** `"src/generated"`

Output directory for generated files. Created if it does not exist.

#### `target`

**Type:** string — **Default:** `"viem"`

Code generation target. One of: `viem`, `zod`, `wagmi`, `ethers`, `ethers5`, `web3js`, `python`, `go`, `rust`, `swift`, `csharp`, `kotlin`, `solidity`.

#### `wrappers`

**Type:** bool — **Default:** `true`

Set to `false` to skip wrapper files for wrapper-oriented TypeScript targets. This affects `viem`, `wagmi`, `ethers`, `ethers5`, and `web3js`. It does not suppress primary outputs such as `*.zod.ts`, `I*.sol`, or other non-TypeScript bindings.

#### `contracts`

**Type:** array of strings — **Default:** `[]`

Limit generation to named contracts. Empty means all.

#### `exclude`

**Type:** array of strings — **Default:** `[]`

Skip contracts matching glob patterns. Supports `*` wildcards.

```toml
[abi-typegen]
exclude = ["*Test", "*Mock", "I*"]
```

## Hardhat (`hardhat.config.ts`)

```typescript
import "@0xdoublesharp/hardhat-abi-typegen";

const config: HardhatUserConfig = {
  typegen: {
    out: "src/generated",
    target: "viem",
    wrappers: true,
    contracts: [],
    exclude: [],
  },
};
```

## CLI flags

Flags override config file values:

```sh
abi-typegen generate \
  --artifacts ./out \
  --out ./types \
  --target viem,python \
  --contracts Token,Vault \
  --exclude "*Test,*Mock" \
  --no-wrappers \
  --clean \
  --check \
  --hardhat
```

When `--target` contains multiple comma-separated values, abi-typegen writes each target into its own subdirectory under `--out`. For example, `--out ./types --target viem,python` produces `./types/viem` and `./types/python`.

Use comma-separated multi-target generation instead of the removed `all` and `all-ts` aliases.

| Flag | Description |
|------|-------------|
| `--artifacts <path>` | Path to compiled artifacts |
| `--out <path>` | Output directory |
| `--target <target>` | Target (comma-separated for multiple) |
| `--contracts <names>` | Limit to named contracts |
| `--exclude <patterns>` | Skip contracts matching globs |
| `--no-wrappers` | Only emit ABI files |
| `--clean` | Remove stale files |
| `--check` | Exit non-zero if output is stale |
| `--hardhat` | Use Hardhat artifact layout |

## `fetch` command

Pulls a contract ABI from a block explorer (or a local file), saves it as a Foundry artifact, and immediately generates typed bindings — all in one step.

```sh
# From a block explorer
abi-typegen fetch --name <NAME> --network <NETWORK> <ADDRESS>

# From a local ABI file (no API key needed)
abi-typegen fetch --name <NAME> --file <ABI.json>
```

| Flag | Description |
|------|-------------|
| `--name <NAME>` | Contract name for the artifact and generated files (required) |
| `--network <NETWORK>` | Named network shortcut, default `mainnet` |
| `--url <URL>` | Full Etherscan-compatible API URL — overrides `--network` |
| `--file <PATH>` | Load ABI from a local file instead of fetching; accepts a raw ABI array `[...]` or a full artifact `{"abi": [...]}` |
| `--api-key <KEY>` | Explorer API key (or `ETHERSCAN_API_KEY` env var or `.env` file) |
| `--artifacts <path>` | Artifacts directory (default: from foundry.toml or `out/`) |
| `--force` | Overwrite an existing artifact |

Generation uses the same configuration as `generate` (`target`, `out`, etc. from `foundry.toml` or CLI flags on the outer command).

### API key

Most explorers require an API key. Provide it via `--api-key`, the `ETHERSCAN_API_KEY` environment variable, or a `.env` file in the working directory:

```sh
# .env
ETHERSCAN_API_KEY=your_key_here
```

The `.env` file is loaded automatically before argument parsing.

### Network shortcuts

All chains from the [Etherscan V2 supported-chains list](https://docs.etherscan.io/supported-chains) are available by name. Etherscan-operated chains route through the V2 unified endpoint; independent explorers use their own API URL.

**Etherscan V2 chains (selected):**

| Name(s) | Chain |
|---|---|
| `mainnet`, `ethereum`, `eth` | Ethereum (1) |
| `sepolia` | Sepolia testnet (11155111) |
| `holesky` | Holesky testnet (17000) |
| `optimism`, `op` | OP Mainnet (10) |
| `base` | Base (8453) |
| `blast` | Blast (81457) |
| `frax`, `fraxtal` | Fraxtal (252) |
| `world`, `worldchain` | World Chain (480) |
| `arbitrum`, `arb` | Arbitrum One (42161) |
| `arbitrum-nova` | Arbitrum Nova (42170) |
| `polygon`, `matic` | Polygon (137) |
| `bsc`, `bnb` | BNB Smart Chain (56) |
| `avalanche`, `avax` | Avalanche C-Chain (43114) |
| `linea` | Linea (59144) |
| `scroll` | Scroll (534352) |
| `gnosis`, `xdai` | Gnosis (100) |
| `mantle` | Mantle (5000) |
| `celo` | Celo (42220) |
| `moonbeam`, `glmr` | Moonbeam (1284) |
| `moonriver`, `movr` | Moonriver (1285) |
| `taiko` | Taiko (167000) |
| `sonic` | Sonic (146) |
| `unichain` | Unichain (130) |
| `hyperevm`, `hype` | HyperEVM (999) |
| `abstract` | Abstract (2741) |
| `berachain`, `bera` | Berachain (80094) |
| `monad` | Monad (143) |
| `apechain`, `ape` | ApeChain (33139) |
| `sei` | Sei (1329) |
| `megaeth` | MegaETH (4326) |
| `katana` | Katana (747474) |
| `xdc` | XDC (50) |
| `btt`, `bittorrent` | BitTorrent Chain (199) |

**Independent explorers:**

| Name(s) | Explorer |
|---|---|
| `polygon-zkevm`, `zkevm` | Polygonscan zkEVM |
| `zksync` | zkSync Era native explorer |
| `fantom`, `ftm` | Ftmscan |
| `cronos`, `cro` | Cronoscan |
| `metis` | Andromeda explorer |
| `manta` | Manta Pacific explorer |

Pass `--url` to use any other explorer:

```sh
abi-typegen fetch --name MyToken \
  --url https://api.routescan.io/v2/network/mainnet/evm/56/etherscan \
  0xabc...
```
