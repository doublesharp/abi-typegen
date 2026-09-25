# Native contract bindings

Generate bindings from the same artifacts used for TypeScript:

```sh
abi-typegen generate --target go,swift,kotlin,csharp,java,dart,php,c,cpp
```

When selecting multiple targets, each gets its own output directory. Install the consuming language's SDK
separately; the generator does not install dependencies. `--no-wrappers` keeps
primary ABI metadata and value types while omitting callable helpers.

## Runtime dependencies

| Target       | Consumer runtime                                                                                               |
| ------------ | -------------------------------------------------------------------------------------------------------------- |
| Python       | web3.py 8 (Python 3.10+)                                                                                       |
| Go           | go-ethereum 1.17                                                                                               |
| Swift        | web3swift 3.x with Web3Core and BigInt; Swift 6                                                                |
| Kotlin, Java | web3j 6; JDK 21                                                                                                |
| C#           | Nethereum.Web3 6.1                                                                                             |
| Dart         | web3dart 3                                                                                                     |
| PHP          | PHP 8.2+, Brick Math 1.0.0, `web3p/ethereum-tx` 0.4.3, and cURL; enable `curl`, `gmp`, `mbstring`, and `iconv` |
| Ruby         | eth 0.5.17 and Bundler                                                                                         |
| Elixir       | Ethers 0.8.0; `ex_secp256k1` 0.8.0 for local signing                                                           |
| Shell        | Bash 3.2+ and Foundry cast                                                                                     |
| COBOL        | GnuCOBOL 3.2 and shared Rust runtime; optional libcurl/json-c RPC                                              |
| Godot        | Godot 4.7.2, GDExtension, pinned godot-cpp 10.0.0, SCons 4.10.0, and shared Rust runtime                       |
| Unreal       | Unreal Engine 5.8.3, the generated plugin, and shared Rust runtime                                             |
| C, C++       | `abi-typegen-runtime`, built with Rust/Alloy; C11 or C++17 compiler                                            |

SDK-backed wrappers use the application's provider and signing configuration.
The generated PHP client accepts an RPC URL and can sign locally with a supplied
private key; the application remains responsible for storing that key. Other
targets leave RPC and signing to the SDK integration. Read operations return typed
results; write operations expose the target SDK's transaction or operation object.
Preparing a transaction, submitting it, and waiting for its receipt are distinct
operations.

Canonical signatures select overloads. Except for the limited COBOL target,
generated codecs cover positional
arguments, tuples, arrays, results, declared custom errors, and events. Indexed
strings, bytes, arrays, and tuples decode to their 32-byte topic hashes because
the original values are absent from the log. Anonymous events have no signature
topic; the caller must choose the appropriate event decoder.

## PHP

PHP output requires PHP 8.2 or newer, the `curl`, `gmp`, `mbstring`, and `iconv`
extensions, Brick Math 1.0.0, and `web3p/ethereum-tx` 0.4.3. Install the PHP
packages in the consuming project:

```sh
composer require brick/math:^1.0 web3p/ethereum-tx:^0.4.3
abi-typegen generate --target php --out ./generated --package 'App\Contracts'
```

Generated files include named tuple, event, error, and transaction-option classes,
plus one contract class per ABI. Contract classes provide strict offline encoders
and decoders, JSON-RPC read helpers, signed transaction helpers, receipt queries,
log queries, event filters and decoders, and declared custom-error decoders. The
client signs legacy EIP-155 transactions. It does not generate
EIP-1559 transactions, deployment helpers, or subscriptions. The upstream signing
package emits `ArrayAccess` return-type deprecation notices on PHP 8.5.

Given a generated `Token` binding, a read uses the configured RPC client:

```php
<?php
require __DIR__ . '/vendor/autoload.php';
foreach (glob(__DIR__ . '/generated/*.php') ?: [] as $file) require_once $file;

use App\Contracts\Token;
use App\Contracts\TokenClient;
use Brick\Math\BigInteger;

$client = new TokenClient(getenv('TOKEN_ADDRESS'), getenv('RPC_URL'));
$balance = Token::callBalanceOf(
    $client,
    getenv('ACCOUNT_ADDRESS'),
);
echo $balance->toBase(10);
```

For writes, construct the client with a private key, sender address, and chain ID.
Pass gas price and gas limit through the generated transaction options, then wait
for a receipt before treating the returned hash as a successful transaction.

## Ruby

Ruby bindings use the `eth` gem. Reads execute through your `Eth::Client`; write
methods build transaction hashes for your application to sign and submit. Use
`eth` 0.5.17 with Bundler; its secp256k1 dependency may require a system library.

```sh
abi-typegen generate --target ruby --out ./generated
```

```ruby
require_relative "generated/Token"

client = Eth::Client.create(ENV.fetch("RPC_URL"))
token = TokenContract.new(ENV.fetch("TOKEN_ADDRESS"), client)
owner = ENV.fetch("ACCOUNT_ADDRESS")
balance = token.balance_of(owner)
calldata = token.encode_transfer(owner, 1000)
transaction = token.transfer(owner, 1000, gas_limit: 100_000)
```

