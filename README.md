# abi-typegen

<p align="center">
  <img src="docs/abi-typegen.png" alt="abi-typegen" width="360" />
</p>

<p align="center"><strong>Typed bindings from Solidity ABI artifacts.</strong></p>
<p align="center">Point it at Foundry or Hardhat output and get client code for the SDK you use.</p>
<p align="center">
  <a href="https://github.com/doublesharp/abi-typegen/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/doublesharp/abi-typegen/ci.yml?branch=main&label=ci"></a>
  <a href="https://github.com/doublesharp/abi-typegen/actions/workflows/coverage.yml"><img alt="Coverage workflow" src="https://img.shields.io/github/actions/workflow/status/doublesharp/abi-typegen/coverage.yml?branch=main&label=coverage"></a>
  <a href="https://doublesharp.github.io/abi-typegen/coverage/"><img alt="Coverage report" src="https://img.shields.io/badge/coverage-report-2ea043"></a>
  <a href="https://crates.io/crates/abi-typegen"><img alt="crates.io" src="https://img.shields.io/crates/v/abi-typegen"></a>
  <a href="https://www.npmjs.com/package/@0xdoublesharp/abi-typegen"><img alt="npm" src="https://img.shields.io/npm/v/@0xdoublesharp/abi-typegen?label=npm"></a>
</p>

`abi-typegen` is a Rust CLI. It reads compiled Solidity artifacts and writes typed
bindings for 14 targets: six TypeScript SDKs, Python, Go, Rust, Swift, C#, Kotlin,
Solidity interfaces, and YAML. It reads Foundry and Hardhat layouts, and it can
generate several targets in one run.

## Why abi-typegen

When you compile a contract, the compiler writes a JSON file called the ABI
that lists its functions, events, and errors. abi-typegen turns that file into typed
code, so your editor and compiler catch a wrong argument before it reaches the
chain.

- One native binary. No Node or Python runtime is needed to generate.
- Reads Foundry `out/` and Hardhat `artifacts/contracts/`.
- `generate --check` fails CI when committed bindings are stale.
- `diff` shows what would change. `json` prints the parsed ABI.
- `fetch` downloads a verified ABI from any Etherscan-compatible explorer.
- A Hardhat plugin regenerates on every compile. For Foundry,
  `abi-typegen forge-install` prints a shell function that adds `forge typegen`.

## Install

Install the CLI with cargo, download a prebuilt binary, or add the npm package
to a JavaScript project.

### Rust CLI

```sh
cargo install abi-typegen
```

