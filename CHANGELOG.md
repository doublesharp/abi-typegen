# Changelog

Notable changes, reconstructed from Git history. Release sections follow repository
tags; dates use the tagged commits' recorded local dates. Changes after the latest
tag appear under Unreleased.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.5.0] - 2026-09-23

This release reworks the Go, Rust, Swift, and Kotlin targets so their output
compiles, embeds the ABI and selectors, and names every tuple. Generated names
and layouts change for these four targets. The migration tables below map old
names to new ones. TypeScript, Python, C#, Solidity, and YAML output is unchanged.

### Added

- Rust, Swift, and Kotlin output embeds the JSON ABI (Go already did). All four
  embed the canonical signature and selector of every function, event, and
  error, and the topic of every non-anonymous event.
- `package` setting and `--package` flag (on `generate` and `diff`) for the Go
  package name and Kotlin package. The default stays `contracts`. Invalid values
  are rejected when the configuration is read.
- Rust output includes alloy's contract instance (`#[sol(rpc)]`), typed
  `…Call`/`…Return` structs, and event and error decoding. `--no-wrappers`
  removes only the contract instance.
- CI builds generated bindings in Go, Rust, Swift, and Kotlin consumer projects
  (`make e2e-go`, `e2e-rust`, `e2e-swift`, `e2e-kotlin`).

### Changed

- Rust: files are snake_case (`tuple_cases.rs`) with a generated `mod.rs`. Each
  file is an alloy `sol!` invocation, so types derive `PartialEq`, `Eq`, `Hash`,
  `Default` (where possible), and serde with ABI field names, events keep their
  indexed fields, and overloads follow alloy (`deposit_0Call`). NatSpec becomes
  rustdoc that passes `-D warnings`. Contracts with a fixed array longer than 32
  elements omit the serde derives, which serde cannot provide for such arrays.
- Go: output is gofmt-clean, imports only what it uses, and leaves a blank line
  between the generated-code header and the package clause. Integer widths other
  than 8, 16, 32, and 64 bits use `*big.Int`, which is what go-ethereum decodes.
  Overloads follow abigen (`Deposit`, `Deposit0`). Contract NatSpec documents the
  ABI constant instead of the package.
- Swift: every type, property, and initializer is `public`, and every struct is
  `Sendable` and `Hashable`. Types and constants nest in `public enum <Name>`.
  Imports `Web3Core` instead of `web3swift`.
- Kotlin: types and constants nest in `object <Name>` in the configured package.
  Tuples extend web3j's `StaticStruct`/`DynamicStruct`. Integers are
  `BigInteger`; `bytesN` and `bytes` use web3j's `BytesN` and `DynamicBytes`,
  which compare by value and keep their size. No `UInt`/`ULong`, so Java calls
  every getter by its plain name.
- All four targets keep acronyms (`TokenURI`, not `TokenUri`), name unnamed
  parameters after their types (`address`, `address2`) instead of `arg0`, and
  render tuples as named types from the struct's `internalType`.

### Fixed

- Rust rendered single-field tuples as parenthesized types (`(Address)`).
- Go reported unused `math/big` and `common` imports, and decoded `uint24`-style
  fields into native integers that go-ethereum rejects.
- Swift output needed `import Web3Core` and could not be used from another module.
- Kotlin tuples were `Map<String, Any>`, `ByteArray` fields compared by identity,
  and events and errors kept their ABI casing while parameter types did not.
- The fuzz targets built `Config` with a removed field.

### Migration

Go (`TupleCases`, `Vault`, and `Token` from the sample project):

| 0.4                                                  | 0.5                                   |
| ---------------------------------------------------- | ------------------------------------- |
| `package contracts` (fixed)                          | `package <package>`                   |
| Inline `struct { Account common.Address }`           | `TupleCasesTupleAccountPosition`      |
| `VaultDepositUint256AddressParams`                   | `VaultDepositParams`                  |
| `VaultDepositUint256Params`                          | `VaultDeposit0Params`                 |
| `uint24` field as `uint32`                           | `*big.Int`                            |
| `TokenTokenUriParams`                                | `TokenTokenURIParams`                 |
| Field `Arg0`                                         | Field named after its type (`Address`) |

Rust:

| 0.4                                          | 0.5                                          |
| -------------------------------------------- | -------------------------------------------- |
| `Token.rs`, no module file                   | `token.rs` plus `mod.rs` re-exporting `Token` |
| `TokenTransferParams`                        | `Token::transferCall`                        |
| `TokenTransferEvent`                         | `Token::Transfer`                            |
| `TokenInsufficientBalanceError`              | `Token::InsufficientBalance`                 |
| `TupleCasesDepositTupleAddressEndTupleParams` | `TupleCases::deposit_0Call`                  |
| Serde `deposited_at`                         | Serde `depositedAt`                          |

Swift:

