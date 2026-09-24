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
| `go`       | `<Name>.go`    | ABI, selectors, and structs in go-ethereum's type model       |
| `rust`     | `<name>.rs`    | alloy `sol!` bindings plus the JSON ABI, with a `mod.rs`      |
| `swift`    | `<Name>.swift` | ABI, selectors, and public value types for web3swift          |
| `csharp`   | `<Name>.cs`    | C# contract-related types                                     |
| `kotlin`   | `<Name>.kt`    | ABI, selectors, and value types built on web3j                |
| `solidity` | `I<Name>.sol`  | Solidity interface reconstructed from ABI data                |
| `yaml`     | `<Name>.yaml`  | Human-readable functions, events, errors, and parameter types |

The Solidity target reconstructs tuple structs and emits events, errors,
overloads, and external function signatures. It cannot recover a contract's
implementation from an ABI.

Output APIs vary by language. Rust output includes alloy's contract instance.
The Go, Swift, and Kotlin output does not yet include a bound contract client or
typed call wrappers. Inspect the output before integrating it with your runtime SDK.

## Go, Rust, Swift, and Kotlin

Each file embeds the contract's JSON ABI plus the canonical signature and selector
of every function, event, and error. Each tuple becomes a named type.

| Target | SDK and dependencies                                                       | Layout                                                |
| ------ | -------------------------------------------------------------------------- | ----------------------------------------------------- |
| Go     | `github.com/ethereum/go-ethereum` (`common`, plus `math/big`)              | One file per contract in the configured `package`     |
| Rust   | `alloy` with `contract` and `serde` (or `sol-types`, `json`, `serde` without wrappers), and `serde` with `derive` | `<name>.rs` per contract, re-exported from `mod.rs` |
| Swift  | web3swift 3.x (`BigInt`, `Web3Core`); depend on the `web3swift` product    | `public enum <Name>` holding every type and constant  |
| Kotlin | `org.web3j:abi` (web3j 6 requires JDK 21)                                  | `object <Name>` in the configured `package`           |

- **Go** follows abigen. Constants are `<Name>TransferSignature`,
  `<Name>TransferSelector` (`[4]byte`), `<Name>TransferEventTopic`
  (`common.Hash`), and `<Name>XErrorSelector`. Structs are
  `<Name><Tuple>`, `<Name>TransferParams`, `<Name>TransferEvent`, and
  `<Name>XError`. Output is gofmt-clean, and structs pack and unpack through
  go-ethereum's `abi` package. When two ABI names export to the same Go field
  name, the suffixed fields carry `abi:"..."` tags.
- **Rust** emits `alloy::sol! { #[sol(rpc, abi, all_derives, extra_derives(serde::Serialize, serde::Deserialize))] contract <Name> { ... } }`
  and a `<NAME>_ABI` string constant. alloy generates the structs, `…Call`
  and `…Return` types, events with indexed fields, errors, and selectors. Types
  derive `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `Default` (where every field
  supports it), and serde with ABI field names. serde supports arrays of at most
  32 elements, so a contract with a longer fixed array omits the serde derives.
  `wrappers = false` removes only `rpc`. NatSpec becomes rustdoc that passes
  `-D warnings`. A contract whose module name is a Rust keyword uses a raw
  identifier (`pub mod r#override;`).
- **Swift** types are `public struct <Type>: Sendable, Hashable` with a public
  memberwise initializer. Constants are static lets such as
  `Token.transferSelector` (`Data`) and `Token.TransferEventTopic`. Output builds
  in Swift 6 language mode.
- **Kotlin** tuples are data classes that extend web3j's `StaticStruct` or
  `DynamicStruct`, so they encode directly. Integers are `BigInteger`, addresses
  are `String`, `bytesN` is web3j's `BytesN`, and `bytes` is `DynamicBytes`, all
  compared by value. Constants are `const val` strings such as
  `Token.TRANSFER_SELECTOR`. Names that shout to the same constant get a numeric
  suffix in ABI order (`premiumPeriod` after `PREMIUM_PERIOD` gives
  `PREMIUM_PERIOD_SIGNATURE2`). Java reads the constants, the `Token.JSON` ABI, and every
  getter without name mangling. A struct field named `value`, `typeAsString`, or
  `componentType` gets a trailing underscore because web3j's `Array` already
  defines those getters. web3j cannot encode fixed arrays longer than 32 elements.

In Swift and Kotlin, a tuple named like a type the generated code uses (`Data`,
`Address`, `Uint256`) gets a suffix (`Data2`, `Uint256Tuple`) so it cannot shadow
that type. Anonymous events have no topic constant, since they do not log their signature
hash. A struct declared in the rendered contract drops the contract qualifier:
`struct Vault.Position` in `Vault` is `Position` (Go `VaultPosition`). Structs from
other contracts and libraries keep it (`TupleAccountPosition`). Unnamed parameters
are named after their types (`address`, `address2`, `uint256`, `bytes32Array`);
Rust keeps alloy's `_0`, `_1`.

## Overloaded functions

TypeScript wrapper and type names distinguish overloaded signatures. For example:

```text
deposit(uint256)          -> depositUint256
deposit(uint256,address)  -> depositUint256Address
```

Ethers wrappers use canonical ABI signatures to select the runtime method. Tuple
and array inputs participate in signature naming. When aliases would collide with
another alias or an existing method name, the ethers renderer adds a numeric
suffix. Wagmi read, write, and event hooks share one module namespace. Functions
whose names differ only in casing or separators, such as `PREMIUM_PERIOD` and
`premiumPeriod`, keep their original spelling in the hook name
(`useTokenPREMIUM_PERIOD`, `useTokenPremiumPeriod`). Any remaining clash gets a
numeric suffix. web3.js wrappers type overloads under the keys web3 registers at
runtime: the plain name, which picks an overload by argument count, and the
quoted signature, such as `methods['deposit(uint256)']`.

