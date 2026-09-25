<p align="center">
  <img src="https://raw.githubusercontent.com/doublesharp/abi-typegen/main/docs/abi-typegen.png" alt="abi-typegen" width="360" />
</p>

<p align="center"><strong>Fast typed bindings from Solidity ABI artifacts.</strong></p>
<p align="center">22 targets &middot; 16 languages &middot; Foundry &amp; Hardhat</p>

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

See the [installation guide](https://github.com/doublesharp/abi-typegen/blob/main/docs/installation.md)
for supported platforms and download troubleshooting.

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

There are 25 CLI targets across 18 languages. Unity uses the `csharp` target
with an adapter package.

### JavaScript and TypeScript

| Target    | Framework or SDK                         | What you get                                     |
| --------- | ---------------------------------------- | ------------------------------------------------ |
| `viem`    | [viem](https://viem.sh/)                 | Typed contract helpers and ABI                   |
| `wagmi`   | [wagmi](https://wagmi.sh/)               | React hooks for reads, writes, and events        |
| `ethers`  | [ethers v6](https://docs.ethers.org/v6/) | Typed contract interfaces and connection helpers |
| `ethers5` | [ethers v5](https://docs.ethers.org/v5/) | Typed contract interfaces and connection helpers |
| `web3js`  | [web3.js v4](https://docs.web3js.org/)   | Typed contract methods                           |
| `zod`     | [Zod 4](https://zod.dev/)                | Validation schemas and ABI                       |

### Native languages

| Language | Target   | Runtime or SDK                                                                                              | What you get                                                      |
| -------- | -------- | ----------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| C        | `c`      | [Shared Rust runtime](https://github.com/doublesharp/abi-typegen/blob/main/docs/native-bindings.md#c-and-c) | C11 codecs and client helpers with explicit ownership             |
| C++      | `cpp`    | [Shared Rust runtime](https://github.com/doublesharp/abi-typegen/blob/main/docs/native-bindings.md#c-and-c) | C++17 codecs and client helpers with automatic cleanup            |
| C#       | `csharp` | [Nethereum](https://docs.nethereum.com/)                                                                    | Typed DTOs, contract methods, and deployment                      |
| Dart     | `dart`   | [web3dart](https://pub.dev/packages/web3dart)                                                               | Typed values, codecs, calls, and transactions                     |
| Elixir   | `elixir` | [Ethers](https://ethers.hexdocs.pm/Ethers.html)                                                             | Transaction data, reads, sends, events, and errors                |
| Go       | `go`     | [go-ethereum](https://geth.ethereum.org/docs/developers)                                                    | Typed calls, transactions, deployment, and events                 |
| Java     | `java`   | [web3j](https://docs.web3j.io/latest/)                                                                      | Typed values, codecs, calls, and transactions                     |
| Kotlin   | `kotlin` | [web3j](https://docs.web3j.io/latest/)                                                                      | Typed values, codecs, calls, and transactions                     |
| PHP      | `php`    | [Brick Math](https://github.com/brick/math), [ethereum-tx](https://github.com/web3p/ethereum-tx), cURL      | Codecs, RPC reads, signed legacy transactions, receipts, and logs |
| Python   | `python` | [web3.py](https://web3py.readthedocs.io/en/stable/)                                                         | Reads, transaction builders, codecs, events, and errors           |
| Ruby     | `ruby`   | [eth](https://github.com/q9f/eth.rb)                                                                        | Calls, transaction builders, codecs, events, and errors           |
| Rust     | `rust`   | [Alloy](https://alloy.rs/introduction/getting-started/)                                                     | `sol!` types, codecs, and contract instances                      |
| Swift    | `swift`  | [web3swift](https://github.com/web3swift-team/web3swift)                                                    | Typed values, codecs, and contract operations                     |

### Game engines

| Engine                                                                        | Target                                                                                                                                 | What you get                                                             |
| ----------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| [Unity](https://docs.unity3d.com/Manual/index.html)                           | `csharp` + [Unity adapter](https://github.com/doublesharp/abi-typegen/tree/main/integrations/unity/com.doublesharp.abi-typegen.unity/) | C# bindings with Unity HTTP transport and object lifecycle support       |
| [Godot](https://docs.godotengine.org/en/stable/)                              | `godot`                                                                                                                                | GDScript bindings and a native extension for codecs and asynchronous RPC |
| [Unreal Engine](https://dev.epicgames.com/documentation/en-us/unreal-engine/) | `unreal`                                                                                                                               | C++ codecs and Blueprint nodes for asynchronous scalar reads             |

See [engine packages](https://github.com/doublesharp/abi-typegen/blob/main/integrations/README.md) for setup and
[native bindings](https://github.com/doublesharp/abi-typegen/blob/main/docs/native-bindings.md) for supported features and platforms.

### Shell and ABI formats

| Target     | Works with                                                                                                    | What you get                                                        |
| ---------- | ------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| `shell`    | [Bash](https://www.gnu.org/software/bash/manual/bash.html) and [Foundry cast](https://www.getfoundry.sh/cast) | Sourceable helpers for encoding, reads, sends, deployment, and logs |
| `solidity` | [Solidity](https://docs.soliditylang.org/en/latest/)                                                          | Interfaces, tuple structs, events, and errors                       |
| `yaml`     | [YAML](https://yaml.org/)                                                                                     | Readable ABI descriptions                                           |

Zod, Solidity, and YAML describe or validate contracts; they do not submit
transactions. See [native bindings](https://github.com/doublesharp/abi-typegen/blob/main/docs/native-bindings.md) for SDK versions
and [configuration](https://github.com/doublesharp/abi-typegen/blob/main/docs/configuration.md) for target aliases.

### COBOL

| Target  | Works with                                                                                | What you get                                          |
| ------- | ----------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| `cobol` | [GnuCOBOL](https://gnucobol.sourceforge.io/doc/gnucobol.html) and the shared Rust runtime | Reads taking one address and returning one uint256 🤷 |

Target aliases: `ethers6` → ethers, `web3` → web3js, `cs` → csharp, `kt` → kotlin, `sol` → solidity, `yml` → yaml, `c++` → cpp

## Commands

```sh
npx abi-typegen generate                 # write generated bindings
npx abi-typegen generate --check         # fail if output is stale (CI)
npx abi-typegen generate --clean         # remove stale generated files
npx abi-typegen diff                     # show what would change (dry run)
npx abi-typegen json --pretty            # dump parsed ABI as JSON
npx abi-typegen watch                    # watch artifacts and regenerate
npx abi-typegen init                     # add Foundry configuration
npx abi-typegen forge-install --shell zsh # print Forge shell integration
npx abi-typegen help generate            # show command help
npx abi-typegen --version                # print the installed version
npx abi-typegen fetch --name WETH \
  --network mainnet 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2          # fetch ABI from block explorer
npx abi-typegen fetch --name WETH \
  --file ./WETH.abi.json                 # import a local ABI file
```

## Generation options

| Option                 | Description                                            |
| ---------------------- | ------------------------------------------------------ |
| `--config <path>`      | Configuration file                                     |
| `--package <name>`     | Package, namespace, or Unreal module name              |
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
for dependencies, ownership rules, supported features, and SDK limits.
