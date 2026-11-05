extends RefCounted

const TestUtil = preload("res://tests/test_util.gd")
const HASH_A = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
const HASH_B = "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
const TX = "0xcccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"

class FakeRequest extends Node:
    signal finished(outcome: Dictionary)
    var response: Dictionary
    var done := false
    var cancellation_counter := {"count": 0}
    func deliver() -> void:
        if done:
            return
        done = true
        finished.emit(response)
        queue_free()
    func _process(_delta: float) -> void:
        if response.get("process_only", false):
            deliver()
    func is_done() -> bool:
        return done
    func cancel() -> void:
        if not done:
            cancellation_counter["count"] += 1
        response = {"ok": false, "error": {"code": "cancelled"}}
        deliver()

class FakeClient extends RefCounted:
    var host := Node.new()
    var responses: Array = []
    var calls: Array = []
    var cancellation_counter := {"count": 0}
    func rpc(method: String, params: Array):
        calls.append([method, params.duplicate(true)])
        var request := FakeRequest.new()
        host.add_child(request)
        request.cancellation_counter = cancellation_counter
        if responses.is_empty():
            request.response = {"ok": false, "error": {"code": "unexpected_request", "message": method}}
        else:
            var next: Dictionary = responses.pop_front()
            request.response = next
        if not request.response.get("hold", false) and not request.response.get("process_only", false):
            request.call_deferred("deliver")
        return request
    func failed_request(code: String, message: String):
        var request := FakeRequest.new()
        host.add_child(request)
        request.response = {"ok": false, "error": {"code": code, "message": message}}
        request.call_deferred("deliver")
        return request
    func add_child(child: Node) -> void:
        host.add_child(child)
    func queue_free() -> void:
        host.queue_free()

func success(value) -> Dictionary:
    return {"ok": true, "value": value}

func receipt(status = "0x1") -> Dictionary:
    return {"transactionHash": TX, "blockNumber": "0x5", "blockHash": HASH_A, "status": status}

func log_value(index = "0x0") -> Dictionary:
    return {"blockNumber": "0x5", "blockHash": HASH_A, "transactionHash": TX, "transactionIndex": "0x0", "logIndex": index, "removed": false, "address": "0x" + "11".repeat(20), "topics": [], "data": "0x"}

func decoder(_topics: Array, data: PackedByteArray) -> Dictionary:
    if data.hex_encode() == "ff":
        return {"ok": false, "error": {"code": "decode_failed"}}
    return success("0x0")

func canonical(number = "0x5", block_hash = HASH_A) -> Dictionary:
    return success({"hash": block_hash, "number": number})

func receipt_script(status = "0x1", head = "0x5") -> Array:
    return [success(receipt(status)), success(head), canonical(), success(receipt(status)), canonical(), success(head)]

func watch_script(logs: Array, head = "0x5", endpoint = "0x5", block_hash = HASH_A) -> Array:
    return [success(head), canonical(endpoint, block_hash), success(logs), canonical(endpoint, block_hash)]

func make_client(tree: SceneTree, responses: Array):
    var client := FakeClient.new()
    client.responses = responses
    tree.root.add_child(client.host)
    return client

