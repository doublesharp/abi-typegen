extends RefCounted

const CodecCases = preload("res://generated/codec_cases.gd")
const TokenBinding = preload("res://generated/token.gd")
const TestUtil = preload("res://tests/test_util.gd")

func _word(hex_body: String) -> PackedByteArray:
    return hex_body.hex_decode()

func _append(a: PackedByteArray, b: PackedByteArray) -> PackedByteArray:
    var out := a.duplicate()
    out.append_array(b)
    return out

func run() -> Array[String]:
    var t = TestUtil.new()
    var one := AbiTypegenCodec.word_from_u64(1)
    var two := AbiTypegenCodec.word_from_u64(2)
    var big := _word("0000000000000100000000000000000000000000000000000000000000001234")
    var owner := "0x0000000000000000000000000000000000000001"
    var other := "0x0000000000000000000000000000000000000002"

    t.equal(big.size(), 32, "large uint256 uses exactly 32 bytes")

    var large_encoded := AbiTypegenCodec.encode_call(
        TokenBinding.ABI_JSON,
        TokenBinding.TRANSFER_SIGNATURE,
        [owner, big],
        TokenBinding.TRANSFER_INPUT_TYPES
    )
    if t.ok(large_encoded, "encode uint256 above 2^128"):
        var calldata: PackedByteArray = large_encoded["value"]
        t.equal(calldata.size(), 68, "static transfer calldata size")
        t.equal(calldata.slice(36, 68), big, "large uint256 calldata remains exact")

    var decoded_large := TokenBinding.decode_balance_of_result(big)
    if t.ok(decoded_large, "decode uint256 above 2^128"):
        t.equal(decoded_large["value"], big, "large uint256 result remains exact")

    # Overloaded functions keep separate generated names/signatures and encode distinct selectors.
    var overloaded_a := CodecCases.encode_safe_transfer_from_1(owner, other, one)
    var overloaded_b := CodecCases.encode_safe_transfer_from_2(owner, other, one, PackedByteArray([1, 2, 3]))
    if t.ok(overloaded_a, "encode first overload") and t.ok(overloaded_b, "encode second overload"):
        var a: PackedByteArray = overloaded_a["value"]
        var b: PackedByteArray = overloaded_b["value"]
        t.equal(a.slice(0, 4).hex_encode(), "42842e0e", "first overload selector")
        t.equal(b.slice(0, 4).hex_encode(), "b88d4fde", "second overload selector")

    # Tuple encoding: selector + uint256 word + bool word.
    var tuple_encoded := CodecCases.encode_store_tuple({"value": big, "flag": true})
    if t.ok(tuple_encoded, "encode tuple"):
        var tuple_data: PackedByteArray = tuple_encoded["value"]
        t.equal(tuple_data.size(), 68, "static tuple calldata size")
        t.equal(tuple_data.slice(4, 36), big, "tuple uint field remains exact")
        t.equal(tuple_data.slice(36, 68), one, "tuple bool field encoded as one")

    var missing_tuple := CodecCases.encode_store_tuple({"value": one})
    t.check(not missing_tuple.get("ok", false), "missing tuple field is rejected")
    var extra_tuple := CodecCases.encode_store_tuple({"value": one, "flag": true, "extra": 7})
    t.check(not extra_tuple.get("ok", false), "extra tuple field is rejected")

    # Dynamic array encoding layout after selector: offset=32, length=2, then two words.
    var array_encoded := CodecCases.encode_store_array([one, big])
    if t.ok(array_encoded, "encode dynamic array"):
        var array_data: PackedByteArray = array_encoded["value"]
        t.equal(array_data.size(), 132, "dynamic array calldata size")
        t.equal(array_data.slice(4, 36), AbiTypegenCodec.word_from_u64(32), "array offset")
        t.equal(array_data.slice(36, 68), two, "array length")
        t.equal(array_data.slice(68, 100), one, "array item 0")
        t.equal(array_data.slice(100, 132), big, "array item 1")

    var array_result := PackedByteArray()
    array_result.append_array(AbiTypegenCodec.word_from_u64(32))
    array_result.append_array(two)
    array_result.append_array(one)
    array_result.append_array(big)
    var decoded_array := CodecCases.decode_read_array_result(array_result)
    if t.ok(decoded_array, "decode dynamic array"):
        t.equal(decoded_array["value"], [one, big], "decoded array values")

    var tuple_result := _append(big, one)
    var decoded_tuple := CodecCases.decode_read_tuple_result(tuple_result)
    if t.ok(decoded_tuple, "decode tuple result"):
        t.equal(decoded_tuple["value"], {"value": big, "flag": true}, "decoded tuple value")

    # Malformed/oversized boundary cases must return explicit failures.
    var short_word := PackedByteArray([1])
    var bad_integer := AbiTypegenCodec.encode_call(
        TokenBinding.ABI_JSON, TokenBinding.TRANSFER_SIGNATURE,
        [owner, short_word], TokenBinding.TRANSFER_INPUT_TYPES
    )
    t.check(not bad_integer.get("ok", false), "short uint256 is rejected")

    var uint8_descriptor := [{"kind": "uint", "bits": 8}]
    var uint8_abi := "[{\"type\":\"function\",\"name\":\"f\",\"inputs\":[{\"name\":\"x\",\"type\":\"uint8\"}],\"outputs\":[],\"stateMutability\":\"pure\"}]"
    var overflow := AbiTypegenCodec.encode_call(uint8_abi, "f(uint8)", [AbiTypegenCodec.word_from_u64(256)], uint8_descriptor)
    t.check(not overflow.get("ok", false), "uint8 overflow is rejected")

    var malicious_array := PackedByteArray()
    malicious_array.append_array(AbiTypegenCodec.word_from_u64(32))
    malicious_array.append_array(_word("0000000000000000000000000000000000000000000000000000000080000000"))
    var oversized := CodecCases.decode_read_array_result(malicious_array)
    t.check(not oversized.get("ok", false), "truncated/oversized dynamic array is rejected")

    var malformed_address := AbiTypegenCodec.encode_call(
        TokenBinding.ABI_JSON, TokenBinding.BALANCE_OF_SIGNATURE,
        ["0x1234"], TokenBinding.BALANCE_OF_INPUT_TYPES
    )
    t.check(not malformed_address.get("ok", false), "malformed address is rejected")

    # Indexed dynamic event fields are returned losslessly as their 32-byte topic hashes.
    var hash := "0x" + "ab".repeat(32)
    var indexed_event := CodecCases.decode_indexed_references_event(
        [CodecCases.INDEXED_REFERENCES_EVENT_TOPIC, hash, hash, hash], PackedByteArray()
    )
    if t.ok(indexed_event, "decode indexed dynamic event"):
        var expected_hash := "ab".repeat(32).hex_decode()
        t.equal(indexed_event["value"]["label"], expected_hash, "indexed string exposes topic hash")
        t.equal(indexed_event["value"]["payload"], expected_hash, "indexed bytes exposes topic hash")
        t.equal(indexed_event["value"]["values"], expected_hash, "indexed array exposes topic hash")

    # Custom error decoding uses the shared runtime; no selector/hash logic lives in GDScript.
    var error_data := "118cdaa7".hex_decode()
    error_data.append_array(one)
    var decoded_error := CodecCases.decode_ownable_unauthorized_account_error(error_data)
    if t.ok(decoded_error, "decode custom error"):
        t.equal(decoded_error["value"], owner, "custom error address")

    return t.failures
