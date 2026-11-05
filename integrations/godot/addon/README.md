# Godot JSON-RPC operations

Copy this directory to `res://addons/abi_typegen/` alongside the abi-typegen
GDExtension. Import the project so Godot registers the addon classes. Generated
contract files remain in their own directory; this addon is shared by all of them.
The native test runner stages these scripts automatically before import or tests.

Add an `AbiTypegenClient` to the scene tree and set its `rpc_url`. The client owns
receipt operations, watchers, and HTTP requests. Connect signals or await
`finished` immediately after creating an operation. Polling starts on a deferred
call so listeners can connect first.

```gdscript
var simulation = await AbiTypegenOperations.simulate(
    client, {"to": contract_address, "data": calldata_hex}, "pending"
).finished
var estimate = await AbiTypegenOperations.estimate_gas(
    client, {"to": contract_address, "data": calldata_hex, "from": account}
).finished
var pending = AbiTypegenOperations.wait_for_receipt(client, transaction_hash, 3, 1.0, 120.0)
var receipt = await pending.finished
```

Simulation returns `eth_call` hexadecimal bytes. Gas estimation returns an exact
hexadecimal quantity, including estimates wider than signed 64 bits. Transactions
use RPC strings for quantities and addresses; the native client rejects JSON
numeric parameters. These helpers do not sign or submit transactions.

Receipt waits return a successful receipt only after its confirmation depth and
canonical block hash and number are checked. Before settlement, the operation
reads the receipt again, verifies its canonical block again, and rereads the head.
A changed receipt or lost confirmation depth fails with `reorg`. A confirmed
canonical receipt with
`status == "0x0"` fails with `transaction_reverted`. An orphaned or disappeared
canonical block fails with `reorg`. The deadline covers both polling delays and
pending HTTP requests. `cancel()` stops the pending transport and further polls.
Removing the owner from the tree settles unfinished operations as cancelled.

`finished` emits `{ "ok": true, "value": value }` or
`{ "ok": false, "error": { "code": code, "message": message } }`.
`completed(value)` and `failed(error)` provide the corresponding value separately.
Cancellation also emits `cancelled()`. Receipt nodes and watcher nodes free
themselves after settlement. Owner-destruction signals can fire during tree
cleanup; defer scene changes from those listeners.

## Event polling

```gdscript
var watcher = AbiTypegenOperations.watch_events(
    client,
    {"address": contract_address, "topics": [Token.TRANSFER_EVENT_TOPIC]},
    Callable(Token, "decode_transfer_event_typed"),
    starting_block,
    3,
    1.0,
    100
)
watcher.event_received.connect(_on_transfer) # Receives decoded value and raw log.
watcher.failed.connect(_on_watch_failed)
# Later:
watcher.cancel()
```

`from_block` is required. The watcher sends explicit inclusive `fromBlock` and
`toBlock` ranges, bounded by `block_batch_size` and the sampled head minus the
confirmation depth. Filters accept `address` and `topics`; range fields and
`blockHash` are rejected because the watcher owns its cursor.

The decoder receives `(topics: Array, data: PackedByteArray)` and must return the
same success or failure envelope. Generated `decode_*_event_typed` helpers return
typed event objects. Ordinary `decode_*_event` helpers return ABI values.

Every log in a range must pass metadata, filter, decoding, and canonical block
checks before the first callback. Removed logs are rejected. Duplicate identities
within a response are delivered once; conflicting duplicates fail. Events are
sorted by block, transaction index, and log index.

The endpoint hash is read before fetching logs and after checking their blocks.
A change fails the whole range with `reorg`. The watcher retains one completed
endpoint checkpoint and checks it before the next poll; its hash commits the
previous ancestry, so a deeper reorg also stops the watcher. State does not grow
with the number of delivered ranges.

`cursor` advances only after a complete validated range has been delivered. Any
RPC, decoding, or canonicality failure stops the watcher without consuming that
range. Save `watcher.cursor` inside the failure listener before the node is freed,
then restart from that cursor after resolving the error. A reorg affecting already
delivered history needs application reconciliation before restart.

## Typed requests and limits

`AbiTypegenTypedRequest.wrap(pending_request, converter)` converts successful raw
ABI values, including scalar and null results. Its converter may return a typed
value or an outcome envelope. RPC and conversion errors retain their envelopes,
and `cancel()` delegates to the source request. The adapter retains itself until
completion, so awaiting a temporary adapter is supported. Wrap pending requests
immediately; an already completed request returns `already_completed` because
native requests do not retain their final result.

Block numbers and log/transaction indices must be canonical nonnegative RPC hex
quantities that fit Godot's signed 64-bit `int`. ABI uint256 values and gas
quantities never pass through floats. `from_block` and range endpoints must also
leave room for the next cursor. Confirmations use the observed chain head; they
are not a chain finality guarantee. The implementation uses HTTP polling rather
than WebSocket subscriptions. Polling intervals and receipt deadlines use wall-clock seconds and continue
during SceneTree pause or changes to `Engine.time_scale`. The operation and its
pending HTTP request process independently of gameplay pause. The client must
remain in the scene tree.

Run the deterministic operation suite with:

```sh
python3 e2e/native/godot/run.py import
python3 e2e/native/godot/run.py suite --suite operations
```