A write builder returns a Ruby `Hash` containing transaction fields. It does
not submit a transaction or return an on-chain transaction identifier. Supply signing, nonce and fee policy through
the SDK. Generated helpers also cover result decoding, event filters and log
decoding, custom errors, and constructor data. Large integers use Ruby integers.
`--no-wrappers` keeps ABI metadata and named tuple values without requiring `eth`.

## Shell

The `shell` target emits Bash libraries backed by Foundry `cast`. Source the file
and call its functions; Cast handles ABI values, RPC, and signing.

```sh
abi-typegen generate --target shell --out ./generated
```

```bash
source ./generated/Token.sh
export ATG_RPC_URL=http://127.0.0.1:8545
atg_token_balance_of_call "$TOKEN_ADDRESS" "$OWNER_ADDRESS"
atg_token_transfer_encode "$RECIPIENT_ADDRESS" 1000
```

Set `ATG_CAST_BIN` to select the Cast executable. RPC and wallet settings use
`ATG_RPC_URL`, `ATG_PRIVATE_KEY`, `ATG_FROM`, and Bash arrays such as
`ATG_CAST_SEND_ARGS`. The generated library does not change the caller's shell
options. Pass tuple and array values using Cast's argument syntax, quoted as one
shell argument. Keep uint256 values as strings instead of shell arithmetic.

Read helpers offer decoded `*_call` and raw `*_call_raw` results. Writes use
`*_send`; constructor helpers provide argument encoding and deployment from
caller-supplied bytecode. Event `*_logs` helpers delegate filtering to Cast;
`*_decode_data` decodes only the nonindexed payload. Indexed values remain in log
topics. Anonymous event signature filtering is unsupported. The installed Cast
version determines output formatting and wallet options. These are Bash scripts,
not portable POSIX `sh` libraries.

## COBOL (experimental)

COBOL can read a token balance through the shared Rust runtime. This target
supports `view` or `pure` functions with exactly one `address` input and one
`uint256` output. All functions, events, and errors retain signature metadata
comments; unsupported signatures do not get callable subprograms.

```sh
abi-typegen generate --target cobol --out ./generated
cargo build --release -p abi-typegen-runtime
```

For `Token.balanceOf`, the generated `Token.cob` contains
`TOKEN-BALANCE-OF-ENC`, `TOKEN-BALANCE-OF-DEC`, and `TOKEN-BALANCE-OF-CALL`.
`Token.cobol.c` bridges fixed COBOL buffers to the runtime and releases its owned
results. Compile the COBOL as free-form source with GnuCOBOL. Link both sources
with the matching `abi-typegen-runtime` library. This does not require a COBOL
implementation of the Ethereum ABI.

Offline encoding and decoding require only the shared runtime. For RPC reads,
compile the bridge with `-DATG_COBOL_WITH_CURL=1` and link libcurl and json-c.
Without that option, `*-CALL` reports an error while codecs remain available.
Reads use `latest` and omit `from`, so the provider chooses the default caller.
The CALL helper cannot configure a sender or historical block; use another client
with the offline codec for reads that depend on those settings.
The generated source defines the required buffer sizes and argument order.
`uint256` results are ASCII decimal text in `PIC X(78)`, accompanied by a length.
Status zero means success; a nonzero status comes with an error buffer.

Use GnuCOBOL for compilation and libcurl and json-c for optional RPC reads.
IBM Enterprise COBOL compatibility has not been established.

This target does not generate writes, signing, deployment, event/error decoding,
other scalar signatures, tuples, or arrays. `--no-wrappers` keeps signature
comments and the C ABI constant, with no callable COBOL programs.

## C and C++

C generates `atg_<Name>.h`. C++ generates that C header plus `atg_<Name>.hpp`. Both ship
`abi_typegen.h`, which declares the runtime interface. Build the matching runtime:

```sh
cargo build --release -p abi-typegen-runtime
```

Link the resulting `abi_typegen_runtime` shared or static library. Cargo writes it
to the active target directory. The generator's npm executable does not contain
this library. Header-only metadata output does not require linking the runtime.

C uses 32-byte big-endian words for signed and unsigned integers; signed values
use two's complement. Addresses hold 20 bytes. Byte and string values have an
explicit length; strings contain UTF-8. Fixed arrays have a fixed C array member,
and dynamic arrays contain a typed pointer and length. Tuple types are reusable
structs. Field names have an `atg_` prefix to avoid C/C++ keywords.

Encoding functions return an owned `atg_result *`. Check `atg_result_error`, use
`atg_result_data` and `atg_result_len`, then call `atg_result_free`. Decoding fills
a typed result struct whose strings and arrays borrow storage from the returned
runtime result. Keep that result alive until all borrowed fields are no longer
used. Never free a borrowed `atg_value` child.

C++ `Decoded<T>` owns the runtime result and exposes its fields through `value()`
or `operator->`. It is movable, not copyable. Encoding returns a
`std::vector<uint8_t>`; failed operations throw `std::runtime_error`.

