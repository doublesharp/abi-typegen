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
    out: "src/generated", // output directory (default: "src/generated")
    target: "viem", // target name or comma-separated targets
    wrappers: true, // emit typed wrappers (default: true)
    contracts: ["Token"], // optional - limit to named contracts
    exclude: ["*Test", "*Mock"], // optional - exclude by glob pattern
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
    out: "src/generated", // output directory (default: "src/generated")
    target: "viem", // target name or comma-separated targets
    wrappers: true, // emit typed wrappers (default: true)
    contracts: ["Token"], // optional - limit to named contracts
    exclude: ["*Test", "*Mock"], // optional - exclude by glob pattern
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

| Option      | Type                 | Default           | Description                                                           |
| ----------- | -------------------- | ----------------- | --------------------------------------------------------------------- |
| `out`       | `string`             | `"src/generated"` | Output directory for generated files                                  |
| `target`    | `string \| string[]` | `"viem"`          | Target name, comma-separated targets, or array of targets (see below) |
| `wrappers`  | `boolean`            | `true`            | Emit typed wrapper files when supported                               |
| `contracts` | `string[]`           | `[]`              | Limit to named contracts (empty = all)                                |
| `exclude`   | `string[]`           | `[]`              | Exclude contracts matching glob patterns                              |

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

Multi-target examples — each target gets its own output subdirectory:

```typescript
target: "viem,python,rust"; // comma-separated string
target: ["viem", "python", "rust"]; // array of strings
```

## Notes

- When using `target: "zod"`, install the latest `zod` package in the consuming project
- Use comma-separated multi-target generation instead of the removed `all` and `all-ts` aliases

See [github.com/doublesharp/abi-typegen](https://github.com/doublesharp/abi-typegen) for full documentation.

## Native contract bindings

Go, Swift, Kotlin, C#, Java, and Dart wrappers use their runtime SDKs for contract
calls and transactions. PHP includes ABI codecs, JSON-RPC reads, legacy transaction
signing, receipt and log queries, and event/error decoding. It requires PHP 8.2+ with
cURL, GMP, mbstring, and iconv. It does not generate EIP-1559 or deployment helpers. C/C++ bindings link `abi-typegen-runtime` and accept a
caller-supplied transport/signing adapter. `--no-wrappers` preserves primary ABI
metadata and value types. See the [native binding guide](https://github.com/doublesharp/abi-typegen/blob/main/docs/native-bindings.md)
for dependencies, ownership rules, supported features, and SDK limits.
