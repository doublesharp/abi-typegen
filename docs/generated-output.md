# Generated Output

`abi-typegen` generates files based on the configured target.

## TypeScript targets

TypeScript wrapper targets (`viem`, `wagmi`, `ethers`, `ethers5`, `web3js`) emit:
- `<Name>.abi.ts` — the ABI as an `as const` constant (always)
- `<Name>.<target>.ts` — typed wrappers (when `wrappers = true`)
- `index.ts` — barrel re-exporting all contracts

The `zod` target emits:
- `<Name>.abi.ts` — the ABI as an `as const` constant
- `<Name>.zod.ts` — Zod 4 validation schemas using `import * as z from 'zod'`
- `index.ts` — barrel re-exporting ABI and schema modules

Install the latest `zod` package in the consuming project when using the `zod` target.

## Non-TypeScript targets

Non-TypeScript targets (`python`, `go`, `rust`, `swift`, `csharp`, `kotlin`, `solidity`, `yaml`) emit a single primary file per contract:
- `<Name>.py`, `<Name>.go`, `<Name>.rs`, `<Name>.swift`, `<Name>.cs`, `<Name>.kt`
- `I<Name>.sol` for the `solidity` target
- `<Name>.yaml` for the `yaml` target

The `solidity` target reconstructs interface-compatible structs from ABI tuples and emits Solidity `interface` files with events, custom errors, overloads, and `external` function signatures.

The `yaml` target emits a human-readable YAML description of each contract's functions, events, and errors with their parameter types.

## Overload naming

Overloaded functions use signature-based disambiguation:

```
deposit(uint256)           → depositUint
deposit(uint256, address)  → depositUintAddress
```

## Named multi-returns

View functions with named outputs return typed objects instead of tuples:

```typescript
// Named outputs → object
getPosition(user: string): Promise<{ shares: bigint; depositedAt: bigint; token: string }>

// Unnamed outputs → tuple
getValues(): Promise<[bigint, bigint]>
```

## NatSpec

`@notice`, `@param`, and `@return` tags from Solidity NatSpec are emitted as documentation comments in all targets (JSDoc, docstrings, doc comments, KDoc, XML docs, and Solidity NatSpec comments).

## Reserved word escaping

Parameter names that are reserved words in the target language are automatically escaped with an underscore prefix:

| Language | Example reserved names | Escaped as |
|----------|----------------------|------------|
| Python | `from`, `lambda`, `yield` | `_from`, `_lambda`, `_yield` |
| Rust | `type`, `fn`, `self`, `match` | `_type`, `_fn`, `_self`, `_match` |
| Swift | `self`, `is`, `func`, `let` | `_self`, `_is`, `_func`, `_let` |
| Kotlin | `fun`, `val`, `when`, `object` | `_fun`, `_val`, `_when`, `_object` |

Go and C# are not affected because field names use PascalCase, which avoids collisions with lowercase keywords. Solidity parameter names originate from Solidity ABIs and are inherently valid.

## Imports

All TypeScript imports use `.js` extensions for ESM compatibility.

## Type mappings

| Solidity | Viem | Ethers v6 | Ethers v5 | Python | Go | Rust |
|----------|------|-----------|-----------|--------|----|------|
| `uint8`–`uint48` | `number` | `number` | `number` | `int` | `uint8`–`uint64` | `u8`–`u64` |
| `uint56`–`uint256` | `bigint` | `bigint` | `BigNumber` | `int` | `*big.Int` | `U256` |
| `bool` | `boolean` | `boolean` | `boolean` | `bool` | `bool` | `bool` |
| `address` | `` `0x${string}` `` | `string` | `string` | `ChecksumAddress` | `common.Address` | `Address` |
| `bytes` | `` `0x${string}` `` | `string` | `string` | `bytes` | `[]byte` | `Bytes` |
| `string` | `string` | `string` | `string` | `str` | `string` | `String` |
| `T[]` | `readonly T[]` | `T[]` | `T[]` | `list[T]` | `[]T` | `Vec<T>` |