A generated client accepts an address, application context, and an
`atg_transport_fn`. The callback receives typed transaction options and encoded
calldata. It returns an owned runtime result containing return bytes for a read
or the submitted transaction hash for a write. The adapter handles networking,
signing, nonce policy, and receipt polling. Deployment helpers accept creation
bytecode from the caller; deployment callbacks receive a null destination.

Event filter helpers construct topic filters; null indexed topics are wildcards.
Pass the filter to the application's log provider and decode returned logs with
the generated event decoder. Hash indexed reference values before filtering.

## Current boundaries

- Java and Kotlin use web3j's input encoder. Fixed arrays outside lengths 1–32,
  including those nested in tuples or arrays, are unsupported inputs and fail
  explicitly. Generated decoders cover nested dynamic arrays and tuples.
- Deployment helpers are available for Python, Go, C#, C, and C++. Supply creation bytecode
  from your compiler artifacts. Other targets can use their underlying SDK's
  deployment API.
- PHP submits legacy EIP-155 transactions. It does not generate EIP-1559
  transactions, deployment helpers, or subscriptions. `web3p/ethereum-tx` emits
  `ArrayAccess` return-type deprecation notices on PHP 8.5.
- Event query and subscription convenience APIs vary by target. Go includes typed
  subscriptions through a backend that supports subscriptions (for example WebSocket). Swift and C/C++
  provide topic builders and decoders; their provider integration remains with the
  application. Generated fallback and receive transaction helpers are not uniform;
  use the SDK's raw transaction API when needed.
- C/C++ require a separately built runtime library and an application transport.
  Release executables do not bundle a precompiled runtime for every platform.

These boundaries differ from information that Ethereum never supplies: indexed
reference values are hashes, and anonymous events do not identify their own ABI.

## Elixir

Generated contract functions prepare transaction data. Pass that data to Ethers
when you want to read from the chain or submit a signed transaction.

```sh
abi-typegen generate --target elixir --out ./lib/contracts
```

Add `{:ethers, "== 0.8.0"}` to your Mix dependencies. A generated Token read is:

```elixir
Token.balance_of(owner)
|> Ethers.call(to: token_address, rpc_opts: [url: rpc_url])
```

Writes use `Ethers.send_transaction/2` with the application's signer configuration.
Event filters live under `Token.EventFilters`; custom-error structs and decoders
live under `Token.Errors`. `Token.abi_json/0` exposes the complete ABI.
`--no-wrappers` emits only the metadata module, which needs no Ethers dependency.

Function names use snake case. Distinct ABI names that would collide receive
unique aliases while retaining their original selectors. Event-filter name
collisions, event overloads with indistinguishable indexed arguments, overloaded
custom-error names, and functions that shadow reserved Ethers helpers are
rejected before files are written. The generator does not supply a wallet or
manage transaction receipts for the application.

## Unity compatibility

Unity uses the existing `csharp` target. The adapter package at
`integrations/unity/com.doublesharp.abi-typegen.unity` adds Unity HTTP transport,
an application signer interface, and cancellation tied to GameObject lifetime.
See the [Unity adapter package](../integrations/unity/com.doublesharp.abi-typegen.unity/README.md)
for setup.
Supported configurations include Unity 6000.6.3f1 on macOS with Mono and IL2CPP.
Linux, Android, iOS, and WebGL compatibility is not yet verified.

## Godot (experimental)

The `godot` target emits GDScript ABI bindings and uses a Godot 4 GDExtension for
the shared codec and asynchronous HTTP JSON-RPC client. The extension links the
Rust runtime statically and is compiled against C ABI v1. It currently builds
for Linux x86_64 and macOS arm64. This integration is experimental; other
platforms and export templates are not yet supported. Build the extension
with Godot 4.7.2, SCons 4.10.0, and the pinned godot-cpp dependency in
[`integrations/godot/extension`](../integrations/godot/extension/).

Use `--no-wrappers` to emit ABI metadata without generated codec/client calls.
The GDExtension is not needed to load that metadata-only output. Write wrappers
accept an application-provided signer.
No local signer, deployment wrapper, event subscriptions, or fallback/receive
wrappers are bundled. Constructor encoding is available. uint256 values use
exact 32-byte `PackedByteArray` words.

## Unreal (experimental)

The `unreal` target emits a plugin-facing C++ API and generated C ABI codecs.
The plugin uses C ABI v1, linking the Rust runtime statically on macOS and through
a DLL on Windows. Blueprint nodes cover asynchronous scalar `pure` and `view` reads;
the native generated codec API covers the full supported ABI values. Generated
reads use `eth_call` with `to` and `data` at `latest`, without a `from` value, so
views that depend on `msg.sender` use the provider's default caller. Applications
that need another caller can use the generated calldata with their own RPC flow.
The plugin uses the application's provider and signer; it does not include a
wallet or local key store.

The [Unreal plugin](../integrations/unreal/Plugins/AbiTypegen/) targets Unreal
Engine 5.8.3 on macOS arm64. Windows build rules are provided, but Windows
compatibility has not been established; Linux support is not implemented.
