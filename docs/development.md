# Development

Run commands from the repository root unless a different directory is shown.
See [CONTRIBUTING.md](../CONTRIBUTING.md) for code conventions and pull requests.
Normal clones use Cargo's `target/` and package-manager defaults. A Scratch volume
or compiler cache is not required. To move disposable output, use the optional
[storage guide](development-storage.md).

## Source build

Use a Rust toolchain compatible with `rust-version` in [Cargo.toml](../Cargo.toml).
The project uses Rust edition 2024.

```sh
cargo build --workspace
cargo test --workspace
cargo run -- generate --help
```

For an optimized executable:

```sh
cargo build --release
./target/release/abi-typegen --version
```

On Windows, the executable has an `.exe` suffix. The native CLI does not require
Node or Python.

## Repository layout

| Path                          | Responsibility                                                           |
| ----------------------------- | ------------------------------------------------------------------------ |
| `src/`                        | CLI commands, artifact discovery, generation, watching, and ABI fetching |
| `crates/abi-typegen-core/`    | ABI parsing and intermediate types                                       |
| `crates/abi-typegen-config/`  | Configuration and target selection                                       |
| `crates/abi-typegen-codegen/` | Target renderers and type mapping                                        |
| `npm/abi-typegen/`            | npm launcher, binary installation, and checksum preparation              |
| `npm/hardhat-abi-typegen/`    | Hardhat 2 and Hardhat 3 plugin entry points                              |
| `tests/`, `npm/tests/`        | CLI, packaging, installer, and storage regression tests                  |
| `e2e/`                        | Foundry and Hardhat projects used to exercise generated bindings         |
| `fuzz/`                       | Fuzz targets, curated seeds, and local corpus/output                     |

## Rust checks

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
```

The Makefile also provides `build`, `test`, `check`, `fmt`, and `lint` targets.
`make test` runs Rust tests; it does not require optional storage tooling.

## npm development

Use Node and pnpm versions supported by the npm packages and the
[CI workflow](../.github/workflows/ci.yml). Install the repository's lint tooling:

```sh
cd npm
pnpm install --frozen-lockfile --ignore-scripts
pnpm run lint
pnpm run format:check
cd ..
```

Build the debug CLI before running packaging tests, which exercise that binary:

```sh
cargo build
node --test npm/tests/package.test.mjs npm/tests/installer.test.mjs npm/tests/checksums.test.mjs
```

Installer tests use local HTTP fixtures and real `curl`/`tar` processes. They need
permission to listen on a loopback port. The tests verify fixtures without
publishing a package or downloading production release assets.

To exercise the local CLI directly, use `cargo run -- ...` or the debug executable.
An npm source directory is not a release package: it has no pinned checksum
manifest until [release preparation](releasing.md) runs.

## Optional checks

- Storage tooling: `make test-storage` uses Python 3.11+ on macOS or Linux.
- Workflow syntax: `actionlint` checks `.github/workflows/`.
- Foundry integration: install Forge and the sample's pnpm dependencies, then run
  `make e2e-foundry`.
- Hardhat integration: install dependencies in the relevant `e2e/hardhat-sample`
  or `e2e/hardhat3-sample` directory, then run `make e2e-hardhat` or
  `make e2e-hardhat3`.
- Coverage: `make coverage` requires `cargo-llvm-cov` and Node/npm.
- Fuzzing: the `fuzz-*` Makefile targets require `cargo-fuzz` and nightly Rust.
  Curated seeds stay under `fuzz/seeds`; discovered corpus and logs are local data.

## Documentation and changes

Document current behavior and runnable workflows under `docs/`. Put release
history in [CHANGELOG.md](../CHANGELOG.md), with changes since the latest tag under
Unreleased. Keep execution plans, task checklists, and one-off test results out of
user guides. When changing behavior, add a regression test and update the relevant
guide. The [release guide](releasing.md) covers publication.
