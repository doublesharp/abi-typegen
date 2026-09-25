extends RefCounted

const TokenBinding = preload("res://generated/token.gd")
const TestUtil = preload("res://tests/test_util.gd")
const DevSigner = preload("res://tests/anvil_dev_signer.gd")

func _word_u64(value: int) -> PackedByteArray:
    return AbiTypegenCodec.word_from_u64(value)

func _hex_bytes(value: String) -> PackedByteArray:
    if not value.begins_with("0x") or value.length() % 2 != 0:
        return PackedByteArray()
    return value.substr(2).hex_decode()

func _await_request(request) -> Dictionary:
    if request == null:
        return {"ok": false, "error": {"code": "null_request", "message": "request is null"}}
    return await request.finished

func _receipt(tree: SceneTree, client: AbiTypegenClient, tx_hash: String) -> Dictionary:
    for _attempt in range(30):
        var outcome: Dictionary = await _await_request(client.rpc("eth_getTransactionReceipt", [tx_hash]))
        if not outcome.get("ok", false):
            return outcome
        if outcome.get("value") != null:
            return outcome
        await tree.create_timer(0.1).timeout
    return {"ok": false, "error": {"code": "receipt_timeout", "message": "transaction receipt not available"}}

func run(tree: SceneTree) -> Array[String]:
    var t = TestUtil.new()
    var rpc_url := OS.get_environment("ATG_RPC_URL")
    var token_address := OS.get_environment("ATG_TOKEN_ADDRESS")
    var delay_url := OS.get_environment("ATG_DELAY_RPC_URL")
    t.check(not rpc_url.is_empty(), "ATG_RPC_URL is required for live suite")
    t.check(AbiTypegenCodec.is_address_hex(token_address), "ATG_TOKEN_ADDRESS is required for live suite")
    t.check(not delay_url.is_empty(), "ATG_DELAY_RPC_URL is required for cancellation/destruction coverage")
    if not t.failures.is_empty():
        return t.failures

    var client := AbiTypegenClient.new()
    client.rpc_url = rpc_url
    client.timeout_seconds = 5.0
    tree.root.add_child(client)

    # Engine boundary rejects JSON numeric Variants before JSON serialization.
    var numeric_param: Dictionary = await _await_request(client.rpc("eth_chainId", [1]))
    t.check(not numeric_param.get("ok", false), "numeric JSON-RPC params are rejected")
    t.equal(numeric_param.get("error", {}).get("code"), "numeric_json_param", "numeric JSON-RPC rejection code")

    var accounts: Dictionary = await _await_request(client.rpc("eth_accounts", []))
    if not t.ok(accounts, "read Anvil accounts"):
        client.queue_free()
        return t.failures
    var account_list: Array = accounts["value"]
    t.check(not account_list.is_empty(), "Anvil exposes a development account")
    if account_list.is_empty():
        client.queue_free()
        return t.failures
    var owner := str(account_list[0])
    t.check(AbiTypegenCodec.is_address_hex(owner), "Anvil development account is an address")
    var signer = DevSigner.new(owner)
    var large_amount := _hex_bytes("0x0000000000000100000000000000000000000000000000000000000000001234")
    var larger_amount := _hex_bytes("0x0000000000000100000000000000000000000000000000000000000000001235")
    t.equal(large_amount.size(), 32, "live mint amount is a 256-bit word")

    # Asynchronous read through generated wrapper.
    var before: Dictionary = await _await_request(TokenBinding.call_balance_of(client, token_address, owner))
    t.ok(before, "async balanceOf before write")

    # Injected development signer submits mint. The signer delegates to Anvil's unlocked account.
    var mint_request = TokenBinding.send_mint(
        client, signer, token_address, owner, large_amount, {"gas": "0x493e0"}
    )
    var mint_sent: Dictionary = await _await_request(mint_request)
    if t.ok(mint_sent, "submit mint transaction"):
        var mint_hash := str(mint_sent["value"])
        var mint_receipt: Dictionary = await _receipt(tree, client, mint_hash)
        if t.ok(mint_receipt, "mint receipt"):
            var receipt: Dictionary = mint_receipt["value"]
            t.equal(receipt.get("status"), "0x1", "mint receipt status")
            var found_transfer := false
            for raw_log in receipt.get("logs", []):
                if typeof(raw_log) == TYPE_DICTIONARY and raw_log.get("topics", []).size() > 0 and str(raw_log["topics"][0]).to_lower() == TokenBinding.TRANSFER_EVENT_TOPIC:
                    var decoded := TokenBinding.decode_transfer_event(raw_log["topics"], _hex_bytes(str(raw_log.get("data", "0x"))))
                    if decoded.get("ok", false):
                        found_transfer = true
                        t.equal(decoded["value"]["amount"], large_amount, "mint Transfer amount")
                        break
            t.check(found_transfer, "mint receipt contains decodable Transfer event")

    var balance: Dictionary = await _await_request(TokenBinding.call_balance_of(client, token_address, owner))
    if t.ok(balance, "balanceOf after mint"):
        t.equal(balance["value"], large_amount, "mint preserves amount above 128 bits")

    var approval_sent: Dictionary = await _await_request(
        TokenBinding.send_approve(client, signer, token_address, owner, large_amount, {"gas": "0x493e0"})
    )
    if t.ok(approval_sent, "submit approve transaction"):
        var approval_receipt := await _receipt(tree, client, str(approval_sent["value"]))
        if t.ok(approval_receipt, "approve receipt"):
            t.equal(approval_receipt["value"].get("status"), "0x1", "approve receipt status")
            var found_approval := false
            for raw_log in approval_receipt["value"].get("logs", []):
                if typeof(raw_log) == TYPE_DICTIONARY and raw_log.get("topics", []).size() > 0 and str(raw_log["topics"][0]).to_lower() == TokenBinding.APPROVAL_EVENT_TOPIC:
                    var decoded := TokenBinding.decode_approval_event(raw_log["topics"], _hex_bytes(str(raw_log.get("data", "0x"))))
                    if decoded.get("ok", false):
                        found_approval = true
                        t.equal(decoded["value"]["amount"], large_amount, "Approval event preserves amount above 128 bits")
                        break
            t.check(found_approval, "approve receipt contains decodable Approval event")

    var allowance: Dictionary = await _await_request(TokenBinding.call_allowance(client, token_address, owner, owner))
    if t.ok(allowance, "allowance after approve"):
        t.equal(allowance["value"], large_amount, "approve preserves allowance above 128 bits")

    # JSON-RPC method error is surfaced as rpc_error, not converted to null/success.
    var rpc_error: Dictionary = await _await_request(client.rpc("abi_typegen_missing_method", []))
    t.check(not rpc_error.get("ok", false), "RPC error is reported")
    t.equal(rpc_error.get("error", {}).get("code"), "rpc_error", "RPC error code")

    # Contract revert. Assumption documented in README: sample Token.transfer is expected to
    # revert when the caller tries to transfer one more token than the minted balance.
    var transfer_encoded := TokenBinding.encode_transfer(owner, larger_amount)
    if t.ok(transfer_encoded, "encode reverting transfer"):
        var transfer_bytes: PackedByteArray = transfer_encoded["value"]
        var raw_call := {
            "to": token_address,
            "from": owner,
            "data": "0x" + transfer_bytes.hex_encode()
        }
        var reverted: Dictionary = await _await_request(client.rpc("eth_call", [raw_call, "latest"]))
        t.check(not reverted.get("ok", false), "contract revert is reported")
        t.equal(reverted.get("error", {}).get("code"), "rpc_error", "contract revert arrives as JSON-RPC error")

    # Cancellation uses an independent delayed server so the request is definitely pending.
    var delayed := AbiTypegenClient.new()
    delayed.rpc_url = delay_url
    delayed.timeout_seconds = 5.0
    tree.root.add_child(delayed)

    var wrong_id: Dictionary = await _await_request(delayed.rpc("abi_typegen_wrong_id", []))
    t.equal(wrong_id.get("error", {}).get("code"), "invalid_rpc_response", "wrong JSON-RPC id is rejected")
    var wrong_version: Dictionary = await _await_request(delayed.rpc("abi_typegen_wrong_version", []))
    t.equal(wrong_version.get("error", {}).get("code"), "invalid_rpc_response", "wrong JSON-RPC version is rejected")
    var http_error: Dictionary = await _await_request(delayed.rpc("abi_typegen_http_error_result", []))
    t.equal(http_error.get("error", {}).get("code"), "http_status", "HTTP error body with result is rejected")

    var freeing_request = delayed.rpc("abi_typegen_wrong_id", [])
    var freed_callback_count := {"count": 0}
    freeing_request.finished.connect(func(_outcome):
        freed_callback_count["count"] += 1
        freeing_request.queue_free()
    )
    await tree.create_timer(0.1).timeout
    t.equal(freed_callback_count["count"], 1, "finished listener can queue request cleanup once")

    var cancelled_request = delayed.rpc("eth_chainId", [])
    var cancelled_outcomes: Array = []
    cancelled_request.finished.connect(func(outcome): cancelled_outcomes.append(outcome))
    cancelled_request.cancel()
    await tree.process_frame
    t.equal(cancelled_outcomes.size(), 1, "cancel emits exactly one finished outcome")
    if cancelled_outcomes.size() == 1:
        t.equal(cancelled_outcomes[0].get("error", {}).get("code"), "cancelled", "cancellation error code")

    # Explicit destruction while HTTPRequest is pending must not invoke the freed request.
    var destroyed_request = delayed.rpc("eth_chainId", [])
    var destroyed_callback := {"called": false}
    destroyed_request.finished.connect(func(_outcome): destroyed_callback["called"] = true)
    destroyed_request.free()
    await tree.create_timer(2.5).timeout
    t.check(not destroyed_callback["called"], "freed pending request emits no callback")

    await tree.process_frame
    t.equal(client.get_child_count(), 0, "completed client requests release child nodes")
    t.equal(delayed.get_child_count(), 0, "completed and canceled delayed requests release child nodes")

    delayed.queue_free()
    client.queue_free()
    return t.failures