| 0.4                                     | 0.5                                        |
| --------------------------------------- | ------------------------------------------ |
| `struct TokenTransferParams` (internal) | `public struct Token.TransferParams`       |
| `struct TokenTransferEvent`             | `Token.TransferEvent`                      |
| Tuple field `(account: EthereumAddress)` | `TupleCases.TupleAccountPosition`         |
| `VaultDepositUint256Params`             | `Vault.Deposit1Params`                     |

Kotlin:

| 0.4                                        | 0.5                                          |
| ------------------------------------------ | -------------------------------------------- |
| `package contracts` (fixed)                | `package <package>`                          |
| `data class TokenTransferParams`           | `Token.TransferParams`                       |
| `data class TokenTransferEvent`            | `Token.TransferEvent`                        |
| `TokenpausedEvent` (event `paused`)        | `Token.PausedEvent`                          |
| Tuple as `Map<String, Any>`                | `Vault.Position` (a web3j `StaticStruct`)    |
| `ByteArray` for `bytes32` / `bytes`        | web3j `Bytes32` / `DynamicBytes`             |
| `UInt` / `ULong` for small integers        | `BigInteger`                                 |

## [0.4.3] - 2026-09-23

### Fixed

- ethers v6: `<Name>Contract` combines the generated methods with
  `BaseContract`, so `getAddress()`, `interface`, `target`, `on()`, and
  `queryFilter()` are typed and `connect()` returns the generated type. Event
  filters return `DeferredTopicFilter`, which `queryFilter()` and `on()` accept.
- web3.js: `<Name>Contract` is web3's `Contract<typeof <Name>Abi>` with typed
  `methods`, so events, options, `send()`, `estimateGas()`, and `encodeABI()`
  keep web3's types. The factory no longer casts the ABI to `any`.
- web3.js: overloaded methods are typed under the keys web3 registers at
  runtime (`deposit` and `'deposit(uint256)'`). The previous `depositUint256`
  aliases did not exist on web3 contracts.
- web3.js: methods with several outputs are typed as web3's result object, keyed
  by position and name, instead of a tuple.

## [0.4.2] - 2026-09-23

### Fixed

- viem: emit NatSpec `@returns` tags in ABI output order so repeated runs
  produce identical files.
- viem: give `get<Name>Contract` an explicit, client-generic
  `GetContractReturnType`, fixing TS7056 declaration emit on large ABIs.
- wagmi: give write hooks an explicit return type built from
  `UseWriteContractReturnType`, fixing TS2883 declaration emit.
- wagmi: payable write hooks accept `{ value }` and forward it to
  `writeContract`.
- wagmi: allocate unique hook names when function names differ only in casing
  (`PREMIUM_PERIOD` and `premiumPeriod`) or overloads span read and write hooks.
- ethers v6 and v5: methods take a trailing `overrides` argument typed by state
  mutability, so payable calls can pass `value`.
- ethers v5: import `BigNumber`, `BigNumberish`, and `BytesLike` as types for
  `verbatimModuleSyntax` compatibility.
- web3.js: type integer return values as `bigint` and integer inputs as
  `Numbers`, matching web3 v4. Payable `send` accepts `value`. Import `Contract`
  from `web3` instead of `web3-eth-contract`.

## [0.4.1] - 2026-09-14

### Fixed

- Update rustls to 0.23.45 and rustls-webpki to 0.103.15 to resolve the
  TLS handshake advisory RUSTSEC-2026-0285.
- Make storage regression tests independent of recursive GNU Make directory logging.

## [0.4.0] - 2026-09-14

Tagged, but publication was stopped after CI found the issues fixed in 0.4.1.

### Added

- User and maintainer guides for installation, development, storage configuration,
  and release publication, with a documentation index and contribution guidelines.
- SHA-256 verification of downloaded binary archives against hashes bundled in the
  npm package. Release builds generate a versioned checksum manifest, and npm
  publication verifies every platform archive before embedding its hashes.
- Optional, per-checkout build and cache storage configured with
  `python3 .cargo/setup-scratch.py --root PATH`, with an optional required mount.
  Cargo, pnpm, and Make use saved local settings without repeated flags.
- `--disable` support to restore default storage paths while retaining external
  data and copying discovered fuzz corpus and coverage history into the checkout.
- Installer regression coverage for transient HTTP errors, HTTPS restrictions,
  archive integrity, extraction, and package contents. Separate CI jobs cover the
  installer on Linux, macOS, and Windows and storage tooling on Linux and macOS.

### Changed

- npm downloads use bounded retries with exponential backoff, connection and
  transfer timeouts, and an overall deadline. Downloads finish and pass checksum
  verification before extraction; the binary is replaced only after validation.
- Normal development keeps default build and cache paths without requiring Scratch,
  sccache, or Python. Python storage tests run separately through `make test-storage`.