The native targets number overloads the way their SDKs do, in ABI order:

| Target        | `safeTransferFrom(a,b,c)`          | `safeTransferFrom(a,b,c,d)`        |
| ------------- | ---------------------------------- | ---------------------------------- |
| Go            | `TokenSafeTransferFromParams`      | `TokenSafeTransferFrom0Params`     |
| Rust (alloy)  | `safeTransferFrom_0Call`           | `safeTransferFrom_1Call`           |
| Swift, Kotlin | `SafeTransferFrom0Params`          | `SafeTransferFrom1Params`          |

Names keep acronyms: `tokenURI` gives `TokenURI`, not `TokenUri`. Treat generated
names as part of the output API and recompile consumers when the ABI changes.

## Contract types

Where the SDK has its own contract type, the wrapper builds on it, so SDK members
stay available:

- ethers v6: `<Name>Contract` is `BaseContract` combined with the generated
  `<Name>Methods`, so `getAddress()`, `interface`, `target`, `on()`, and
  `queryFilter()` are typed. `connect()` returns `<Name>Contract`. Event filters
  return `DeferredTopicFilter`, which `queryFilter()` and `on()` accept.
- ethers v5: `<Name>Contract` extends `ethers.Contract`.
- web3.js: `<Name>Contract` is web3's `Contract<typeof <Name>Abi>` with `methods`
  replaced by `<Name>Methods`. Events, `options`, and each method's `send()`,
  `estimateGas()`, and `encodeABI()` keep web3's types. The wrapper narrows
  arguments and `call()` results.
- viem: `get<Name>Contract` returns viem's `GetContractReturnType`.

## Transaction options

Wrappers type the value and overrides each SDK accepts, based on the function's
state mutability:

| Target    | View / pure                            | Nonpayable                             | Payable                                     |
| --------- | -------------------------------------- | -------------------------------------- | ------------------------------------------- |
| ethers v6 | `overrides?: Omit<Overrides, 'value'>` | `overrides?: Omit<Overrides, 'value'>` | `overrides?: Overrides`                     |
| ethers v5 | `overrides?: CallOverrides`            | `overrides?: Overrides`                | `overrides?: PayableOverrides`              |
| wagmi     | n/a                                    | `write(args)`                          | `write(args, options?: { value?: bigint })` |
| web3.js   | `call(options?, block?)`               | web3's `send(options?)`                | web3's `send(options?)`, with `value`       |

The trailing ethers parameter is named `overrides` unless an ABI input already
uses that name, in which case it gets an underscore prefix. Viem helpers return
viem's `GetContractReturnType`, so `contract.write.<fn>(args, { value })` accepts
a value for payable functions.

Wrapper return types are written out explicitly, so projects that emit
declarations (`declaration: true`) can re-export generated wrappers for large
ABIs.

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

Unsigned examples below show the size boundaries. Go and Rust use a native integer
only for widths their SDKs decode natively (Go: 8, 16, 32, and 64 bits; alloy also
128) and a big-integer type otherwise. Swift uses `BigUInt`/`BigInt` and Kotlin
uses `BigInteger` for every width.

| Solidity  | Viem     | Ethers v6 | Ethers v5   | web3.js  | Go         | Rust   |
| --------- | -------- | --------- | ----------- | -------- | ---------- | ------ |
| `uint8`   | `number` | `bigint`  | `number`    | `bigint` | `uint8`    | `u8`   |
| `uint24`  | `number` | `bigint`  | `number`    | `bigint` | `*big.Int` | `U24`  |
| `uint48`  | `number` | `bigint`  | `number`    | `bigint` | `*big.Int` | `U48`  |
| `uint56`  | `bigint` | `bigint`  | `BigNumber` | `bigint` | `*big.Int` | `U56`  |
| `uint64`  | `bigint` | `bigint`  | `BigNumber` | `bigint` | `uint64`   | `u64`  |
| `uint128` | `bigint` | `bigint`  | `BigNumber` | `bigint` | `*big.Int` | `u128` |
| `uint256` | `bigint` | `bigint`  | `BigNumber` | `bigint` | `*big.Int` | `U256` |

web3.js outputs follow web3 v4's default return format. web3.js integer inputs
accept web3's `Numbers` type (`number | bigint | string`).

Python integer outputs use `int`. Other common mappings include Go
`common.Address`, Rust `Address`/`Bytes`, and TypeScript hex-string address and
byte values. Dynamic arrays and fixed arrays have target-specific forms; ethers
fixed-size arrays are emitted as tuples.

## Documentation and identifiers

Renderers propagate available NatSpec into their language's documentation syntax.
A raw ABI without documentation metadata cannot supply source comments.

Parameter names that match reserved words are adjusted:

| Language | Examples                                                   |
| -------- | ---------------------------------------------------------- |
| Python   | `from` becomes `_from`; `lambda` becomes `_lambda`         |
| Rust     | `type` becomes `type_`; `self` becomes `self_`             |
| Swift    | `default` becomes `` `default` ``; `func` becomes `` `func` `` |
| Kotlin   | `in` becomes `` `in` ``; `when` becomes `` `when` ``        |

Compile or type-check generated files in the consuming project, especially after
changing SDK versions, tuple shapes, overloads, or contract names. Regenerate from
the ABI rather than editing generated output by hand.
