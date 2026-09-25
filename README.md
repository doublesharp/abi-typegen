# abi-typegen

<p align="center">
  <img src="docs/abi-typegen.png" alt="abi-typegen" width="360" />
</p>

<p align="center"><strong>One ABI. Your language.</strong></p>
<p align="center">Compile your contracts. Generate typed code for your app.</p>
<p align="center">
  <a href="https://github.com/doublesharp/abi-typegen/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/doublesharp/abi-typegen/ci.yml?branch=main&label=ci"></a>
  <a href="https://github.com/doublesharp/abi-typegen/actions/workflows/coverage.yml"><img alt="Coverage workflow" src="https://img.shields.io/github/actions/workflow/status/doublesharp/abi-typegen/coverage.yml?branch=main&label=coverage"></a>
  <a href="https://doublesharp.github.io/abi-typegen/coverage/"><img alt="Coverage report" src="https://img.shields.io/badge/coverage-report-2ea043"></a>
  <a href="https://crates.io/crates/abi-typegen"><img alt="crates.io" src="https://img.shields.io/crates/v/abi-typegen"></a>
  <a href="https://www.npmjs.com/package/@0xdoublesharp/abi-typegen"><img alt="npm" src="https://img.shields.io/npm/v/@0xdoublesharp/abi-typegen?label=npm"></a>
</p>

A Solidity ABI lists the functions your app can call and the events a contract can
emit. `abi-typegen` turns that list into code for your language, with named
arguments, return types, and helpers for your Ethereum SDK.

It reads Foundry and Hardhat artifacts, runs as one native binary, and can generate
several targets at once. Your editor can then point out a wrong argument before
you send a transaction.

<p align="center">
  <a href="#quick-start">Quick start</a> ·
  <a href="#choose-your-target">Targets</a> ·
  <a href="#use-the-generated-code">Examples</a> ·
  <a href="#keep-bindings-in-sync">CI</a> ·
  <a href="docs/README.md">Docs</a>
</p>

## Install

Install the generator once. Add your chosen Ethereum SDK to the app that uses its
output.

```sh
cargo install abi-typegen
```

