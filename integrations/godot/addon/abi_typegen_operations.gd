## Shared HTTP JSON-RPC operations used by generated contract bindings.
class_name AbiTypegenOperations
extends RefCounted

const Operation = preload("abi_typegen_operation.gd")
const Watcher = preload("abi_typegen_event_watcher.gd")
const TypedRequest = preload("abi_typegen_typed_request.gd")
const Validation = preload("abi_typegen_rpc_validation.gd")

## Simulates a transaction at the requested block without signing or submitting it.
static func simulate(client, transaction: Dictionary, block: String = "latest"):
    if not Validation.block(block):
        return client.failed_request("invalid_block", "simulation block must be a known tag or canonical signed-64-bit quantity")
    return TypedRequest.wrap(client.call("rpc", "eth_call", [transaction.duplicate(true), block]), _simulation_result)

static func _simulation_result(value) -> Dictionary:
    if not Validation.is_hex(value) or value.length() % 2 != 0:
        return Validation.failure("invalid_simulation", "eth_call result must contain hexadecimal bytes")
    return Validation.success(value)

## Estimates gas, preserving the exact hexadecimal uint256 quantity.
static func estimate_gas(client, transaction: Dictionary):
    return TypedRequest.wrap(client.call("rpc", "eth_estimateGas", [transaction.duplicate(true)]), _gas_result)

static func _gas_result(value) -> Dictionary:
    if not Validation.is_quantity(value):
        return Validation.failure("invalid_gas_estimate", "eth_estimateGas result must be an exact hexadecimal quantity")
    return Validation.success(value)

## Waits for a successful canonical receipt with the configured confirmation depth.
static func wait_for_receipt(client, tx_hash: String, confirmations: int = 1, poll_seconds: float = 1.0, timeout_seconds: float = 120.0) -> AbiTypegenOperation:
    var operation = Operation.new()
    operation.process_mode = Node.PROCESS_MODE_ALWAYS
    operation.client = client
    operation.tx_hash = tx_hash
    operation.confirmations = confirmations
    operation.poll_seconds = poll_seconds
    operation.timeout_seconds = timeout_seconds
    client.add_child(operation)
    operation.call_deferred("_start")
    return operation

## Polls finalized ranges, decoding a complete batch before emitting any event.
static func watch_events(client, filter: Dictionary, decoder: Callable, from_block: int, confirmations: int = 1, poll_seconds: float = 1.0, block_batch_size: int = 100) -> AbiTypegenEventWatcher:
    var watcher = Watcher.new()
    watcher.process_mode = Node.PROCESS_MODE_ALWAYS
    watcher.client = client
    watcher.filter = filter.duplicate(true)
    watcher.decoder = decoder
    watcher.cursor = from_block
    watcher.confirmations = confirmations
    watcher.poll_seconds = poll_seconds
    watcher.block_batch_size = block_batch_size
    client.add_child(watcher)
    watcher.call_deferred("_start")
    return watcher
