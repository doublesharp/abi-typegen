# Contributing

Bug reports, documentation improvements, and code changes are welcome. Start with
the [documentation index](docs/README.md) for usage or the
[development guide](docs/development.md) for source builds and repository layout.

## Reporting a problem

Include the abi-typegen version, operating system and architecture, installation
method, command, and relevant configuration. Describe what you expected and what
happened. For generation issues, provide the smallest ABI or artifact that
reproduces the problem, the generated output, and the consuming SDK/compiler
versions. Remove API keys and other credentials before sharing logs or fixtures.

For installation failures, include the download error and platform target. Keep
checksum verification enabled while investigating; a hash mismatch needs an
explanation, not a bypass.

## Local setup

Use a Rust toolchain compatible with `rust-version` in [Cargo.toml](Cargo.toml),
then run:

```sh
cargo build --workspace
cargo test --workspace
```

Rust development needs no Scratch volume, sccache, Node, or Python. npm and
Hardhat changes additionally need the JavaScript toolchain described in the
[development guide](docs/development.md#npm-development). Optional build/cache
redirection is documented in the [storage guide](docs/development-storage.md).
Machine-specific settings and caches must remain untracked.

Create a focused branch from the intended base, for example:

```sh
git switch -c feature/describe-your-change
```

Keep unrelated edits separate. Avoid generated output, dependency updates, or
formatting changes that are not needed for the proposed change. Include generated
fixtures when they are part of the relevant test or documented example.

## Code conventions

- Do not use `unwrap()` in library code. Propagate errors with `?`, or use
  `expect("reason")` where an invariant justifies it.
- Explain each use of `unsafe` with a `// SAFETY:` comment.
- Use `alloy-primitives` types such as `Address`, `B256`, and `U256` for their
  corresponding values rather than raw byte-array substitutes.
- Define library errors with `thiserror`. Document public items so `cargo doc`
  builds cleanly.
- Use `tracing` macros in library crates. Reserve `println!` for user-facing
  binary output; do not add library `println!` or `eprintln!` calls.
- Validate ABI inputs at parse boundaries, including integer widths and byte
  sizes. Do not defer invalid-input errors to rendering.
- Inspect structured types directly. Do not infer types by searching rendered
  strings. Prefer exhaustive enum matches when dispatching on targets or types.
- Keep flags faithful to their names. For example, disabling wrappers must retain
  primary output such as ABI modules, schemas, and non-wrapper bindings.

## Tests and validation

Add a regression test when fixing a bug. Favor observable behavior: a minimal ABI
that produces the right output, generated code that compiles, a CLI failure that
preserves existing files, or an installer that rejects a substituted archive.
Avoid tests that only repeat implementation details.

Run the checks appropriate to the changed code and resolve failures before
submitting. Do not dismiss a failing check solely because the failure predates
your changes; investigate and explain the scope of any additional fix.

For Rust changes:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
```

For npm scripts or plugins, run the lint, formatting, and Node tests in the
[development guide](docs/development.md#npm-development). Renderer changes should
also exercise relevant generated-code compilation or runtime checks. Storage
changes use `make test-storage`; workflow changes use `actionlint`.

Documentation changes should have valid links and commands that match the CLI.
Run safe local examples against a temporary project rather than changing a user's
contracts, shell configuration, or published packages.

## Documentation and changelog

Keep `docs/` focused on current usage, behavior, limitations, and maintainable
workflows. Do not use it for execution plans, completed-task checklists, or
one-off test reports. Link to existing guides rather than duplicating instructions.

Add notable user-facing changes under Unreleased in [CHANGELOG.md](CHANGELOG.md).
Released sections follow repository tags. Do not invent a release or publication
date from an untagged version bump.

## Pull requests

Explain the concrete problem, the resulting behavior, and how you validated the
change. Include a reproduction or before/after example when useful. Identify
compatibility changes and any validation you could not run. Keep the title and
description about the final patch rather than the sequence of attempts.

Version bumps, tagging, and publication follow the [release guide](docs/releasing.md).
Ordinary fixes do not need to publish a package or modify existing release assets.
