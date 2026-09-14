# Generated output

Output depends on the selected target. A single target writes directly into the
configured output directory; multiple targets write into separate target-named
subdirectories. See [configuration](configuration.md) for selection and overrides.

## TypeScript files

Every TypeScript target emits `<Name>.abi.ts`, which exports an `as const` ABI.
For a nonempty selection, `index.ts` re-exports the generated modules.

| Target               | Additional file     | Effect of `wrappers = false` |
| -------------------- | ------------------- | ---------------------------- |
| `viem`               | `<Name>.viem.ts`    | Omit the wrapper module      |
| `wagmi`              | `<Name>.wagmi.ts`   | Omit the wrapper module      |
| `ethers` / `ethers6` | `<Name>.ethers.ts`  | Omit the wrapper module      |
| `ethers5`            | `<Name>.ethers5.ts` | Omit the wrapper module      |
| `web3js` / `web3`    | `<Name>.web3.ts`    | Omit the wrapper module      |
| `zod`                | `<Name>.zod.ts`     | Keep the schema module       |

Install the SDK used by your selected wrapper target in the consuming project.
Zod output uses Zod 4 APIs and imports from `zod`; use a compatible Zod 4 version.
Generation does not install SDK dependencies for you.

Generated relative TypeScript imports use `.js` extensions for ESM compatibility.
SDK imports use their package names. Ensure your TypeScript/build configuration
supports those import conventions.

## Other targets

Each selected contract produces one primary file. These targets do not emit
TypeScript ABI modules or an `index.ts` barrel.

| Target     | File           | Output purpose                                                |
| ---------- | -------------- | ------------------------------------------------------------- |
| `python`   | `<Name>.py`    | Python type declarations compatible with web3.py usage        |
| `go`       | `<Name>.go`    | ABI constant and typed structs for Go integration             |
| `rust`     | `<Name>.rs`    | Data types using Rust and Alloy primitive types               |
| `swift`    | `<Name>.swift` | Swift contract-related types                                  |
| `csharp`   | `<Name>.cs`    | C# contract-related types                                     |
| `kotlin`   | `<Name>.kt`    | Kotlin contract-related types                                 |
| `solidity` | `I<Name>.sol`  | Solidity interface reconstructed from ABI data                |
| `yaml`     | `<Name>.yaml`  | Human-readable functions, events, errors, and parameter types |

The Solidity target reconstructs tuple structs and emits events, errors,
overloads, and external function signatures. It cannot recover a contract's
implementation from an ABI.

Output APIs vary by language. In particular, Rust data types and Go structs are
not a complete deployed-contract client or deployment factory. Inspect the output
before integrating it with your runtime SDK.

## Overloaded functions

Generated wrapper/type names distinguish overloaded signatures. For example:

```text
deposit(uint256)          -> depositUint256
deposit(uint256,address)  -> depositUint256Address
```

Ethers wrappers use canonical ABI signatures to select the runtime method. Tuple
and array inputs participate in signature naming. When aliases would collide with
another alias or an existing method name, the ethers renderer adds a numeric
suffix. Treat generated names as part of the output API and recompile consumers
when the ABI changes.

## Named return values

Ethers named tuple results preserve both numeric positions and named properties.
Representative result types are:

```typescript
// ethers v6
type Position = [bigint, bigint, string] & {
  shares: bigint;
  depositedAt: bigint;
  token: string;
};

// ethers v5, with BigNumber imported from ethers
type PositionV5 = [BigNumber, BigNumber, string] & {
  shares: BigNumber;
  depositedAt: BigNumber;
  token: string;
};
```

Unnamed multi-return values use tuples. Input and output types can differ: ethers
integer inputs use `BigNumberish`, bytes inputs use `BytesLike`, and ethers v6
address inputs use `AddressLike`.

## Integer output mappings

Unsigned examples below show the size boundaries. Go and Rust use the smallest
supported native integer type that can hold the ABI width, then switch to a large
integer type.

| Solidity  | Viem     | Ethers v6 | Ethers v5   | Go         | Rust   |
| --------- | -------- | --------- | ----------- | ---------- | ------ |
| `uint8`   | `number` | `bigint`  | `number`    | `uint8`    | `u8`   |
| `uint24`  | `number` | `bigint`  | `number`    | `uint32`   | `u32`  |
| `uint48`  | `number` | `bigint`  | `number`    | `uint64`   | `u64`  |
| `uint56`  | `bigint` | `bigint`  | `BigNumber` | `uint64`   | `u64`  |
| `uint64`  | `bigint` | `bigint`  | `BigNumber` | `uint64`   | `u64`  |
| `uint128` | `bigint` | `bigint`  | `BigNumber` | `*big.Int` | `u128` |
| `uint256` | `bigint` | `bigint`  | `BigNumber` | `*big.Int` | `U256` |

Python integer outputs use `int`. Other common mappings include Go
`common.Address`, Rust `Address`/`Bytes`, and TypeScript hex-string address and
byte values. Dynamic arrays and fixed arrays have target-specific forms; ethers
fixed-size arrays are emitted as tuples.

## Documentation and identifiers

Renderers propagate available NatSpec into their language's documentation syntax.
A raw ABI without documentation metadata cannot supply source comments.

Python, Rust, Swift, and Kotlin parameter names that match reserved words receive
an underscore prefix:

| Language | Examples                                           |
| -------- | -------------------------------------------------- |
| Python   | `from` becomes `_from`; `lambda` becomes `_lambda` |
| Rust     | `type` becomes `_type`; `self` becomes `_self`     |
| Swift    | `self` becomes `_self`; `func` becomes `_func`     |
| Kotlin   | `fun` becomes `_fun`; `when` becomes `_when`       |

Compile or type-check generated files in the consuming project, especially after
changing SDK versions, tuple shapes, overloads, or contract names. Regenerate from
the ABI rather than editing generated output by hand.