You can also download a [prebuilt binary](https://github.com/doublesharp/abi-typegen/releases)
or add the CLI to a JavaScript project:

```sh
pnpm add -D @0xdoublesharp/abi-typegen
pnpm exec abi-typegen --help
```

The npm package installs the native executable. Generating code does not require
an RPC connection, and the standalone binary needs neither Node nor Python.

See [installation](docs/installation.md) for platforms and download verification.

## Quick start

Compile your contracts, then point the generator at the files your compiler
created. The default target is viem and the default output is `src/generated`.

### Foundry

Use your existing Foundry project:

```sh
forge build
abi-typegen generate
```

Add your preferences to `foundry.toml`:

```toml
[abi-typegen]
out = "src/generated"
target = "viem"
```

Keep a watcher running while you work:

```sh
abi-typegen watch
```

It regenerates when artifacts change, so run `forge build` after editing Solidity.
For a `forge typegen` shell command, see [Forge integration](docs/forge-integration.md).

### Hardhat

The plugin generates bindings when Hardhat compiles your contracts.

```sh
pnpm add -D @0xdoublesharp/hardhat-abi-typegen
```

For Hardhat 3, add it to `hardhat.config.ts`:

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

```sh
pnpm exec hardhat compile
```

Hardhat 2 uses a side-effect import instead of `plugins`:

```ts
import "@0xdoublesharp/hardhat-abi-typegen/hardhat2";
```

You can also use the CLI directly with `abi-typegen generate --hardhat`.

## Choose your target

Pick the library your app already uses. A target determines the generated API and
its runtime dependency; it does not install that dependency for you.

```sh
abi-typegen generate --target go
```

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

| Language | Target   | Runtime or SDK                                                                                         | What you get                                                      |
| -------- | -------- | ------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------- |
| C        | `c`      | [Shared Rust runtime](docs/native-bindings.md#c-and-c)                                                 | C11 codecs and client helpers with explicit ownership             |
| C++      | `cpp`    | [Shared Rust runtime](docs/native-bindings.md#c-and-c)                                                 | C++17 codecs and client helpers with automatic cleanup            |
| C#       | `csharp` | [Nethereum](https://docs.nethereum.com/)                                                               | Typed DTOs, contract methods, and deployment                      |
| Dart     | `dart`   | [web3dart](https://pub.dev/packages/web3dart)                                                          | Typed values, codecs, calls, and transactions                     |
| Elixir   | `elixir` | [Ethers](https://ethers.hexdocs.pm/Ethers.html)                                                        | Transaction data, reads, sends, events, and errors                |
| Go       | `go`     | [go-ethereum](https://geth.ethereum.org/docs/developers)                                               | Typed calls, transactions, deployment, and events                 |
| Java     | `java`   | [web3j](https://docs.web3j.io/latest/)                                                                 | Typed values, codecs, calls, and transactions                     |
| Kotlin   | `kotlin` | [web3j](https://docs.web3j.io/latest/)                                                                 | Typed values, codecs, calls, and transactions                     |
| PHP      | `php`    | [Brick Math](https://github.com/brick/math), [ethereum-tx](https://github.com/web3p/ethereum-tx), cURL | Codecs, RPC reads, signed legacy transactions, receipts, and logs |
| Python   | `python` | [web3.py](https://web3py.readthedocs.io/en/stable/)                                                    | Reads, transaction builders, codecs, events, and errors           |
| Ruby     | `ruby`   | [eth](https://github.com/q9f/eth.rb)                                                                   | Calls, transaction builders, codecs, events, and errors           |
| Rust     | `rust`   | [Alloy](https://alloy.rs/introduction/getting-started/)                                                | `sol!` types, codecs, and contract instances                      |
| Swift    | `swift`  | [web3swift](https://github.com/web3swift-team/web3swift)                                               | Typed values, codecs, and contract operations                     |

### Game engines

| Engine                                                                        | Target                                                                            | What you get                                                             |
| ----------------------------------------------------------------------------- | --------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| [Unity](https://docs.unity3d.com/Manual/index.html)                           | `csharp` + [Unity adapter](integrations/unity/com.doublesharp.abi-typegen.unity/) | C# bindings with Unity HTTP transport and object lifecycle support       |
| [Godot](https://docs.godotengine.org/en/stable/)                              | `godot`                                                                           | GDScript bindings and a native extension for codecs and asynchronous RPC |
| [Unreal Engine](https://dev.epicgames.com/documentation/en-us/unreal-engine/) | `unreal`                                                                          | C++ codecs and Blueprint nodes for asynchronous scalar reads             |

See [engine packages](integrations/README.md) for setup and
[native bindings](docs/native-bindings.md) for supported features and platforms.

### Shell and ABI formats

| Target     | Works with                                                                                                    | What you get                                                        |
| ---------- | ------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| `shell`    | [Bash](https://www.gnu.org/software/bash/manual/bash.html) and [Foundry cast](https://www.getfoundry.sh/cast) | Sourceable helpers for encoding, reads, sends, deployment, and logs |
| `solidity` | [Solidity](https://docs.soliditylang.org/en/latest/)                                                          | Interfaces, tuple structs, events, and errors                       |
| `yaml`     | [YAML](https://yaml.org/)                                                                                     | Readable ABI descriptions                                           |

Zod, Solidity, and YAML describe or validate contracts; they do not submit
transactions. See [native bindings](docs/native-bindings.md) for SDK versions
and [configuration](docs/configuration.md) for target aliases.

### COBOL

| Target  | Works with                                                                                | What you get                                          |
| ------- | ----------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| `cobol` | [GnuCOBOL](https://gnucobol.sourceforge.io/doc/gnucobol.html) and the shared Rust runtime | Reads taking one address and returning one uint256 🤷 |

### Generate for more than one app

Use the same contract artifacts for your frontend, backend, or native app:

```sh
abi-typegen generate --target viem,python,rust
```

Each target gets its own directory:

```text
src/generated/
├── viem/
├── python/
└── rust/
```

You can save the same selection with `target = ["viem", "python", "rust"]`.

## Use the generated code

Import the generated file, connect your SDK, and call the contract through its
typed API. You still choose the RPC endpoint, wallet, and transaction settings.

### Read with viem

For a generated `Token` binding, set `RPC_URL`, `TOKEN_ADDRESS`, and
`ACCOUNT_ADDRESS` in your environment:

```ts
import { createPublicClient, getAddress, http } from "viem";
import { getTokenContract } from "./generated/Token.viem.js";

const client = createPublicClient({ transport: http(process.env.RPC_URL) });
const token = getTokenContract(getAddress(process.env.TOKEN_ADDRESS!), client);
const owner = getAddress(process.env.ACCOUNT_ADDRESS!);

const balance = await token.read.balanceOf([owner]);
console.log(balance); // bigint
```

Generated TypeScript imports use `.js` extensions for ESM. Tuple fields and
function arguments retain their ABI types.

### Read or encode with Python

The Python wrapper uses your web3.py connection. Encoding a call works offline:

```python
import os
from web3 import Web3
from generated.Token import TokenContract

w3 = Web3(Web3.HTTPProvider(os.environ["RPC_URL"]))
token = TokenContract(Web3.to_checksum_address(os.environ["TOKEN_ADDRESS"]), w3)
owner = Web3.to_checksum_address(os.environ["ACCOUNT_ADDRESS"])

balance = token.balance_of(owner)
calldata = token.encode_transfer(owner, 10**18)
```

Write helpers build transactions for your signing flow. Other targets expose
their SDK's transaction or operation types. A submitted transaction hash is not
proof of success; check the mined receipt.

### Call or sign with PHP

The PHP target generates ABI codecs and a small JSON-RPC client. Install its
runtime dependencies, generate the bindings, then pass the RPC URL and signing
settings to the generated client:

```sh
composer require brick/math:^1.0 web3p/ethereum-tx:^0.4.3
abi-typegen generate --target php --out ./generated --package 'App\Contracts'
```

```php
<?php
require __DIR__ . '/vendor/autoload.php';
foreach (glob(__DIR__ . '/generated/*.php') ?: [] as $file) require_once $file;

use App\Contracts\Token;
use App\Contracts\TokenClient;
use App\Contracts\TokenTransactionOptions;
use Brick\Math\BigInteger;

$client = new TokenClient(
    getenv('TOKEN_ADDRESS'),
    getenv('RPC_URL'),
    getenv('PRIVATE_KEY'),
    getenv('FROM_ADDRESS'),
    (int) getenv('CHAIN_ID'),
);
$options = new TokenTransactionOptions($client->gasPrice(), BigInteger::of(500000));
$hash = Token::sendApprove($client, getenv('SPENDER'), BigInteger::of('1000'), $options);
$receipt = $client->getTransactionReceipt($hash);
```

PHP 8.2 or newer needs the `curl`, `gmp`, `mbstring`, and `iconv` extensions. The
client signs legacy EIP-155 transactions locally. It does not build EIP-1559
transactions or deploy contracts. Poll for a receipt before treating a returned
transaction hash as success. Event log filters, event decoders, and declared
custom-error decoders are also generated. The upstream signing library emits
ArrayAccess return-type deprecation notices on PHP 8.5.

### Read a balance from COBOL

The COBOL target supports read-only functions with one `address`
input and one `uint256` output, such as `balanceOf(address)`:

```sh
abi-typegen generate --target cobol --out ./generated
```

It emits free-form GnuCOBOL subprograms and a C bridge to the shared Rust runtime.
Results use decimal text in a 78-character buffer. Other functions, events, and
errors appear as metadata comments. See the [COBOL build guide](docs/native-bindings.md#cobol)
for dependencies and limitations.

### Link C or C++

The generated headers describe your contract. A shared Rust library handles ABI
encoding and decoding, while your application supplies networking and signing.

```sh
abi-typegen generate --target cpp
cargo build --release -p abi-typegen-runtime
```

Build the runtime from this repository and link `abi_typegen_runtime` into your
consumer. C emits `atg_<Name>.h`; C++ also emits `atg_<Name>.hpp`. Both include
`abi_typegen.h`. The [native guide](docs/native-bindings.md#c-and-c) explains
result ownership, transport callbacks, and deployment bytecode.

## Know what the bindings cover

Typed arguments help catch mistakes before a call reaches the chain. They cannot
prove that a transaction will succeed or recover data the chain did not include.

Generated wrappers cover contract calls, ABI values, and the SDK integrations
listed above. Convenience methods for deployment, receipt handling, event queries,
and subscriptions differ by target.

- Java and Kotlin currently cannot encode fixed-array inputs outside lengths 1–32,
  including arrays nested inside tuples. These inputs fail explicitly.
- Indexed strings, bytes, arrays, and tuples appear in logs as hashes. Their
  original values cannot be decoded from those topics.
- Anonymous events have no signature topic, so the caller must choose the decoder.
- C/C++ consumers build and link the runtime separately.

See [current boundaries](docs/native-bindings.md#current-boundaries) for the full
list. Regenerate when upgrading, and check the [changelog](CHANGELOG.md) for API
or naming changes.

## Configure the output

Save your usual settings once. Override them on the command line when you need a
different output directory or contract selection.

```toml
# foundry.toml
[abi-typegen]
out = "src/generated"
target = ["viem", "go"]
wrappers = true
contracts = ["Token", "Vault"]
exclude = ["*Test", "*Mock"]
package = "contracts"
```

An empty `contracts` list selects all contracts. `package` names the Go package or
the Kotlin/Java package. The [configuration guide](docs/configuration.md) covers
Hardhat settings, glob patterns, aliases, and precedence.

```sh
abi-typegen generate \
  --artifacts ./out \
  --out ./types \
  --target java \
  --package com.example.contracts \
  --contracts Token,Vault \
  --clean
```

`--clean` removes stale generated files. `--no-wrappers` keeps primary ABI metadata
and types while omitting callable wrappers where supported. It keeps Zod schemas
and Solidity interfaces. See [generated output](docs/generated-output.md) for
filenames and target-specific behavior.

## Start with an existing contract

You do not need the original Solidity project. Import a local ABI, or fetch one
from an explorer that has verified the contract.

```sh
# Bare ABI arrays and artifact files containing an "abi" field both work.
abi-typegen fetch --name Token --file ./Token.abi.json
```

For an Etherscan-compatible explorer, set `ETHERSCAN_API_KEY` in your environment
or a local `.env` file, then fetch and generate:

```sh
abi-typegen fetch --name WETH --network mainnet \
  0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2
```

The default output includes a saved `out/WETH.sol/WETH.json` artifact and generated
viem files. Network shortcuts include `mainnet`, `sepolia`, `base`, `optimism`, and
`arbitrum`. For another compatible explorer, supply its endpoint:

```sh
abi-typegen fetch --name Token \
  --url "$EXPLORER_API_URL" "$CONTRACT_ADDRESS"
```

## Keep bindings in sync

When a contract changes, its generated code should change with it. A CI check can
catch forgotten updates before they are merged.

For Foundry:

```yaml
- run: forge build
- run: abi-typegen generate --check
```

For Hardhat, the plugin regenerates during compilation:

```yaml
- run: pnpm exec hardhat compile
- run: git diff --exit-code src/generated/
```

Use `--check` when CI should verify files without rewriting them. Use `diff` when
you want to inspect the proposed changes:

```sh
abi-typegen diff
```

## Command reference

| Command         | Example                                                | What it does                                         |
| --------------- | ------------------------------------------------------ | ---------------------------------------------------- |
| `generate`      | `abi-typegen generate --target viem`                   | Write bindings from compiled artifacts               |
| `watch`         | `abi-typegen watch --artifacts out`                    | Regenerate when artifacts change                     |
| `diff`          | `abi-typegen diff --target viem`                       | Preview changes without writing files                |
| `json`          | `abi-typegen json --pretty`                            | Print the parsed ABI as JSON                         |
| `fetch`         | `abi-typegen fetch --name Token --file Token.abi.json` | Import an ABI and generate bindings                  |
| `init`          | `abi-typegen init`                                     | Add an `[abi-typegen]` section to `foundry.toml`     |
| `forge-install` | `abi-typegen forge-install --shell zsh`                | Print shell integration that enables `forge typegen` |
| `help`          | `abi-typegen help generate`                            | Show help for the CLI or a command                   |

Use `abi-typegen --version` to print the installed version and
`abi-typegen help <command>` for command help. Global `--config PATH` selects a
configuration file; `--hardhat` selects the Hardhat artifact layout.

Common generation options:

| Option                    | What it does                                           |
| ------------------------- | ------------------------------------------------------ |
| `--artifacts PATH`        | Read compiled artifacts from a directory               |
| `--out PATH`              | Choose the output directory                            |
| `--target viem,python`    | Generate one or more targets                           |
| `--contracts Token,Vault` | Select contracts by name                               |
| `--exclude '*Test,*Mock'` | Exclude contracts by glob pattern                      |
| `--package NAME`          | Set the package, namespace, or Unreal module name      |
| `--no-wrappers`           | Keep metadata and types without contract wrappers      |
| `--check`                 | Fail if generated output is stale, without writing     |
| `--clean`                 | Remove stale generated files from the output directory |

`watch` reads target and output settings from your configuration.
`forge-install` supports Bash, Zsh, and Fish; follow the
[Forge setup guide](docs/forge-integration.md) to load the printed integration.
See [configuration](docs/configuration.md) for complete examples and
`abi-typegen help <command>` for command-specific options.

## Docs and support

Start with the guide for the part you are working on. For a bug report, include a
small ABI that reproduces the problem, your target, and the relevant SDK version.

- [Installation](docs/installation.md) and [configuration](docs/configuration.md)
- [Generated output](docs/generated-output.md) and [native bindings](docs/native-bindings.md)
- [Changelog](CHANGELOG.md)
- [Report an issue](https://github.com/doublesharp/abi-typegen/issues)

The [documentation index](docs/README.md) links the full set of guides.

## Contribute

See [CONTRIBUTING.md](CONTRIBUTING.md) for source builds, tests, code conventions,
and pull requests.
