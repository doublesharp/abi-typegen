# Configuration

Run abi-typegen from the contract project's root. Compile contracts first; the
CLI reads artifacts rather than compiling Solidity. See [Forge integration](forge-integration.md)
for build and watch workflows.

## Foundry configuration

The CLI reads `foundry.toml` in the working directory. Use
`--config path/to/foundry.toml` to select another TOML file. Paths in the file remain
relative to the working directory. A missing file uses defaults.

```toml
[profile.default]
out = "out"

[abi-typegen]
out = "src/generated"
target = ["viem", "python"]
wrappers = true
contracts = []
exclude = ["*Test", "*Mock"]
```

`[profile.default].out` is the compiled artifact directory.
`[abi-typegen].out` is the generated binding directory. The parser reads the
`default` Foundry profile; use `--artifacts` when building into a different path.
`abi-typegen init` adds an `[abi-typegen]` scaffold to `foundry.toml`.

| Setting     | Default           | Meaning                                                           |
| ----------- | ----------------- | ----------------------------------------------------------------- |
| `out`       | `"src/generated"` | Generated output directory; created as needed                     |
| `target`    | `"viem"`          | One target, a comma-separated string, or an array of target names |
| `wrappers`  | `true`            | Emit callable wrappers for supported targets                      |
| `contracts` | `[]`              | Exact contract names to include; empty selects all                |
| `exclude`   | `[]`              | Contract-name patterns to exclude; `*` matches any sequence       |
| `package`   | `"contracts"`     | Go package name, Kotlin/Java package, or PHP namespace            |

Targets are `viem`, `zod`, `wagmi`, `ethers`, `ethers5`, `web3js`, `python`, `go`,
`rust`, `swift`, `csharp`, `kotlin`, `java`, `dart`, `php`, `c`, `cpp`, `solidity`, and `yaml`.
Aliases are `ethers6` for `ethers`, `web3` for `web3js`, `cs` for `csharp`, `kt` for
`kotlin`, `c++` for `cpp`, `sol` for `solidity`, and `yml` for `yaml`.

These target settings are equivalent:

```toml
target = "viem,python"
```

```toml
target = ["viem", "python"]
```

A single target writes directly under `out`. Multiple targets write under named
subdirectories such as `src/generated/viem/` and `src/generated/python/`.
Use explicit target lists; `all` and `all-ts` are not supported.

Setting `wrappers = false` suppresses wrappers for `viem`, `wagmi`, `ethers`,
`ethers5`, and `web3js` while retaining their ABI modules. For Rust it removes
alloy's `rpc` contract instance and keeps the types, ABI, and selectors. For Go,
Python, Swift, Kotlin, C#, Java, Dart, PHP, C, and C++ it omits callable wrappers while keeping
primary ABI metadata and value types. It does
not suppress Zod schemas, Solidity interfaces, or other output. See
[generated output](generated-output.md) for filenames.

`package` sets the Go package name (a lowercase identifier that is not a keyword),
the Kotlin/Java package (dot-separated identifiers), or the PHP namespace
(backslash-separated identifiers). The default is `contracts`. abi-typegen rejects
values that are invalid for any selected target. Other targets ignore it.

## CLI overrides

The `generate` and `diff` commands accept artifact, output, target, and selection
overrides. Use the command's `--help` for its complete flag set.

```sh
abi-typegen generate \
  --artifacts ./out \
  --out ./types \
  --target viem,python \
  --contracts Token,Vault \
  --exclude "*Test,*Mock" \
  --clean
```

