# Choosing a binding tool

Choose based on the output your application needs, the SDK it uses, and how
artifacts enter your build. Compare generated files and runtime behavior before
replacing an existing generator.

## Where abi-typegen fits

abi-typegen reads Foundry and Hardhat artifacts and generates multiple targets
from one configuration. Its CLI supports stale-output checks, contract filters,
and artifact watching. The native binary runs without Node; the npm launcher and
Hardhat plugin use Node.

The [output guide](generated-output.md) describes the generated files. TypeScript
wrapper targets provide SDK-specific helpers or interfaces. The Rust target
provides Alloy contract bindings. Go, Swift, Kotlin, C#, Java, and Dart provide
SDK-backed contract APIs. PHP generates codecs and JSON-RPC/signing helpers. C/C++
use a shared codec runtime and application-supplied transport. See the [native
guide](native-bindings.md) for runtime requirements and tested boundaries.
Generated APIs are not drop-in replacements for every SDK-specific generator.

## Related tools

| Tool                                                      | When to evaluate it                                                                                                                                                     |
| --------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [TypeChain](https://github.com/dethcrypto/TypeChain)      | You need its SDK-specific TypeScript output or compatibility with an existing TypeChain integration. Check its repository for maintenance status and supported targets. |
| [ABIType](https://github.com/wevm/abitype)                | You need TypeScript type transformations and inference directly from ABI literals. abi-typegen's `as const` ABI modules can be inputs to this approach.                 |
| [abigen](https://geth.ethereum.org/docs/tools/abigen)     | You need Go contract bindings and call/transact integration with go-ethereum. Compare that generated API with abi-typegen's Go wrappers.                                |
| [forge bind](https://getfoundry.sh/forge/reference/bind/) | You need Foundry's Rust binding workflow. Compare its contract interaction output with abi-typegen's Rust data types.                                                   |

Use each tool's documentation for its supported SDK versions and output contract.
Language coverage alone does not establish API compatibility.

## Ethers migration considerations

abi-typegen's `ethers` target uses ethers v6 types; `ethers5` uses ethers v5 types.
Both are exercised by generated-code compilation and decoding tests in the
repository. When migrating, inspect:

- Integer inputs and decoded outputs. Inputs use `BigNumberish`; v6 integer outputs
  use `bigint`, while v5 uses `number` for widths up to 48 bits and `BigNumber` above
  that boundary.
- Bytes and address inputs, which use SDK input aliases rather than only decoded
  output types.
- Fixed-size arrays and named tuple results, including positional access.
- Overloaded methods and aliases. Generated aliases dispatch through canonical
  ABI signatures rather than assuming the alias exists on the SDK contract.
- Factories, deployment helpers, and encode/decode overloads used by your app.
  abi-typegen does not generate TypeChain's entire class/factory API.

## Measuring generation time

Run comparisons on the same contracts, target SDK, and output requirements.
Record tool versions, hardware, contract set, warmup, and whether compilation is
included. A native executable by itself does not guarantee a particular speedup.

The repository includes [e2e/bench.sh](../e2e/bench.sh). Install dependencies in
both sample projects before running it:

```sh
./e2e/bench.sh 10
```

The script builds the release CLI and prepares the sample artifacts, then warms
the tools and compares repeated abi-typegen ethers generation with TypeChain
through Hardhat. It requires the sample projects' dependencies, Forge, pnpm, and
Python.

[TypeChain's Hardhat task](https://github.com/dethcrypto/TypeChain/blob/master/packages/hardhat/src/index.ts)
invokes compilation internally. That path can also run other configured compile
hooks; the repository's Hardhat sample loads the abi-typegen plugin too. Verify
the actual artifact sets and enabled hooks before interpreting a timing ratio.
These are workflow measurements, not an isolated comparison of renderer functions.

The script removes and regenerates sample output directories between runs. If
using optional storage redirection, follow the [storage guide](development-storage.md)
when restoring links after benchmarking. Treat results as local measurements;
do not carry a fixed speedup claim across machines or future releases.
