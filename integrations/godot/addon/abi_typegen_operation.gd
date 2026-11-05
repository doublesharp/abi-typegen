## Client-owned asynchronous receipt operation with cancellation and a deadline.
class_name AbiTypegenOperation
extends Node

const Validation = preload("abi_typegen_rpc_validation.gd")

signal finished(outcome: Dictionary)
signal completed(value)
signal failed(error: Dictionary)
signal cancelled()

var client
var tx_hash := ""
var confirmations := 1
var poll_seconds := 1.0
var timeout_seconds := 120.0
var _done := false
var _pending
var _deadline: Timer

func _exit_tree() -> void:
    if not _done:
        _finish(Validation.failure("cancelled", "operation owner left the scene tree"), false)
    _pending = null

func _finish(outcome: Dictionary, cleanup: bool = true) -> void:
    if _done:
        return
    _done = true
    if is_instance_valid(_deadline):
        _deadline.stop()
    if is_instance_valid(_pending):
        _pending.cancel()
    _pending = null
    finished.emit(outcome)
    if outcome.get("ok", false):
        completed.emit(outcome.get("value"))
    else:
        var error: Dictionary = outcome.get("error", {})
        failed.emit(error)
        if error.get("code") == "cancelled":
            cancelled.emit()
    if cleanup:
        queue_free()

## Stops the operation and its pending HTTP request without another poll.
func cancel() -> void:
    _finish(Validation.failure("cancelled", "operation cancelled"))

func _rpc(method: String, params: Array) -> Dictionary:
    if _done:
        return Validation.failure("cancelled", "operation finished")
    _pending = client.call("rpc", method, params)
    if _pending == null:
        return Validation.failure("null_request", "client returned no request")
    if _pending is Node:
        _pending.process_mode = Node.PROCESS_MODE_ALWAYS
    var outcome: Dictionary = await _pending.finished
    _pending = null
    return outcome

func _pause() -> void:
    var timer := Timer.new()
    timer.one_shot = true
    timer.ignore_time_scale = true
    timer.process_mode = Node.PROCESS_MODE_ALWAYS
    add_child(timer)
    timer.start(poll_seconds)
    await timer.timeout
    timer.queue_free()

func _start() -> void:
    if _done:
        return
    if not Validation.hash(tx_hash) or confirmations < 1 or not is_finite(poll_seconds) or poll_seconds <= 0 or not is_finite(timeout_seconds) or timeout_seconds <= 0:
        _finish(Validation.failure("invalid_operation", "valid transaction hash, positive confirmations, polling interval and timeout are required"))
        return
    _deadline = Timer.new()
    _deadline.one_shot = true
    _deadline.ignore_time_scale = true
    _deadline.process_mode = Node.PROCESS_MODE_ALWAYS
    add_child(_deadline)
    _deadline.timeout.connect(func(): _finish(Validation.failure("receipt_timeout", "transaction receipt deadline exceeded")))
    _deadline.start(timeout_seconds)
    while not _done:
        var outcome := await _rpc("eth_getTransactionReceipt", [tx_hash])
        if _done:
            return
        if not outcome.get("ok", false):
            _finish(outcome)
            return
        var receipt = outcome.get("value")
        if receipt != null:
            var block_number := _receipt_number(receipt)
            if not block_number.get("ok", false):
                _finish(block_number)
                return
            outcome = await _rpc("eth_blockNumber", [])
            if _done:
                return
            if not outcome.get("ok", false):
                _finish(outcome)
                return
            var head := Validation.quantity(outcome.get("value"))
            if not head.get("ok", false):
                _finish(head)
                return
            var number: int = block_number["value"]
            if head["value"] >= number and head["value"] - number >= confirmations - 1:
                var checked := await _canonical_receipt(receipt, number)
                if _done:
                    return
                if not checked.get("ok", false):
                    _finish(checked)
                    return
                # Receipt identity, block canonicality and depth must still
                # agree immediately before settlement, including status zero.
                outcome = await _rpc("eth_getTransactionReceipt", [tx_hash])
                if _done:
                    return
                if not outcome.get("ok", false):
                    _finish(outcome)
                    return
                var fresh = outcome.get("value")
                if fresh == null:
                    _finish(Validation.failure("reorg", "receipt disappeared during confirmation validation"))
                    return
                var fresh_number := _receipt_number(fresh)
                if not fresh_number.get("ok", false):
                    _finish(fresh_number)
                    return
                if fresh_number["value"] != number or str(fresh["blockHash"]).to_lower() != str(receipt["blockHash"]).to_lower() or fresh["status"] != receipt["status"]:
                    _finish(Validation.failure("reorg", "receipt changed during confirmation validation"))
                    return
                checked = await _canonical_receipt(fresh, number)
                if _done:
                    return
                if not checked.get("ok", false):
                    _finish(checked)
                    return
                outcome = await _rpc("eth_blockNumber", [])
                if _done:
                    return
                if not outcome.get("ok", false):
                    _finish(outcome)
                    return
                var final_head := Validation.quantity(outcome.get("value"))
                if not final_head.get("ok", false):
                    _finish(final_head)
                    return
                if final_head["value"] < number or final_head["value"] - number < confirmations - 1:
                    _finish(Validation.failure("reorg", "receipt confirmation depth changed during validation"))
                    return
                if fresh["status"] == "0x0":
                    _finish(Validation.failure("transaction_reverted", "confirmed canonical transaction receipt has reverted status"))
                    return
                _finish(Validation.success(fresh))
                return
        await _pause()

func _receipt_number(receipt) -> Dictionary:
    if typeof(receipt) != TYPE_DICTIONARY or not Validation.hash(receipt.get("blockHash")) or not Validation.hash(receipt.get("transactionHash")) or str(receipt.get("transactionHash")).to_lower() != tx_hash.to_lower():
        return Validation.failure("invalid_receipt", "receipt hash fields are missing or mismatched")
    var number := Validation.quantity(receipt.get("blockNumber"))
    if not number.get("ok", false) or not receipt.get("status") in ["0x0", "0x1"]:
        return Validation.failure("invalid_receipt", "receipt block number or status is invalid")
    return number

func _canonical_receipt(receipt: Dictionary, number: int) -> Dictionary:
    var outcome := await _rpc("eth_getBlockByNumber", [Validation.hex_quantity(number), false])
    if _done or not outcome.get("ok", false):
        return outcome
    var canonical = outcome.get("value")
    if canonical == null:
        return Validation.failure("reorg", "receipt canonical block no longer exists")
    if typeof(canonical) != TYPE_DICTIONARY or not Validation.hash(canonical.get("hash")) or not Validation.quantity(canonical.get("number")).get("ok", false) or Validation.quantity(canonical.get("number"))["value"] != number:
        return Validation.failure("invalid_block", "canonical block must have the requested number and a valid hash")
    if str(canonical["hash"]).to_lower() != str(receipt["blockHash"]).to_lower():
        return Validation.failure("reorg", "receipt block is no longer canonical")
    return Validation.success(canonical)
