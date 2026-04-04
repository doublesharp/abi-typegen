<p align="center">
  <img src="https://raw.githubusercontent.com/doublesharp/abi-typegen/main/docs/abi-typegen.png" alt="abi-typegen" width="360" />
</p>

<p align="center"><strong>Fast typed bindings from Solidity ABI artifacts.</strong></p>
<p align="center">14 targets &middot; 8 languages &middot; Foundry &amp; Hardhat &middot; ~30x faster than TypeChain</p>

# @0xdoublesharp/abi-typegen

Pre-built binary for [abi-typegen](https://github.com/doublesharp/abi-typegen). Reads compiled Solidity artifacts and generates production-ready typed bindings — no native toolchain required.

## Install

```sh
npm install -D @0xdoublesharp/abi-typegen
yarn add -D @0xdoublesharp/abi-typegen
pnpm add -D @0xdoublesharp/abi-typegen
```

## Usage

```sh
# Single target
npx abi-typegen generate --target viem

# Hardhat artifact layout
npx abi-typegen generate --hardhat --target ethers

# Multi-target (each gets its own output subdirectory)
npx abi-typegen generate --target viem,python,rust
```

## Targets

| Target | Flag | Language | Ecosystem |
|---|---|---|---|
| viem | `--target viem` | TypeScript | [viem](https://viem.sh/) |
| zod | `--target zod` | TypeScript | [Zod](https://zod.dev/) 4 |
| wagmi | `--target wagmi` | TypeScript | [wagmi](https://wagmi.sh/) v2 |
| ethers v6 | `--target ethers` | TypeScript | [ethers](https://docs.ethers.org/v6/) v6 |
| ethers v5 | `--target ethers5` | TypeScript | ethers v5 |
| web3.js | `--target web3js` | TypeScript | [web3.js](https://docs.web3js.org/) v4 |
| Python | `--target python` | Python | [web3.py](https://web3py.readthedocs.io/) |
| Go | `--target go` | Go | [go-ethereum](https://geth.ethereum.org/) |
| Rust | `--target rust` | Rust | [alloy](https://alloy.rs/) |
| Swift | `--target swift` | Swift | [web3swift](https://github.com/web3swift-team/web3swift) |
| C# | `--target csharp` | C# | [Nethereum](https://nethereum.com/) |
| Kotlin | `--target kotlin` | Kotlin | [web3j](https://docs.web3j.io/) |
| Solidity | `--target solidity` | Solidity | External interfaces |
| YAML | `--target yaml` | YAML | Human-readable ABI descriptions |

Target aliases: `ethers6` → ethers, `web3` → web3js, `cs` → csharp, `kt` → kotlin, `sol` → solidity, `yml` → yaml

## Commands

```sh
npx abi-typegen generate                 # write generated bindings
npx abi-typegen generate --check         # fail if output is stale (CI)
npx abi-typegen generate --clean         # remove stale generated files
npx abi-typegen diff                     # show what would change (dry run)
npx abi-typegen json --pretty            # dump parsed ABI as JSON
npx abi-typegen watch                    # watch artifacts and regenerate
npx abi-typegen fetch --name WETH \
  --network mainnet 0xc02aaa...          # fetch ABI from block explorer
npx abi-typegen fetch --name WETH \
  --file ./WETH.abi.json                 # import a local ABI file
```

## CLI Options

| Option | Description |
|---|---|
| `--target <name>` | Target name or comma-separated names (see table above) |
| `--artifacts <path>` | Path to compiled artifacts directory |
| `--out <path>` | Output directory |
| `--hardhat` | Use Hardhat artifact layout (`artifacts/contracts/`) |
| `--contracts <names>` | Comma-separated contract allowlist |
| `--exclude <patterns>` | Comma-separated glob patterns (e.g. `*Test,*Mock`) |
| `--no-wrappers` | Disable wrapper function generation |
| `--check` | Exit non-zero if output is stale |
| `--clean` | Remove stale generated files |

## Fetch Networks

The `fetch` command supports 80+ networks via `--network`. Some examples:

| Group | Networks |
|---|---|
| Ethereum | `mainnet` (default), `sepolia`, `holesky`, `hoodi` |
| OP Stack | `optimism`, `base`, `blast`, `fraxtal`, `worldchain`, `unichain` |
| Arbitrum | `arbitrum`, `arbitrum-nova`, `arbitrum-sepolia` |
| Polygon | `polygon`, `polygon-amoy`, `polygon-zkevm` |
| BNB Chain | `bsc`, `opbnb` |
| Avalanche | `avalanche`, `fuji` |
| L2s | `linea`, `scroll`, `zksync`, `mantle`, `sonic`, `taiko`, `swellchain` |
| Alt L1s | `gnosis`, `celo`, `moonbeam`, `moonriver`, `fantom`, `cronos`, `berachain`, `sei` |
| Newer chains | `hyperevm`, `abstract`, `monad`, `megaeth`, `apechain`, `katana` |
| Other | `manta`, `metis`, `xdc`, `bittorrent` |

Pass `--url <URL>` for any Etherscan-compatible explorer not in the list.

## Hardhat Plugin

For automatic generation on compile, use [@0xdoublesharp/hardhat-abi-typegen](https://www.npmjs.com/package/@0xdoublesharp/hardhat-abi-typegen).

## Configuration

Multi-target generation can also be configured in `foundry.toml`:

```toml
[abi-typegen]
target = ["viem", "python", "rust"]   # also accepts "viem,python,rust"
```

## Notes

- When using `--target zod`, install the latest `zod` package in the consuming project
- Multi-target runs write each target to its own subdirectory under the output path

See [github.com/doublesharp/abi-typegen](https://github.com/doublesharp/abi-typegen) for full documentation.