| Flag                   | Meaning                                                                    |
| ---------------------- | -------------------------------------------------------------------------- |
| `--artifacts <path>`   | Compiled artifact directory                                                |
| `--out <path>`         | Generated output directory                                                 |
| `--target <names>`     | One target or a comma-separated list                                       |
| `--contracts <names>`  | Contract allowlist, repeated or comma-separated                            |
| `--exclude <patterns>` | Comma-separated contract-name patterns; quote shell wildcards              |
| `--no-wrappers`        | Suppress wrappers (and Rust's `rpc` instance) while keeping primary output |
| `--package <name>`     | On `generate` and `diff`, the Go package or Kotlin/Java package            |
| `--clean`              | On `generate`, remove stale generated files                                |
| `--check`              | On `generate`, compare output without writing; exit nonzero if stale       |
| `--config <path>`      | Select a Foundry-style TOML configuration file                             |
| `--hardhat`            | Use the Hardhat artifact layout and defaults                               |

Generation validates the selected artifacts and renders the output before writing
or cleaning. Invalid artifacts and duplicate output names fail without modifying
existing generated files. Unchanged files are not rewritten.

Use separate commands for updating files and checking them:

```sh
abi-typegen generate --clean
abi-typegen generate --check
abi-typegen diff
```

`--check` does not write or clean output, even if `--clean` is also passed.
`diff` reports changes without writing. Keep generated output in a dedicated
directory when using cleanup.

## Hardhat plugin

Add the plugin to an existing Hardhat project. Hardhat 3 uses the plugin-object
form:

```typescript
import { defineConfig } from "hardhat/config";
import abiTypegen from "@0xdoublesharp/hardhat-abi-typegen";

export default defineConfig({
  plugins: [abiTypegen],
  typegen: {
    out: "src/generated",
    target: ["viem", "python"],
    wrappers: true,
    contracts: [],
    exclude: ["*Test", "*Mock"],
  },
});
```

Retain your project's compiler, network, and other settings alongside `typegen`.
For Hardhat 2, import the plugin for its registration side effect and add the same
`typegen` options to the exported configuration:

```typescript
import "@0xdoublesharp/hardhat-abi-typegen";
```

The plugin invokes generation after compilation and passes its configuration to
the CLI. A direct `abi-typegen --hardhat` invocation does **not** execute or read
`hardhat.config.ts`; it selects `artifacts/contracts` as the default artifact
path. Supply target and output flags explicitly when calling the CLI directly:

```sh
abi-typegen generate --hardhat --target viem --out src/generated
```

## Importing an ABI

`fetch` accepts an explorer address or a local JSON file. It saves a validated
artifact and then generates bindings using the configured targets and output.

```sh
abi-typegen fetch --name Token --network mainnet 0x0000000000000000000000000000000000000001
abi-typegen fetch --name Token --file ./Token.abi.json
```

The address above is a syntax example; replace it with a verified contract address.
Local input can be a raw ABI array or an artifact object containing an `abi` field.

| Flag                 | Meaning                                                  |
| -------------------- | -------------------------------------------------------- |
| `--name <name>`      | Required contract name for artifact and generated files  |
| `--file <path>`      | Local ABI input; mutually exclusive with an address      |
| `--network <name>`   | Built-in explorer shortcut; defaults to `mainnet`        |
| `--url <url>`        | Etherscan-compatible API endpoint; overrides `--network` |
| `--api-key <key>`    | Explorer API key; also read from `ETHERSCAN_API_KEY`     |
| `--artifacts <path>` | Where to save the artifact                               |
| `--force`            | Permit replacing an existing artifact                    |

Set target/output options in `[abi-typegen]` for `fetch`; it has no `--target` or
`--out` flags. You can select that TOML file with the global `--config` option:

```sh
abi-typegen --config ./foundry.toml fetch --name Token --file ./Token.abi.json
```

The CLI loads `.env` before parsing arguments. Keep API keys out of committed
configuration and shell examples:

```dotenv
ETHERSCAN_API_KEY=your_key_here
```

Network aliases and URLs are compiled into the CLI in [src/fetch.rs](../src/fetch.rs).
Examples include `mainnet`, `sepolia`, `base`, `arbitrum`, and `polygon`. They are
not discovered from a live chain list, and an alias does not guarantee explorer
availability. Use `--url` for a compatible endpoint not covered by your installed
version.
