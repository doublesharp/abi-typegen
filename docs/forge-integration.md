# Forge integration

Compile with Forge, then run abi-typegen against the resulting artifacts. The CLI
reads `[profile.default].out` and `[abi-typegen]` from `foundry.toml`; see
[configuration](configuration.md).

## Build and generate

```sh
forge build
abi-typegen generate
```

For another artifact directory or target:

```sh
abi-typegen generate --artifacts ./out --target viem --out ./src/generated
```

A Makefile can keep compilation and generation together:

```makefile
typegen:
	forge build
	abi-typegen generate --clean

check-generated:
	forge build
	abi-typegen generate --check
```

## CI

After installing Forge and abi-typegen and checking out the committed generated
files, run:

```yaml
- run: forge build
- run: abi-typegen generate --check
```

`--check` compares expected output without writing and exits nonzero when files
are missing or stale. Use `abi-typegen diff` locally to inspect changes, regenerate,
and commit the resulting output according to your project's policy.

## Optional shell integration

`abi-typegen forge-install` prints a shell function; it does not edit your shell
configuration. Inspect the output, then add it once to the appropriate startup
file. For example:

```sh
# zsh
abi-typegen forge-install --shell zsh >> ~/.zshrc
source ~/.zshrc

# bash
abi-typegen forge-install --shell bash >> ~/.bashrc
source ~/.bashrc
```

For fish:

```fish
abi-typegen forge-install --shell fish >> ~/.config/fish/config.fish
source ~/.config/fish/config.fish
```

The function forwards `forge typegen ...` to abi-typegen and sends other arguments
to the actual Forge executable:

```sh
forge typegen generate
forge typegen watch
forge typegen diff
```

This changes only shell dispatch. It does not automatically compile contracts
before generation. Remove the added function from the startup file to undo it.

## Watch mode

Watch Solidity compilation and ABI generation in separate terminals:

```sh
# Terminal 1
forge build --watch

# Terminal 2, after the artifact directory exists
abi-typegen watch
```

abi-typegen watches compiled artifacts with a 200 ms debounce. It does not compile
Solidity source. Set target/output preferences in `foundry.toml`; `watch` accepts
an artifact path but not `generate`'s target or output overrides.
