# Forge Integration

Foundry has no native plugin system. `abi-typegen` integrates via shell functions, build tools, and CI.

## Shell function

`abi-typegen forge-install` prints a shell function that intercepts `forge typegen` and dispatches to `abi-typegen`.

```sh
# bash / zsh
abi-typegen forge-install >> ~/.zshrc && source ~/.zshrc

# fish
abi-typegen forge-install --shell fish >> ~/.config/fish/config.fish
```

After setup:

```sh
forge typegen generate
forge typegen watch
forge typegen diff
```

## Makefile

```makefile
typegen:
	forge build && abi-typegen generate

check:
	abi-typegen generate --check
```

## CI

```yaml
- run: forge build
- run: abi-typegen generate --check
```

The `--check` flag exits non-zero if any generated files are out of date.

## Watch mode

```sh
# terminal 1
forge build --watch

# terminal 2
abi-typegen watch
```

Monitors the artifact directory with a 200ms debounce.