- npm packages include their checksum manifest and exclude local binary output.
  Ordinary `npm pack` rejects missing or invalid checksum manifests.

### Fixed

- Transient GitHub download failures such as HTTP 504 no longer immediately fail
  installation. Failed downloads are cleaned up without misleading tar errors.
- An existing binary no longer bypasses installer verification solely because its
  path exists. Failed installation attempts preserve the existing binary.
- Storage setup rejects tracked paths, conflicting data, symlinked parent
  directories, and redirected configuration or cache paths before migration.

## [0.3.2] - 2026-09-09

### Fixed

- Package a stable npm command launcher so npm creates the `abi-typegen` command
  link before postinstall downloads the native binary.
- Forward CLI arguments, exit codes, and termination signals to the native process;
  terminating the launcher also terminates its child.

### Added

- npm packaging and launcher regression tests for command linking, CLI behavior,
  and child-process cleanup.

## [0.3.1] - 2026-09-09

Git history includes a `0.3.0` version bump on 2026-06-24 but no `v0.3.0` tag.
Those changes are included here, in the next tagged release after `0.2.0`.

### Added

- Hardhat 3 plugin support using configuration and Solidity compilation hooks,
  while retaining Hardhat 2 support.
- Generated-binding compilation and runtime tests covering ethers v5/v6 overloads,
  nested tuples and arrays, Solidity interfaces, and multi-target CLI behavior.
- Coverage reporting with downloadable artifacts and a GitHub Pages report.
- Dependency policy, Rust advisory, and release compatibility checks in CI.

### Changed

- Align ethers bindings with runtime input and output types, including
  `BigNumberish`, `BytesLike`, ethers v6 `AddressLike`, and named tuple results
  that preserve positional access.
- Use dynamic programming for exclusion-pattern matching to avoid exponential
  backtracking on adversarial patterns.
- Render and validate selected artifacts before writing or cleaning output; retain
  unchanged generated files instead of rewriting them.
- Publish npm packages with provenance.

### Fixed

- Discover all contract artifacts inside Foundry `.sol` directories, use contract
  names, and ignore Hardhat debug artifacts.
- Reject invalid artifacts, duplicate contract names, and output filename
  collisions before modifying generated files.
- Keep multi-target generation, checking, diffing, fetching, and watching consistent.
- Clean stale YAML output and stale generated files when the selected contract set
  is empty; omit TypeScript barrel files for non-TypeScript targets.
- Correct nested readonly array types, tuple overload suffixes, and Solidity
  struct-name collisions in generated bindings.
- Dispatch ethers overloads using canonical ABI signatures and preserve event
  filter argument positions.
- Resolve the Hardhat plugin's binary correctly from linked projects.

### Security

- Raise the `anyhow` dependency requirement and update the lockfile to `1.0.104`
  to address the unsoundness advisory identified by the dependency checks.

## [0.2.0] - 2026-04-04

### Added

- YAML renderer (`--target yaml`, alias `yml`) for human-readable ABI descriptions.
- Multi-target configuration in `foundry.toml` and Hardhat: accept comma-separated
  strings such as `"viem,python"` and arrays such as `["viem", "python"]`.

### Changed

- Rust source builds require Rust 1.85 and use edition 2024.

### Fixed

- Remove inline tuple comments from Python type annotations that produced invalid
  function signatures.
- Escape language-specific reserved parameter names in Python, Rust, Swift, and
  Kotlin, including names such as `from`, `type`, `self`, and `fun`.

## [0.1.0] - 2026-04-01

Initial release.

### Added

- Native Rust CLI for generating typed bindings from Foundry and Hardhat Solidity
  ABI artifacts.
- TypeScript, Python, Go, Rust, Swift, C#, Kotlin, and Solidity output targets.
- Multi-target generation and the `generate`, `watch`, `diff`, `json`, and `fetch`
  commands.
- `--check` for CI, `--clean` for stale generated files, and `--exclude` patterns.
- ABI fetching from Etherscan-compatible explorers.
- Named multi-return types, signature-based overload disambiguation, NatSpec
  propagation, and `as const` ABI exports for viem/wagmi inference.
- Hardhat plugin and an npm wrapper that downloads platform-specific binaries.

[Unreleased]: https://github.com/doublesharp/abi-typegen/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/doublesharp/abi-typegen/compare/v0.4.3...v0.5.0
[0.4.3]: https://github.com/doublesharp/abi-typegen/compare/v0.4.2...v0.4.3
[0.4.2]: https://github.com/doublesharp/abi-typegen/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/doublesharp/abi-typegen/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/doublesharp/abi-typegen/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/doublesharp/abi-typegen/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/doublesharp/abi-typegen/compare/v0.2.0...v0.3.1
[0.2.0]: https://github.com/doublesharp/abi-typegen/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/doublesharp/abi-typegen/releases/tag/v0.1.0
