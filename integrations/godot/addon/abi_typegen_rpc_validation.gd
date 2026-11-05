## Strict JSON-RPC quantities and fixed hashes without floating point conversion.
class_name AbiTypegenRpcValidation
extends RefCounted

static func failure(code: String, message: String) -> Dictionary:
    return {"ok": false, "error": {"code": code, "message": message}}

static func success(value) -> Dictionary:
    return {"ok": true, "value": value}

static func is_hex(value, byte_count: int = -1) -> bool:
    if typeof(value) != TYPE_STRING or not value.begins_with("0x"):
        return false
    var digits: String = value.substr(2)
    if byte_count >= 0 and digits.length() != byte_count * 2:
        return false
    for digit in digits:
        if not digit in "0123456789abcdefABCDEF":
            return false
    return true

static func is_quantity(value, max_digits: int = 64) -> bool:
    if not is_hex(value):
        return false
    var digits: String = value.substr(2)
    return not digits.is_empty() and digits.length() <= max_digits and (digits.length() == 1 or digits[0] != "0")

## Block/index counters are limited to Godot's nonnegative signed 64-bit integers.
static func quantity(value) -> Dictionary:
    if not is_quantity(value, 16):
        return failure("invalid_quantity", "expected a canonical nonnegative signed-64-bit RPC quantity")
    var digits: String = value.substr(2).to_lower()
    if digits.length() == 16 and not digits[0] in "01234567":
        return failure("invalid_quantity", "RPC counter exceeds signed 64-bit range")
    return success(digits.hex_to_int())

static func hex_quantity(value: int) -> String:
    return "0x%x" % value

static func hash(value) -> bool:
    return is_hex(value, 32)

static func block(value) -> bool:
    return typeof(value) == TYPE_STRING and (value in ["latest", "pending", "earliest", "safe", "finalized"] or quantity(value).get("ok", false))
