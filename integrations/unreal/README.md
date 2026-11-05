# Unreal ABI adapters

Generate with `--target unreal --package YourModule`. The plugin provides the
shared Rust ABI runtime, Unreal HTTP transport, async lifetime handling and
lossless reflected address/byte/256-bit word values.

Each function has typed `Encode<Name>` and `Decode<Name>` methods on its generated
`U<Contract>UnrealMetadata` class. ABI types accepted by the parser have reflected
codecs: tuples become named USTRUCTs, arrays become TArrays, and nested arrays use USTRUCTs with a
`Values` member because UHT forbids nested reflected containers. Fixed arrays
use TArrays and their encoder enforces the ABI length. Type and helper names
separate each ancestor with `_` to prevent array-element and sibling-name
collisions. Integers, including signed two's complement values, remain full
`FAbiTypegenWord256` words.

View/pure functions have generated Blueprint async read nodes. Events have
`Decode<Name>Event` helpers and generated Blueprint `U<Contract><Name>EventWatcher`
actions with typed `OnEvent` and `OnFailure` delegates. Indexed strings, bytes,
arrays and tuples expose their topic hash as bytes32; the original indexed value
cannot be recovered from that hash. Custom errors have typed `Decode<Name>Error`
helpers. Overloads receive distinct method/type names.

`--no-wrappers` retains the ABI, native C bindings, reflected DTOs and metadata.
It removes reflected callable codec helpers, read nodes and event watcher actions.

## Transactions and receipts

Encode calldata with the generated helper and construct an
`FAbiTypegenUnsignedTransaction`. Reuse your application's `IAbiTypegenSigner`
implementation for submission. There is no built-in signer or key storage.

The native C++ `FAbiTypegenRpcClient::SimulateTransaction` and `EstimateGas`
helpers take the unsigned transaction and an explicit sender. Simulation validates
returned byte hex. Gas estimation validates a JSON-RPC quantity up to 256 bits;
`ParseGasEstimate` converts it to an exact reflected word. RPC revert data remains
available on the response for the generated custom-error decoder.

Retain a native `FAbiTypegenPollSession::WaitReceipt` handle to wait for mining
and confirmations. Construct its transport with
`FAbiTypegenPollSession::HttpTransport(RpcUrl, RequestOwner)`. A canonical mined
receipt returns its execution status in `bSucceeded`: false means the transaction
reverted. Transport, timeout and reorg errors are separate completion errors.
Confirmations include the containing block. Confirmation checks re-read the
receipt, canonical block and chain head before completion. The receipt adapter
is a native C++ API; the generated Blueprint surface covers reads, codecs and
event watching.

## Event cursors and lifetime

Blueprint watcher actions require an explicit start block, confirmation count,
range size, poll interval and timeout. The native `WatchLogs` API accepts the
same policy plus exact positional topics (up to four). Topic wildcards and OR
filters are not exposed by this adapter. Range size is limited to 10,000 blocks.

The poller checks the previous range's canonical anchor, captures the next range
endpoint before `eth_getLogs`, then validates the endpoint and every returned
log block before delivery. A detected reorg terminates the session with an
explicit error; choose a safe restart block. RPC, malformed-log and decode errors
retain the current cursor. Logs are ordered by block and log index. Duplicate
or removed logs in a range are rejected. Native range callbacks return true only
when the entire range has decoded successfully; false terminates without
advancing. Generated watchers decode the entire range before broadcasting.

Keep handles alive until completion. Dropping a native polling handle or calling
`Cancel` stops its requests and suppresses future callbacks. Generated actions
also stop on owner destruction or UObject teardown. Calls and callbacks use the
game thread. A polling timeout includes time spent in in-flight HTTP requests.
