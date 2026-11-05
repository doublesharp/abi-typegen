## Polls explicit confirmed block ranges and fails if delivered history reorganizes.
class_name AbiTypegenEventWatcher
extends "abi_typegen_operation.gd"

signal event_received(value, log: Dictionary)

## Next undelivered block. It changes only after a complete successful range.
var cursor := 0
var filter: Dictionary = {}
var decoder: Callable
var block_batch_size := 100
var _checkpoint_number := -1
var _checkpoint_hash := ""

func _valid_filter() -> bool:
    for key in filter:
        if not key in ["address", "topics"]:
            return false
    if filter.has("address"):
        var addresses = filter["address"] if typeof(filter["address"]) == TYPE_ARRAY else [filter["address"]]
        if addresses.is_empty():
            return false
        for address in addresses:
            if not Validation.is_hex(address, 20):
                return false
    if filter.has("topics"):
        if typeof(filter["topics"]) != TYPE_ARRAY or filter["topics"].size() > 4:
            return false
        for topic in filter["topics"]:
            if topic == null:
                continue
            var choices = topic if typeof(topic) == TYPE_ARRAY else [topic]
            for choice in choices:
                if not Validation.hash(choice):
                    return false
    return true

func _matches_filter(log: Dictionary) -> bool:
    if filter.has("address"):
        var addresses = filter["address"] if typeof(filter["address"]) == TYPE_ARRAY else [filter["address"]]
        var match_address := false
        for address in addresses:
            if str(address).to_lower() == str(log["address"]).to_lower():
                match_address = true
        if not match_address:
            return false
    var expected: Array = filter.get("topics", [])
    var actual: Array = log["topics"]
    for index in range(expected.size()):
        if index >= actual.size():
            return false
        var topic = expected[index]
        if topic == null:
            continue
        var choices = topic if typeof(topic) == TYPE_ARRAY else [topic]
        var match_topic := false
        for choice in choices:
            if str(choice).to_lower() == str(actual[index]).to_lower():
                match_topic = true
        if not match_topic:
            return false
    return true

func _canonical(number: int) -> Dictionary:
    var outcome := await _rpc("eth_getBlockByNumber", [Validation.hex_quantity(number), false])
    if _done or not outcome.get("ok", false):
        return outcome
    var block = outcome.get("value")
    if block == null:
        return Validation.failure("reorg", "requested canonical block no longer exists")
    if typeof(block) != TYPE_DICTIONARY or not Validation.hash(block.get("hash")) or not Validation.quantity(block.get("number")).get("ok", false) or Validation.quantity(block.get("number"))["value"] != number:
        return Validation.failure("invalid_block", "canonical block is missing a valid hash")
    return Validation.success(str(block["hash"]).to_lower())