func run(tree: SceneTree) -> Array[String]:
    var t = TestUtil.new()
    var path := "res://addons/abi_typegen/abi_typegen_operations.gd"
    t.check(FileAccess.file_exists(path), "shared Godot operations addon exists")
    if not FileAccess.file_exists(path):
        return t.failures
    var ops = load(path)
    var typed = load("res://addons/abi_typegen/abi_typegen_typed_request.gd")
    var client = make_client(tree, [success("0x1234"), success("0x5208")])
    var simulation: Dictionary = await ops.simulate(client, {"to": "0x" + "11".repeat(20)}, "pending").finished
    t.equal(simulation, success("0x1234"), "simulation retains hex return bytes")
    var gas: Dictionary = await ops.estimate_gas(client, {"value": "0x100000000000000000000"}).finished
    t.equal(gas, success("0x5208"), "gas remains exact hex quantity")
    t.equal(client.calls[0][0], "eth_call", "simulation RPC method")
    t.equal(client.calls[0][1][1], "pending", "simulation block passthrough")
    t.equal(client.calls[1][0], "eth_estimateGas", "estimation RPC method")
    client.queue_free()

    client = make_client(tree, [success("0x7")])
    var adapter = typed.wrap(client.rpc("typed", []), func(value): return success({"decoded": value}))
    var converted: Dictionary = await adapter.finished
    t.equal(converted, success({"decoded": "0x7"}), "typed adapter converts successful value")
    client.queue_free()

    client = make_client(tree, [success(null), success(receipt()), success("0x5")] + receipt_script("0x1", "0x6"))
    var confirmed: Dictionary = await ops.wait_for_receipt(client, TX, 2, 0.001, 2.0).finished
    t.ok(confirmed, "receipt waits for configured confirmations")
    t.equal(client.calls.size(), 9, "receipt polls until depth and rechecks receipt, canonical block, and final head")
    var final_queries: Array = []
    for call in client.calls.slice(3):
        final_queries.append(call[0])
    t.equal(final_queries, ["eth_getTransactionReceipt", "eth_blockNumber", "eth_getBlockByNumber", "eth_getTransactionReceipt", "eth_getBlockByNumber", "eth_blockNumber"], "receipt settlement query order")
    client.queue_free()

    client = make_client(tree, receipt_script("0x0"))
    var reverted: Dictionary = await ops.wait_for_receipt(client, TX, 1, 0.001, 1.0).finished
    t.equal(reverted.get("error", {}).get("code"), "transaction_reverted", "reverted receipt fails")
    client.queue_free()

    client = make_client(tree, [success(receipt()), success("0x6"), success({"hash": HASH_B, "number": "0x5"})])
    var reorg: Dictionary = await ops.wait_for_receipt(client, TX, 1, 0.001, 1.0).finished
    t.equal(reorg.get("error", {}).get("code"), "reorg", "receipt rejects noncanonical block")
    client.queue_free()

    client = make_client(tree, [])
    var operation = ops.wait_for_receipt(client, TX, 1, 0.001, 1.0)
    var outcomes: Array = []
    operation.finished.connect(func(value): outcomes.append(value))
    operation.cancel()
    await tree.process_frame
    t.equal(outcomes.size(), 1, "receipt cancel completes once before deferred start")
    t.equal(client.calls.size(), 0, "cancelled receipt starts no RPC")
    client.queue_free()

    client = make_client(tree, [success(null)])
    var timeout: Dictionary = await ops.wait_for_receipt(client, TX, 1, 0.1, 0.01).finished
    t.equal(timeout.get("error", {}).get("code"), "receipt_timeout", "receipt timeout covers polling interval")
    client.queue_free()

    # A complete validated range is delivered once despite duplicate RPC logs.
    client = make_client(tree, watch_script([log_value(), log_value()]))
    var watcher = ops.watch_events(client, {}, decoder, 5, 1, 0.001, 100)
    var events: Array = []
    watcher.event_received.connect(func(value, _log): events.append(value))
    for _frame in range(15):
        await tree.process_frame
        if watcher.cursor == 6:
            break
    t.equal(events, ["0x0"], "watcher suppresses duplicate logs")
    t.equal(watcher.cursor, 6, "validated range advances cursor")
    t.equal(client.calls[2][1][0].get("fromBlock"), "0x5", "watcher sends explicit start cursor")
    watcher.cancel()
    client.queue_free()

    # Decode failures are atomic: neither earlier valid logs nor cursor changes escape.
    var invalid := log_value("0x1")
    invalid["data"] = "0xff"
    client = make_client(tree, watch_script([log_value(), invalid]))
    watcher = ops.watch_events(client, {}, decoder, 5, 1, 0.001, 100)
    events = []
    watcher.event_received.connect(func(value, _log): events.append(value))
    var decode_failure: Dictionary = await watcher.finished
    t.equal(decode_failure.get("error", {}).get("code"), "decode_failed", "watcher propagates decoder failure")
    t.equal(events.size(), 0, "whole batch validates before callbacks")
    t.equal(watcher.cursor, 5, "decode failure retains cursor")
    client.queue_free()

    var removed := log_value()
    removed["removed"] = true
    client = make_client(tree, watch_script([removed]))
    watcher = ops.watch_events(client, {}, decoder, 5, 1, 0.001, 100)
    var removed_failure: Dictionary = await watcher.finished
    t.equal(removed_failure.get("error", {}).get("code"), "removed_log", "watcher rejects removed logs")
    t.equal(watcher.cursor, 5, "removed logs retain cursor")
    client.queue_free()

    client = make_client(tree, watch_script([log_value()]) + [canonical("0x5", HASH_B)])
    watcher = ops.watch_events(client, {}, decoder, 5, 1, 0.001, 100)
    var history_reorg: Dictionary = await watcher.finished
    t.equal(history_reorg.get("error", {}).get("code"), "reorg", "watcher detects changed delivered history")
    t.equal(watcher.cursor, 6, "history reorg fails without reskipping delivered cursor")
    client.queue_free()
    # Pending-request cancellation also cancels the active transport.
    client = make_client(tree, [{"ok": true, "value": null, "hold": true}])
    operation = ops.wait_for_receipt(client, TX, 1, 0.001, 1.0)
    outcomes = []
    operation.finished.connect(func(value): outcomes.append(value))
    await tree.process_frame
    operation.cancel()
    await tree.create_timer(0.03).timeout
    t.equal(outcomes.size(), 1, "pending receipt cancel emits exactly once")
    t.equal(client.cancellation_counter["count"], 1, "pending receipt cancel stops transport")
    t.equal(client.calls.size(), 1, "cancelled receipt never repolls")
    t.equal(client.host.get_child_count(), 0, "cancelled receipt releases operation and request children")
    client.queue_free()

    client = make_client(tree, [{"ok": true, "value": null, "hold": true}])
    var pending_timeout: Dictionary = await ops.wait_for_receipt(client, TX, 1, 0.001, 0.01).finished
    t.equal(pending_timeout.get("error", {}).get("code"), "receipt_timeout", "receipt deadline covers pending transport")
    t.equal(client.cancellation_counter["count"], 1, "receipt deadline cancels pending transport")
    client.queue_free()

    client = make_client(tree, [{"ok": true, "value": null, "hold": true}])
    operation = ops.wait_for_receipt(client, TX, 1, 0.001, 1.0)
    outcomes = []
    operation.finished.connect(func(value): outcomes.append(value))
    await tree.process_frame
    client.queue_free()
    await tree.create_timer(0.03).timeout
    t.equal(outcomes.size(), 1, "destroyed client settles operation waiters once")
    if outcomes.size() == 1:
        t.equal(outcomes[0].get("error", {}).get("code"), "cancelled", "destroyed client settles operation as cancelled")

    # Adapter failures bypass conversion; cancellation retains the source error.
    var conversions := {"count": 0}
    client = make_client(tree, [{"ok": false, "error": {"code": "rpc_error", "message": "failed"}}])
    adapter = typed.wrap(client.rpc("typed", []), func(value):
        conversions["count"] += 1
        return value
    )
    var typed_failure: Dictionary = await adapter.finished
    t.equal(typed_failure.get("error", {}).get("code"), "rpc_error", "typed adapter preserves RPC failures")
    t.equal(conversions["count"], 0, "typed adapter never converts failed RPC values")
    client.queue_free()

    client = make_client(tree, [{"ok": true, "value": null, "hold": true}])
    adapter = typed.wrap(client.rpc("typed", []), func(value): return value)
    outcomes = []
    adapter.finished.connect(func(value): outcomes.append(value))
    adapter.cancel()
    t.equal(outcomes.size(), 1, "typed cancel emits once")
    t.equal(outcomes[0].get("error", {}).get("code"), "cancelled", "typed cancel preserves cancellation code")
    client.queue_free()

    client = make_client(tree, [success("0x2")])
    adapter = typed.wrap(client.rpc("typed", []), func(_value): return {"ok": false, "error": {"code": "conversion_failed"}})
    var conversion_failure: Dictionary = await adapter.finished
    t.equal(conversion_failure.get("error", {}).get("code"), "conversion_failed", "typed conversion errors propagate")
    client.queue_free()

    for status in [null, "0x2", true, 1.0]:
        client = make_client(tree, [success(receipt(status))])
        var invalid_receipt: Dictionary = await ops.wait_for_receipt(client, TX, 1, 0.001, 1.0).finished
        t.equal(invalid_receipt.get("error", {}).get("code"), "invalid_receipt", "invalid receipt status is rejected")
        client.queue_free()

    # Failed RPC, malformed counters and noncanonical blocks do not consume ranges.
    var broken_rpc := {"ok": false, "error": {"code": "rpc_error"}}
    for responses in [
        [success("0x5"), canonical(), broken_rpc],
        [success("0x8000000000000000")],
        [success("0x05")],
        [success(5.0)],
        watch_script([log_value()], "0x5", "0x5", HASH_B)
    ]:
        client = make_client(tree, responses)
        watcher = ops.watch_events(client, {}, decoder, 5, 1, 0.001, 100)
        events = []
        watcher.event_received.connect(func(value, _log): events.append(value))
        var invalid_range: Dictionary = await watcher.finished
        t.check(not invalid_range.get("ok", false), "invalid RPC range fails")
        t.equal(watcher.cursor, 5, "invalid RPC range retains cursor")
        t.equal(events.size(), 0, "invalid RPC range delivers no callbacks")
        client.queue_free()

    var wrong_range := log_value()
    wrong_range["blockNumber"] = "0x4"
    var conflicting := log_value()
    conflicting["data"] = "0x12"
    for logs in [[wrong_range], [log_value(), conflicting]]:
        client = make_client(tree, watch_script(logs))
        watcher = ops.watch_events(client, {}, decoder, 5, 1, 0.001, 100)
        var invalid_logs: Dictionary = await watcher.finished
        t.equal(invalid_logs.get("error", {}).get("code"), "invalid_logs", "out-of-range or conflicting duplicate logs fail")
        t.equal(watcher.cursor, 5, "invalid logs retain cursor")
        client.queue_free()

    client = make_client(tree, watch_script([], "0x8", "0x6"))
    watcher = ops.watch_events(client, {}, decoder, 5, 2, 1.0, 2)
    for _frame in range(15):
        await tree.process_frame
        if watcher.cursor == 7:
            break
    t.equal(client.calls[2][1][0].get("toBlock"), "0x6", "watcher limits confirmed ranges by batch size")
    t.equal(watcher.cursor, 7, "empty canonical range advances cursor")
    watcher.cancel()
    await tree.create_timer(0.03).timeout
    t.equal(client.calls.size(), 4, "cancelled sleeping watcher never repolls")
    client.queue_free()

    client = make_client(tree, [success(receipt()), success("0x5"), success({"hash": HASH_A, "number": "0x4"})])
    var wrong_canonical_number: Dictionary = await ops.wait_for_receipt(client, TX, 1, 0.001, 1.0).finished
    t.equal(wrong_canonical_number.get("error", {}).get("code"), "invalid_block", "receipt verifies canonical block number")
    client.queue_free()

    client = make_client(tree, [success("0x5"), canonical("0x4"), success([log_value()])])
    watcher = ops.watch_events(client, {}, decoder, 5, 1, 0.001, 100)
    var wrong_log_number: Dictionary = await watcher.finished
    t.equal(wrong_log_number.get("error", {}).get("code"), "invalid_block", "watcher verifies canonical block number")
    client.queue_free()

    var already_done := FakeRequest.new()
    tree.root.add_child(already_done)
    already_done.response = success("0x1")
    already_done.deliver()
    adapter = typed.wrap(already_done, func(value): return value)
    var late_adapter: Dictionary = await adapter.finished
    t.equal(late_adapter.get("error", {}).get("code"), "already_completed", "typed adapter rejects already completed source without hanging")

    # Tip rollback can lose depth while preserving the receipt's block hash.
    var deep_receipt := receipt()
    deep_receipt["blockNumber"] = "0x63"
    client = make_client(tree, [
        success(deep_receipt), success("0x64"), canonical("0x63"),
        success(deep_receipt), canonical("0x63"), success("0x63")
    ])
    var lost_depth: Dictionary = await ops.wait_for_receipt(client, TX, 2, 0.001, 1.0).finished
    t.equal(lost_depth.get("error", {}).get("code"), "reorg", "receipt loses confirmations during final canonical validation")
    client.queue_free()

    var failed_deep_receipt := deep_receipt.duplicate(true)
    failed_deep_receipt["status"] = "0x0"
    client = make_client(tree, [
        success(failed_deep_receipt), success("0x64"), canonical("0x63"),
        success(failed_deep_receipt), canonical("0x63"), success("0x63")
    ])
    var failed_lost_depth: Dictionary = await ops.wait_for_receipt(client, TX, 2, 0.001, 1.0).finished
    t.equal(failed_lost_depth.get("error", {}).get("code"), "reorg", "reverted receipt also rechecks final confirmation depth")
    client.queue_free()

    var changed_receipt := receipt()
    changed_receipt["blockHash"] = HASH_B
    for fresh_receipt in [null, changed_receipt]:
        client = make_client(tree, [success(receipt()), success("0x5"), canonical(), success(fresh_receipt)])
        var changed_confirmation: Dictionary = await ops.wait_for_receipt(client, TX, 1, 0.001, 1.0).finished
        t.equal(changed_confirmation.get("error", {}).get("code"), "reorg", "receipt disappearance or movement fails final validation")
        client.queue_free()

    client = make_client(tree, [success(receipt()), success("0x5"), canonical(), success(receipt()), canonical("0x5", HASH_B)])
    var late_canonical_change: Dictionary = await ops.wait_for_receipt(client, TX, 1, 0.001, 1.0).finished
    t.equal(late_canonical_change.get("error", {}).get("code"), "reorg", "fresh receipt must still have a canonical block")
    client.queue_free()

    # Receipt status is terminal only after depth and canonical validation.
    client = make_client(tree, [success(receipt("0x0")), success("0x5")] + receipt_script("0x1", "0x6"))
    var remined: Dictionary = await ops.wait_for_receipt(client, TX, 2, 0.001, 1.0).finished
    t.ok(remined, "unconfirmed reverted receipt can be remined successfully")
    client.queue_free()

    client = make_client(tree, [success(receipt("0x0")), success("0x5"), canonical("0x5", HASH_B)])
    var orphan_revert: Dictionary = await ops.wait_for_receipt(client, TX, 1, 0.001, 1.0).finished
    t.equal(orphan_revert.get("error", {}).get("code"), "reorg", "orphaned reverted receipt reports reorg")
    client.queue_free()

    # An earlier log can become orphaned while another block is validated.
    var earlier := log_value()
    earlier["blockNumber"] = "0x4"
    client = make_client(tree, [success("0x5"), canonical(), success([earlier]), canonical("0x4"), canonical("0x5", HASH_B)])
    watcher = ops.watch_events(client, {}, decoder, 4, 1, 0.001, 100)
    events = []
    watcher.event_received.connect(func(value, _log): events.append(value))
    var batch_race: Dictionary = await watcher.finished
    t.equal(batch_race.get("error", {}).get("code"), "reorg", "endpoint change during batch validation fails")
    t.equal(events.size(), 0, "reorg during canonical batch checks delivers no events")
    t.equal(watcher.cursor, 4, "reorg during canonical batch checks retains cursor")
    client.queue_free()

    client = make_client(tree, [])
    var invalid_simulation: Dictionary = await ops.simulate(client, {}, "0x05").finished
    t.equal(invalid_simulation.get("error", {}).get("code"), "invalid_block", "simulation rejects malformed block selector")
    t.equal(client.calls.size(), 0, "invalid simulation starts no RPC")
    client.queue_free()

    client = make_client(tree, [success(receipt()), success("0x5"), success(null)])
    var missing_receipt_block: Dictionary = await ops.wait_for_receipt(client, TX, 1, 0.001, 1.0).finished
    t.equal(missing_receipt_block.get("error", {}).get("code"), "reorg", "missing canonical receipt block reports reorg")
    client.queue_free()

    client = make_client(tree, watch_script([log_value()]) + [success(null)])
    watcher = ops.watch_events(client, {}, decoder, 5, 1, 0.001, 100)
    var missing_checkpoint: Dictionary = await watcher.finished
    t.equal(missing_checkpoint.get("error", {}).get("code"), "reorg", "missing delivered checkpoint reports reorg")
    client.queue_free()

    # Chain operations must continue through paused menus and slowed gameplay.
    var saved_pause := tree.paused
    var saved_scale := Engine.time_scale
    client = make_client(tree, [{"ok": true, "value": null, "hold": true}])
    client.host.process_mode = Node.PROCESS_MODE_PAUSABLE
    operation = ops.wait_for_receipt(client, TX, 1, 0.001, 0.02)
    outcomes = []
    operation.finished.connect(func(value): outcomes.append(value))
    tree.paused = true
    Engine.time_scale = 0.1
    await tree.create_timer(0.08, true, false, true).timeout
    t.equal(outcomes.size(), 1, "receipt deadline uses wallclock during paused slowed gameplay")
    if outcomes.size() == 1:
        t.equal(outcomes[0].get("error", {}).get("code"), "receipt_timeout", "paused receipt deadline reports timeout")
    tree.paused = saved_pause
    Engine.time_scale = saved_scale
    if is_instance_valid(operation):
        operation.cancel()
    client.queue_free()

    var paused_watch_responses: Array = [success("0x4")] + watch_script([])
    for response in paused_watch_responses:
        response["process_only"] = true
    paused_watch_responses.append({"ok": true, "value": null, "hold": true})
    client = make_client(tree, paused_watch_responses)
    client.host.process_mode = Node.PROCESS_MODE_PAUSABLE
    watcher = ops.watch_events(client, {}, decoder, 5, 1, 0.01, 100)
    outcomes = []
    watcher.finished.connect(func(value): outcomes.append(value))
    tree.paused = true
    Engine.time_scale = 0.1
    await tree.create_timer(0.08, true, false, true).timeout
    t.check(client.calls.size() >= 5, "watcher repeats wallclock polling during paused slowed gameplay")
    if is_instance_valid(watcher):
        t.equal(watcher.cursor, 6, "paused watcher delivers its next confirmed range")
        watcher.cancel()
    tree.paused = saved_pause
    Engine.time_scale = saved_scale
    client.queue_free()

    for bad_source in [null, RefCounted.new(), Node.new()]:
        adapter = typed.wrap(bad_source, func(value): return value)
        var invalid_adapter: Dictionary = await adapter.finished
        t.equal(invalid_adapter.get("error", {}).get("code"), "invalid_typed_request", "malformed typed source fails before signal access")
        if bad_source is Node:
            bad_source.free()

    var short_topics := log_value()
    short_topics["topics"] = [HASH_A]
    client = make_client(tree, watch_script([short_topics]))
    watcher = ops.watch_events(client, {"topics": [HASH_A, null]}, decoder, 5, 1, 0.001, 100)
    events = []
    watcher.event_received.connect(func(value, _log): events.append(value))
    var wildcard_shape: Dictionary = await watcher.finished
    t.equal(wildcard_shape.get("error", {}).get("code"), "invalid_logs", "wildcard topic still requires its log position")
    t.equal(events.size(), 0, "short topic list emits no event")
    t.equal(watcher.cursor, 5, "short topic list preserves cursor")
    client.queue_free()

    for invalid_filter in [{"fromBlock": "0x0"}, {"blockHash": HASH_A}, {"topics": ["0x01"]}, {"address": "0x12"}]:
        client = make_client(tree, [])
        watcher = ops.watch_events(client, invalid_filter, decoder, 5, 1, 0.001, 100)
        var invalid_watcher: Dictionary = await watcher.finished
        t.equal(invalid_watcher.get("error", {}).get("code"), "invalid_watcher", "invalid event filter rejected before RPC")
        t.equal(client.calls.size(), 0, "invalid watcher starts no RPC")
        client.queue_free()
    await tree.process_frame
    return t.failures
