class_name AnvilDevelopmentSigner
extends RefCounted

# Test-only injected signer adapter. It delegates signing to Anvil's unlocked
# development account via eth_sendTransaction and never accepts a private key.
var from_address: String

func _init(address: String) -> void:
    from_address = address

func send_transaction(client: AbiTypegenClient, tx: Dictionary):
    if client == null:
        return null
    if not AbiTypegenCodec.is_address_hex(from_address):
        return client.failed_request("invalid_signer_address", "development signer address is invalid")
    var payload := tx.duplicate(true)
    payload["from"] = from_address
    return client.rpc("eth_sendTransaction", [payload])
