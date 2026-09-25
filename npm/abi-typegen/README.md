<p align="center">
  <img src="https://raw.githubusercontent.com/doublesharp/abi-typegen/main/docs/abi-typegen.png" alt="abi-typegen" width="360" />
</p>

<p align="center"><strong>Fast typed bindings from Solidity ABI artifacts.</strong></p>
<p align="center">19 targets &middot; 13 languages &middot; Foundry &amp; Hardhat</p>

# @0xdoublesharp/abi-typegen

Pre-built binary for [abi-typegen](https://github.com/doublesharp/abi-typegen). Reads compiled Solidity artifacts and generates typed bindings. Generating files requires no native toolchain; compiling the generated output requires the target language and its SDK.

## Install

```sh
npm install -D @0xdoublesharp/abi-typegen
yarn add -D @0xdoublesharp/abi-typegen
pnpm add -D @0xdoublesharp/abi-typegen
```

The installer downloads the platform archive over HTTPS and checks its SHA-256
against `checksums.json` bundled in the npm package before extracting it. Missing
hashes, mismatches, and incomplete archives fail installation. Downloads retry
transient HTTP errors, including 504, up to three times with exponential backoff.
Connection, transfer, and overall download deadlines prevent indefinite waits.

Hashes are pinned during npm publication after checking every GitHub release
archive. This detects corruption and later archive replacement; it trusts the
release and npm publishing process and is not independent build attestation.

Re-running postinstall verifies a fresh archive before replacing the binary.
Installations need `curl` and `tar`. Linux archives currently target glibc.

For maintainers, release tags generate and publish `checksums.json`. Run
`node npm/abi-typegen/scripts/release-checksums.mjs prepare` after the matching
release is available and before packing or publishing the npm package. Ordinary
`npm pack` refuses missing or invalid hashes. The publish workflow performs this
preparation explicitly because it publishes with lifecycle scripts disabled.
See the [installation guide](https://github.com/doublesharp/abi-typegen/blob/main/docs/installation.md)
and [release guide](https://github.com/doublesharp/abi-typegen/blob/main/docs/releasing.md)
for details.

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

| Target    | Flag       | Language   | Ecosystem                                                |
| --------- | ---------- | ---------- | -------------------------------------------------------- |
| viem      | `viem`     | TypeScript | [viem](https://viem.sh/)                                 |
| zod       | `zod`      | TypeScript | [Zod](https://zod.dev/) 4                                |
| wagmi     | `wagmi`    | TypeScript | [wagmi](https://wagmi.sh/) v2                            |
| ethers v6 | `ethers`   | TypeScript | [ethers](https://docs.ethers.org/v6/) v6                 |
| ethers v5 | `ethers5`  | TypeScript | ethers v5                                                |
| web3.js   | `web3js`   | TypeScript | [web3.js](https://docs.web3js.org/) v4                   |
| Python    | `python`   | Python     | [web3.py](https://web3py.readthedocs.io/)                |
| Go        | `go`       | Go         | [go-ethereum](https://geth.ethereum.org/)                |
| Rust      | `rust`     | Rust       | [alloy](https://alloy.rs/)                               |
| Swift     | `swift`    | Swift      | [web3swift](https://github.com/web3swift-team/web3swift) |
| C#        | `csharp`   | C#         | [Nethereum](https://nethereum.com/)                      |
| Kotlin    | `kotlin`   | Kotlin     | [web3j](https://docs.web3j.io/)                          |
| Solidity  | `solidity` | Solidity   | External interfaces                                      |
| Java      | `java`     | Java       | [web3j](https://docs.web3j.io/)                          |
| Dart      | `dart`     | Dart       | [web3dart](https://pub.dev/packages/web3dart)            |
| PHP       | `php`      | PHP        | PHP 8.2+, Brick Math, cURL, `web3p/ethereum-tx`          |
| C         | `c`        | C11        | Shared Rust ABI runtime                                  |
| C++       | `cpp`      | C++17      | Shared Rust ABI runtime                                  |
| YAML      | `yaml`     | Data       | Human-readable ABI descriptions                          |

Target aliases: `ethers6` → ethers, `web3` → web3js, `cs` → csharp, `kt` → kotlin, `sol` → solidity, `yml` → yaml, `c++` → cpp

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

| Option                 | Description                                            |
| ---------------------- | ------------------------------------------------------ |
| `--target <name>`      | Target name or comma-separated names (see table above) |
| `--artifacts <path>`   | Path to compiled artifacts directory                   |
| `--out <path>`         | Output directory                                       |
| `--hardhat`            | Use Hardhat artifact layout (`artifacts/contracts/`)   |
| `--contracts <names>`  | Comma-separated contract allowlist                     |
| `--exclude <patterns>` | Comma-separated glob patterns (e.g. `*Test,*Mock`)     |
| `--no-wrappers`        | Disable wrapper function generation                    |
| `--check`              | Exit non-zero if output is stale                       |
| `--clean`              | Remove stale generated files                           |

## Fetch Networks

The `fetch` command supports 80+ networks via `--network`. Some examples:

| Group        | Networks                                                                          |
| ------------ | --------------------------------------------------------------------------------- |
| Ethereum     | `mainnet` (default), `sepolia`, `holesky`, `hoodi`                                |
| OP Stack     | `optimism`, `base`, `blast`, `fraxtal`, `worldchain`, `unichain`                  |
| Arbitrum     | `arbitrum`, `arbitrum-nova`, `arbitrum-sepolia`                                   |
| Polygon      | `polygon`, `polygon-amoy`, `polygon-zkevm`                                        |
| BNB Chain    | `bsc`, `opbnb`                                                                    |
| Avalanche    | `avalanche`, `fuji`                                                               |
| L2s          | `linea`, `scroll`, `zksync`, `mantle`, `sonic`, `taiko`, `swellchain`             |
| Alt L1s      | `gnosis`, `celo`, `moonbeam`, `moonriver`, `fantom`, `cronos`, `berachain`, `sei` |
| Newer chains | `hyperevm`, `abstract`, `monad`, `megaeth`, `apechain`, `katana`                  |
| Other        | `manta`, `metis`, `xdc`, `bittorrent`                                             |

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

## Native contract bindings

Go, Swift, Kotlin, C#, Java, and Dart wrappers use their runtime SDKs for contract
calls and transactions. PHP includes ABI codecs, JSON-RPC reads, legacy transaction
signing, receipt and log queries, and event/error decoding. It requires PHP 8.2+ with
cURL, GMP, mbstring, and iconv. It does not generate EIP-1559 or deployment helpers. C/C++ bindings link `abi-typegen-runtime` and accept a
caller-supplied transport/signing adapter. `--no-wrappers` preserves primary ABI
metadata and value types. See the [native binding guide](https://github.com/doublesharp/abi-typegen/blob/main/docs/native-bindings.md)
for dependencies, ownership rules, tested capabilities, and SDK limits.