func _start() -> void:
    if _done:
        return
    if cursor < 0 or cursor == 9223372036854775807 or confirmations < 1 or block_batch_size < 1 or not is_finite(poll_seconds) or poll_seconds <= 0 or not decoder.is_valid() or not _valid_filter():
        _finish(Validation.failure("invalid_watcher", "valid filter, decoder, nonnegative cursor and positive polling settings are required"))
        return
    while not _done:
        # A block hash commits its ancestry, so one endpoint detects changes
        # anywhere in previously delivered history without unbounded storage.
        if _checkpoint_number >= 0:
            var checked := await _canonical(_checkpoint_number)
            if _done:
                return
            if not checked.get("ok", false):
                _finish(checked)
                return
            if checked["value"] != _checkpoint_hash:
                _finish(Validation.failure("reorg", "previously delivered block history changed"))
                return
        var outcome := await _rpc("eth_blockNumber", [])
        if _done:
            return
        if not outcome.get("ok", false):
            _finish(outcome)
            return
        var head := Validation.quantity(outcome.get("value"))
        if not head.get("ok", false):
            _finish(head)
            return
        var end: int = head["value"] - (confirmations - 1)
        if end < cursor:
            await _pause()
            continue
        end = cursor + mini(end - cursor, block_batch_size - 1)
        if end == 9223372036854775807:
            _finish(Validation.failure("invalid_quantity", "range endpoint cannot advance within signed 64-bit counters"))
            return
        var before := await _canonical(end)
        if _done:
            return
        if not before.get("ok", false):
            _finish(before)
            return
        var rpc_filter := filter.duplicate(true)
        rpc_filter["fromBlock"] = Validation.hex_quantity(cursor)
        rpc_filter["toBlock"] = Validation.hex_quantity(end)
        outcome = await _rpc("eth_getLogs", [rpc_filter])
        if _done:
            return
        if not outcome.get("ok", false):
            _finish(outcome)
            return
        if typeof(outcome.get("value")) != TYPE_ARRAY:
            _finish(Validation.failure("invalid_logs", "eth_getLogs must return an array"))
            return
        var batch: Array = []
        var hashes: Dictionary = {}
        var identities: Dictionary = {}
        for raw_log in outcome["value"]:
            var validated := _validate_log(raw_log, cursor, end)
            if not validated.get("ok", false):
                _finish(validated)
                return
            var entry: Dictionary = validated["value"]
            var number: int = entry["block"]
            var block_hash: String = entry["log"]["blockHash"].to_lower()
            if hashes.has(number) and hashes[number] != block_hash:
                _finish(Validation.failure("reorg", "RPC logs contain conflicting block hashes"))
                return
            hashes[number] = block_hash
            var identity: String = entry["identity"]
            if identities.has(identity):
                if identities[identity] != entry["log"]:
                    _finish(Validation.failure("invalid_logs", "duplicate log identity contains conflicting content"))
                    return
                continue
            identities[identity] = entry["log"]
            var decoded = decoder.call(entry["log"]["topics"].duplicate(true), str(entry["log"]["data"]).substr(2).hex_decode())
            if _done:
                return
            if typeof(decoded) != TYPE_DICTIONARY or typeof(decoded.get("ok")) != TYPE_BOOL or (decoded.get("ok", false) and not decoded.has("value")) or (not decoded.get("ok", false) and typeof(decoded.get("error")) != TYPE_DICTIONARY):
                _finish(Validation.failure("invalid_decoder_result", "decoder must return a success or failure outcome"))
                return
            if not decoded["ok"]:
                _finish(decoded)
                return
            entry["value"] = decoded.get("value")
            batch.append(entry)
        for number in hashes:
            var checked := before if number == end else await _canonical(number)
            if _done:
                return
            if not checked.get("ok", false):
                _finish(checked)
                return
            if hashes[number] != "" and hashes[number] != checked["value"]:
                _finish(Validation.failure("reorg", "log block is no longer canonical"))
                return
            hashes[number] = checked["value"]
        var after := await _canonical(end)
        if _done:
            return
        if not after.get("ok", false):
            _finish(after)
            return
        if before["value"] != after["value"]:
            _finish(Validation.failure("reorg", "range endpoint changed while validating logs"))
            return
        batch.sort_custom(func(a, b):
            if a["block"] != b["block"]:
                return a["block"] < b["block"]
            if a["transaction"] != b["transaction"]:
                return a["transaction"] < b["transaction"]
            return a["index"] < b["index"]
        )
        for entry in batch:
            if _done:
                return
            event_received.emit(entry["value"], entry["log"].duplicate(true))
        if _done:
            return
        _checkpoint_number = end
        _checkpoint_hash = after["value"]
        cursor = end + 1
        await _pause()

func _validate_log(raw_log, first: int, last: int) -> Dictionary:
    if typeof(raw_log) != TYPE_DICTIONARY:
        return Validation.failure("invalid_logs", "each RPC log must be an object")
    var log: Dictionary = raw_log.duplicate(true)
    if typeof(log.get("removed")) != TYPE_BOOL:
        return Validation.failure("invalid_logs", "log must contain a boolean removed field")
    if log["removed"]:
        return Validation.failure("removed_log", "removed log cannot be delivered")
    if not Validation.hash(log.get("blockHash")) or not Validation.hash(log.get("transactionHash")) or not Validation.is_hex(log.get("address"), 20):
        return Validation.failure("invalid_logs", "log hash or address fields are invalid")
    if typeof(log.get("topics")) != TYPE_ARRAY or log["topics"].size() > 4:
        return Validation.failure("invalid_logs", "log topics must be an array of at most four hashes")
    for topic in log["topics"]:
        if not Validation.hash(topic):
            return Validation.failure("invalid_logs", "log topic is not a 32-byte hash")
    if not Validation.is_hex(log.get("data")) or str(log["data"]).length() % 2 != 0:
        return Validation.failure("invalid_logs", "log data must contain complete hexadecimal bytes")
    var block_number := Validation.quantity(log.get("blockNumber"))
    var transaction := Validation.quantity(log.get("transactionIndex"))
    var index := Validation.quantity(log.get("logIndex"))
    if not block_number.get("ok", false) or not transaction.get("ok", false) or not index.get("ok", false):
        return Validation.failure("invalid_logs", "log counters must be canonical signed-64-bit quantities")
    if block_number["value"] < first or block_number["value"] > last or not _matches_filter(log):
        return Validation.failure("invalid_logs", "log does not match the requested range or filter")
    var identity: String = str(log["blockHash"]).to_lower() + ":" + str(log["transactionHash"]).to_lower() + ":" + str(index["value"])
    return Validation.success({"log": log, "block": block_number["value"], "transaction": transaction["value"], "index": index["value"], "identity": identity})
