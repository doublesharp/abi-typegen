# Documentation

These guides describe how to use and maintain abi-typegen. For version-specific
changes, see the [changelog](../CHANGELOG.md). Documentation on the main branch
can include behavior listed under Unreleased; use a release tag's documentation
when working with an older package.

## Using abi-typegen

- [Installation](installation.md): npm and native installation, supported
  platforms, checksum verification, and download troubleshooting.
- [Configuration](configuration.md): Foundry, Hardhat, CLI overrides, contract
  selection, and importing ABIs.
- [Generated output](generated-output.md): filenames, wrapper behavior, type
  mappings, and what each target provides.
- [Forge integration](forge-integration.md): build commands, CI checks, shell
  integration, and watching artifacts.
- [Choosing a binding tool](comparison.md): output tradeoffs and how to measure
  generation performance for your project.

## Developing and releasing

- [Contributing](../CONTRIBUTING.md): bug reports, code conventions, tests, and
  pull-request guidance.
- [Development](development.md): source builds, repository layout, and validation
  commands.
- [Build and cache storage](development-storage.md): optional local storage
  configuration, migration, and returning to defaults.
- [Releasing](releasing.md): matching package versions, binary assets, checksum
  manifests, and publication order.

- [Native contract bindings](native-bindings.md): runtime dependencies, C/C++ ownership,
  callable wrappers, engine integrations, and local Anvil tests.
