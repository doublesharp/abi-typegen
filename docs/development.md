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
| `crates/abi-typegen-runtime/` | Shared Rust/Alloy codec and C ABI for C/C++ consumers                    |
| `crates/abi-typegen-codegen/` | Target renderers and type mapping                                        |
| `npm/abi-typegen/`            | npm launcher, binary installation, and checksum preparation              |
| `npm/hardhat-abi-typegen/`    | Hardhat 2 and Hardhat 3 plugin entry points                              |
| `tests/`, `npm/tests/`        | CLI, packaging, installer, and storage regression tests                  |
| `e2e/`                        | Foundry and Hardhat projects used to exercise generated bindings         |
| `e2e/native/`                 | Native language consumers that compile and exercise generated bindings   |
| `integrations/`               | Reusable Unity, Unreal, and Godot engine packages                        |
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
- Foundry integration: use Node 24, install Forge and the sample's pnpm dependencies, then run
  `make e2e-foundry`.
- Hardhat integration: install dependencies in the relevant `e2e/hardhat-sample`
  or `e2e/hardhat3-sample` directory, then run `make e2e-hardhat` or
  `make e2e-hardhat3`.
- Native-language integration: `make e2e-native` runs Go, Rust, Swift, Kotlin,
  Java, C#, Dart, PHP, Python, Ruby, shell, Elixir, COBOL, C, and C++ consumers. Individual `e2e-<target>` tasks
  generate bindings from `e2e/foundry-sample` into
  the consumers under `e2e/native/` and run each language's formatter, linter,
  and tests. They need Go, Rust with clippy and rustfmt, Swift 6, and Gradle with
  JDK 21. C/C++ additionally need a C11/C++17 compiler, C# needs .NET 10,
  Dart needs its SDK, PHP needs PHP 8.2+, Composer, and the curl, gmp, mbstring,
  and iconv extensions, and Python needs Python 3.11+ with venv support. The
  consumers do not build until a
  target has generated their bindings.
- Engine integrations are separate from `make e2e-native`: `make e2e-godot`
  requires Godot 4.7.2 and SCons 4.10.0; `make e2e-unity` requires the Unity
  6000.6.3f1 Editor and modules; `make e2e-unreal UNREAL_ROOT=<path>` requires
  Unreal Engine 5.8.3. Godot's Linux compatibility workflow runs on hosted CI.
  Unity and Unreal compatibility workflows require manually configured engine
  runners and are skipped by default. Unity requires repository variables
  `UNITY_ENGINE_CI_ENABLED=true` and `UNITY_EDITOR_6000_6_3F1`; Unreal requires
  `UNREAL_ENGINE_CI_ENABLED=true` and `UNREAL_ROOT_5_8_3`. A skipped workflow is
  not a qualification.
- Coverage: `make coverage` requires `cargo-llvm-cov` and Node/npm.
- Fuzzing: the `fuzz-*` Makefile targets require `cargo-fuzz` and nightly Rust.
  Curated seeds stay under `fuzz/seeds`; discovered corpus and logs are local data.

## Documentation and changes

Document current behavior and runnable workflows under `docs/`. Put release
history in [CHANGELOG.md](../CHANGELOG.md), with changes since the latest tag under
Unreleased. Keep execution plans, task checklists, and one-off test results out of
user guides. When changing behavior, add a regression test and update the relevant
guide. The [release guide](releasing.md) covers publication.

## Local RPC integration tests

Native consumer Make targets use [the Anvil harness](../e2e/native/anvil.py)
for signed submissions against a disposable chain. Install Foundry so `anvil`,
`cast`, and `forge` are available. No external RPC endpoint or secrets are needed.
The harness starts Anvil on an ephemeral local port, deploys the Token fixture,
sets `ATG_RPC_URL`, `ATG_TOKEN_ADDRESS`, `ATG_PRIVATE_KEY`, and `ATG_CHAIN_ID`
for the consumer, and stops the node afterward. The private key is a public
Anvil development key; never use it with real funds. For example:

```sh
python3 e2e/native/anvil.py --cwd e2e/native/go \
  go test ./usage -run TestGeneratedBindingsAnvil -count=1
```

Codec tests cover invalid inputs and ABI layout; Anvil tests cover signed
submissions, receipts, state changes, and event handling. The CI native matrix
runs the same Make targets.

The experimental COBOL consumer uses GnuCOBOL, libcurl, json-c, and pkg-config.
Run `make e2e-cobol` for offline codec checks and an asserted Anvil balance read.

Ruby consumers use Bundler and a system secp256k1 library. Shell consumers use
Bash and Foundry cast. Run `make e2e-ruby` or `make e2e-shell` to generate and test
those bindings, including local Anvil transactions.

Elixir consumers require Elixir and Erlang/OTP, plus Foundry for the independent
ABI comparison and Anvil tests. Run `make e2e-elixir` to compile every generated
contract, compile metadata-only modules without Ethers, and run offline and
signed RPC tests. Mix dependencies are pinned in the consumer lockfile.
