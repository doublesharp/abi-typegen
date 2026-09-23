# Changelog

Notable changes, reconstructed from Git history. Release sections follow repository
tags; dates use the tagged commits' recorded local dates. Changes after the latest
tag appear under Unreleased.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

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

### Changed

- Rewrite the README in plainer language, opening each section with a sentence
  that explains it without assuming prior knowledge. Correct the overload naming example (`depositUint256`) and list wagmi
  v2 and v3 support.

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

[Unreleased]: https://github.com/doublesharp/abi-typegen/compare/v0.4.2...HEAD
[0.4.2]: https://github.com/doublesharp/abi-typegen/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/doublesharp/abi-typegen/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/doublesharp/abi-typegen/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/doublesharp/abi-typegen/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/doublesharp/abi-typegen/compare/v0.2.0...v0.3.1
[0.2.0]: https://github.com/doublesharp/abi-typegen/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/doublesharp/abi-typegen/releases/tag/v0.1.0
