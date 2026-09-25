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
| Python       | web3.py; the consumer fixture pins 8.0.0 (Python 3.10+)                                                        |
| Go           | go-ethereum; the consumer fixture pins 1.17.6                                                                  |
| Swift        | web3swift 3.x with Web3Core and BigInt; Swift 6                                                                |
| Kotlin, Java | web3j 6; JDK 21                                                                                                |
| C#           | Nethereum.Web3; the consumer fixture pins 6.1.0                                                                |
| Dart         | web3dart; the consumer fixture pins 3.0.3                                                                      |
| PHP          | PHP 8.2+, Brick Math 1.0.0, `web3p/ethereum-tx` 0.4.3, and cURL; enable `curl`, `gmp`, `mbstring`, and `iconv` |
| C, C++       | `abi-typegen-runtime`, built with Rust/Alloy; C11 or C++17 compiler                                            |

SDK-backed wrappers use the application's provider and signing configuration.
The generated PHP client accepts an RPC URL and can sign locally with a supplied
private key; the application remains responsible for storing that key. Other
targets leave RPC and signing to the SDK integration. Read operations return typed
results; write operations expose the target SDK's transaction or operation object.
Preparing a transaction, submitting it, and waiting for its receipt are distinct
operations.

Canonical signatures select overloads. Generated codecs cover positional
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

## Tests against a local chain

`e2e/native/anvil.py` starts its own Anvil process on an ephemeral localhost port,
waits for the expected chain ID, and deploys the Token fixture. It supplies the
consumer command with `ATG_RPC_URL`, `ATG_TOKEN_ADDRESS`, `ATG_PRIVATE_KEY`, and
`ATG_CHAIN_ID`, then stops its node even if the consumer fails. The private key is
a public Anvil development key and must never be used with real funds.

For example, after generating Go bindings:

```sh
python3 e2e/native/anvil.py --cwd e2e/native/go \
  go test ./usage -run TestGeneratedBindingsAnvil -count=1
```

The native Make targets generate source, compile consumers, run codec tests, and
exercise RPC submissions where supported. PHP uses `make e2e-php`. GitHub Actions installs Foundry and
each language's toolchain before running those same targets. These tests need no
external chain, RPC service, or repository secret.

Codec tests and Anvil tests serve different purposes. Codec tests cover malformed
bytes, large integers, nested values, overload selection, and ownership. Anvil
checks verify signed submission, receipts, state changes, and event handling in
the consumer runtime. A passing local-chain test is not a production-network
qualification.

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