Prebuilt binaries are on [GitHub Releases](https://github.com/doublesharp/abi-typegen/releases).

### Hardhat plugin

```sh
pnpm add -D @0xdoublesharp/hardhat-abi-typegen
```

To add only the packaged binary to a Node project:

```sh
pnpm add -D @0xdoublesharp/abi-typegen
```

## Quick start

After your contracts compile, one command writes the typed files into your
project.

### Foundry

Build the contracts, then generate:

```sh
forge build
abi-typegen generate
```

Minimal `foundry.toml`:

```toml
[abi-typegen]
out = "src/generated"
target = "viem"            # or "viem,python" or ["viem", "python"]
```

`watch` regenerates whenever the artifacts change:

```sh
abi-typegen watch
```

The `zod` target emits Zod 4 code, so install `zod@4` in the consuming project.

### Hardhat

```ts
import { defineConfig } from "hardhat/config";
import abiTypegen from "@0xdoublesharp/hardhat-abi-typegen";

export default defineConfig({
  plugins: [abiTypegen],
  solidity: "0.8.34",
  typegen: {
    out: "src/generated",
    target: "viem",
    contracts: ["Token"],
    exclude: ["*Test"],
  },
});
```

The plugin generates bindings on every compile:

```sh
npx hardhat compile
```

Hardhat 2 configs can still `import "@0xdoublesharp/hardhat-abi-typegen"`, which
loads a CommonJS fallback. The fallback is also available directly at
`@0xdoublesharp/hardhat-abi-typegen/hardhat2`.

### Multi-target generation

Set several targets in the config file or on the command line:

```toml
# foundry.toml
[abi-typegen]
target = ["viem", "python", "rust"]   # also accepts "viem,python,rust"
```

```sh
abi-typegen generate --target viem,python,rust
```

Each target gets its own subdirectory under the output path:

```text
src/generated/
  viem/
  python/
  rust/
```

## Fetch and generate from a block explorer

You can generate bindings for a contract you didn't compile, as long as its
source is verified on an Etherscan-compatible explorer. `fetch` downloads the
ABI, saves it as a local artifact, and generates bindings in one command:

```sh
abi-typegen fetch --name WETH --network mainnet \
  0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2
```

With the default `viem` target, that writes:

```text
out/WETH.sol/WETH.json      ← saved artifact
src/generated/WETH.abi.ts
src/generated/WETH.viem.ts
src/generated/index.ts
```

### From a local ABI file

`--file` skips the network request. It accepts a bare ABI array (`[...]`) or a
Foundry/Hardhat artifact (`{"abi": [...]}`):

```sh
abi-typegen fetch --name WETH --file ./WETH.abi.json
```

### API key

Most explorers need an API key. Put it in `.env` in the working directory or
export it:

```sh
# .env
ETHERSCAN_API_KEY=your_key_here
```

You can also pass it per command:

```sh
abi-typegen fetch --name WETH --network mainnet --api-key $KEY 0xc02aaa...
```

### Supported networks

`--network` has shortcuts for more than 80 networks, including these:

| Group        | Names                                                                             |
| ------------ | --------------------------------------------------------------------------------- |
| Ethereum     | `mainnet`, `sepolia`, `holesky`, `hoodi`                                          |
| OP Stack     | `optimism`, `base`, `blast`, `fraxtal`, `worldchain`, `unichain`                  |
| Arbitrum     | `arbitrum`, `arbitrum-nova`, `arbitrum-sepolia`                                   |
| Polygon      | `polygon`, `polygon-amoy`                                                         |
| BNB Chain    | `bsc`, `opbnb`                                                                    |
| Avalanche    | `avalanche`, `fuji`                                                               |
| Other L2s    | `linea`, `scroll`, `zksync`, `mantle`, `sonic`, `taiko`                           |
| Alt L1s      | `gnosis`, `moonbeam`, `moonriver`, `celo`, `fantom`, `cronos`, `berachain`, `sei` |
| Newer chains | `hyperevm`, `abstract`, `monad`, `megaeth`, `apechain`, `katana`                  |

For an explorer that isn't listed, pass its API URL with `--url`:

```sh
abi-typegen fetch --name MyToken --url https://api.sonicscan.org/api 0xabc...
```

The tool uses the Etherscan V2 unified endpoint, so every chain on
[docs.etherscan.io/supported-chains](https://docs.etherscan.io/supported-chains)
works.

## Targets

Each target writes code for a specific library, so pick the one your app
already uses.

| Target              | Flag                | Language   | Primary ecosystem                                        |
| ------------------- | ------------------- | ---------- | -------------------------------------------------------- |
| viem                | `--target viem`     | TypeScript | [viem](https://viem.sh/) contract helpers                |
| zod                 | `--target zod`      | TypeScript | [Zod](https://zod.dev/) 4 validation schemas             |
| wagmi               | `--target wagmi`    | TypeScript | [wagmi](https://wagmi.sh/) v2 and v3 React hooks         |
| ethers v6           | `--target ethers`   | TypeScript | [ethers](https://docs.ethers.org/v6/) v6                 |
| ethers v5           | `--target ethers5`  | TypeScript | ethers v5                                                |
| web3.js             | `--target web3js`   | TypeScript | [web3.js](https://docs.web3js.org/) v4                   |
| Python              | `--target python`   | Python     | [web3.py](https://web3py.readthedocs.io/)                |
| Go                  | `--target go`       | Go         | [go-ethereum](https://geth.ethereum.org/)                |
| Rust                | `--target rust`     | Rust       | [alloy](https://alloy.rs/)                               |
| Swift               | `--target swift`    | Swift      | [web3swift](https://github.com/web3swift-team/web3swift) |
| C#                  | `--target csharp`   | C#         | [Nethereum](https://nethereum.com/)                      |
| Kotlin              | `--target kotlin`   | Kotlin     | [web3j](https://docs.web3j.io/)                          |
| Solidity interfaces | `--target solidity` | Solidity   | External contract interfaces                             |
| YAML                | `--target yaml`     | YAML       | Human-readable ABI descriptions                          |

## Configuration

Settings live in `foundry.toml` or `hardhat.config.ts`, so every run uses the
same options. Command-line flags override them for a single run.

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

`generate` writes the bindings. The other commands check, preview, watch, or
fetch.

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

## What gets generated

Each contract produces one or two files. TypeScript targets get the ABI plus a
typed wrapper, and other languages get a single file:

- TypeScript wrapper targets emit `<Name>.abi.ts` and a wrapper such as
  `<Name>.viem.ts` or `<Name>.ethers.ts`.
- `zod` emits `<Name>.abi.ts` and `<Name>.zod.ts`, written against Zod 4
  (`import * as z from 'zod'`).
- `solidity` emits `I<Name>.sol` with the interface, events, custom errors, and
  structs rebuilt from ABI tuples.
- Other targets emit one file per contract (`.py`, `.go`, `.rs`, `.swift`, `.cs`,
  `.kt`, or `.yaml`). Rust files are snake_case with a generated `mod.rs`.
- Go, Rust, Swift, and Kotlin files embed the ABI, every selector and signature,
  and a named type for each tuple.
- Multi-target runs write each target to its own directory.

Overloaded functions in TypeScript get a suffix built from their parameter types:

- `deposit(uint256)` -> `depositUint256`
- `deposit(uint256,address)` -> `depositUint256Address`

Go, Rust, Swift, and Kotlin number overloads the way their SDKs do (`Deposit`,
`Deposit0` in Go; `deposit_0Call` in alloy).

Generated names avoid language keywords, SDK type names, and collisions after
case conversion. Kotlin tuples provide web3j-typed constructors for decoding,
and Go indexed dynamic event fields use `common.Hash`. See
[generated output](docs/generated-output.md) for naming rules and SDK limitations.

Regenerate bindings when upgrading and review the
[changelog](CHANGELOG.md) for changes to generated names.

Wrappers also type transaction options. Payable functions accept a `value` in
every TypeScript wrapper: ethers `overrides`, the wagmi `write` options, web3
`send`, and viem's own `write` options.

### Example output

viem:

```ts
export function getTokenContract<TClient extends Client>(
  address: Address,
  client: TClient,
): GetContractReturnType<typeof TokenAbi, TClient> {
  return getContract({ address, abi: TokenAbi, client });
}

export type TokenTransferParams = {
  to: `0x${string}`;
  amount: bigint;
};
```

Rust:

```rust
alloy::sol! {
    #[sol(rpc, abi, all_derives, extra_derives(serde::Serialize, serde::Deserialize))]
    contract Token {
        event Transfer(address indexed from, address indexed to, uint256 value);
        function transfer(address to, uint256 amount) external returns (bool);
    }
}
```

[docs/generated-output.md](docs/generated-output.md) covers every target in more
detail.

## Performance

Generation speed depends on your machine and contracts, so the repo includes a
benchmark you can run yourself. `./e2e/bench.sh 10` compares the sample
generation workflows.
Results depend on tool versions, targets, contracts, and whether you count
build and task overhead. [Comparison and benchmark guidance](docs/comparison.md)
explains the method and its limits.

## CI

A CI check catches a contract change that was committed without regenerated
bindings.

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

This README covers the basics, and `docs/` has the details. Start with the
[documentation index](docs/README.md):

- [Installation](docs/installation.md) and download verification
- [Configuration](docs/configuration.md) and [generated output](docs/generated-output.md)
- [Forge integration](docs/forge-integration.md)
- [Development](docs/development.md) and [optional build/cache storage](docs/development-storage.md)
- [Release process](docs/releasing.md) and [changelog](CHANGELOG.md)

## Contributing

Bug reports and fixes are welcome. [CONTRIBUTING.md](CONTRIBUTING.md) covers
setup, code conventions, tests, and pull requests. Bug fixes need a regression
test.

## Disposable build storage

This section is optional and only matters if you work on abi-typegen itself.
Builds and package installs use their normal local paths by default, and nothing
requires a Scratch volume or compiler cache. To send build output and caches to
another directory, run `python3 .cargo/setup-scratch.py --root PATH` once. Cargo,
pnpm, and Make then read that saved setting. `python3 .cargo/setup-scratch.py --disable`
turns it off. [The storage guide](docs/development-storage.md) covers requirements,
moving the location, and keeping existing data.
