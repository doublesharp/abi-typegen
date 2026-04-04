# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased] — targeting 0.2.0

### Added
- YAML renderer target (`--target yaml`, alias `yml`) for human-readable ABI descriptions
- Language-specific reserved word escaping for non-TypeScript renderers

### Fixed
- Python renderer no longer emits broken signatures when tuple types appear in parameter lists
- Reserved words in parameter names are now escaped for Python, Rust, Swift, and Kotlin

## [0.1.0] — 2025-04-01

Initial release.

### Highlights
- Native Rust CLI for generating typed bindings from Solidity ABI artifacts
- Foundry and Hardhat artifact support
- Multi-language target support (TypeScript, Python, Go, Rust, Swift, C#, Kotlin, Solidity)

### Features
- Multi-target generation (`--target viem,python,rust`)
- `generate`, `watch`, `diff`, `json`, and `fetch` commands
- `--check` mode for CI (exit non-zero if output is stale)
- `--clean` removes stale generated files not produced by the current run
- `--exclude` patterns to skip contracts
- `fetch` command to download verified ABIs from Etherscan-compatible explorers
- Named multi-returns and signature-based overload disambiguation
- NatSpec propagation to doc comments across all targets
- `as const` ABI export for viem/wagmi type inference
- Hardhat plugin (`@0xdoublesharp/hardhat-abi-typegen`)
- npm wrapper (`@0xdoublesharp/abi-typegen`) with platform-specific binary download
