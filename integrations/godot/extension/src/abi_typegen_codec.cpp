#include "abi_typegen_codec.h"

#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/variant/utility_functions.hpp>

#include <climits>
#include <cstdint>
#include <cstring>

using namespace godot;

namespace {

String kind_of(const Dictionary &type) {
    if (!type.has("kind") || type["kind"].get_type() != Variant::STRING) {
        return String();
    }
    return type["kind"];
}

bool valid_bits(const Dictionary &type, int &bits) {
    if (!type.has("bits") || type["bits"].get_type() != Variant::INT) {
        return false;
    }
    bits = int(type["bits"]);
    return bits >= 8 && bits <= 256 && bits % 8 == 0;
}

bool is_all_hex(const String &value) {
    for (int i = 0; i < value.length(); ++i) {
        const char32_t c = value[i];
        if (!((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f') ||
              (c >= 'A' && c <= 'F'))) {
            return false;
        }
    }
    return true;
}

Dictionary make_error(const String &code, const String &message) {
    Dictionary error;
    error["code"] = code;
    error["message"] = message;
    return error;
}

} // namespace

void AbiTypegenCodec::_bind_methods() {
    ClassDB::bind_static_method("AbiTypegenCodec",
                                D_METHOD("encode_call", "abi_json", "signature", "args", "types"),
                                &AbiTypegenCodec::encode_call);
    ClassDB::bind_static_method("AbiTypegenCodec",
                                D_METHOD("encode_constructor", "abi_json", "bytecode", "args", "types"),
                                &AbiTypegenCodec::encode_constructor);
    ClassDB::bind_static_method("AbiTypegenCodec",
                                D_METHOD("decode_call", "abi_json", "signature", "data", "types", "names"),
                                &AbiTypegenCodec::decode_call);
    ClassDB::bind_static_method("AbiTypegenCodec",
                                D_METHOD("decode_error", "abi_json", "signature", "data", "types", "names"),
                                &AbiTypegenCodec::decode_error);
    ClassDB::bind_static_method("AbiTypegenCodec",
                                D_METHOD("decode_event", "abi_json", "signature", "topics", "data", "types", "names"),
                                &AbiTypegenCodec::decode_event);
    ClassDB::bind_static_method("AbiTypegenCodec", D_METHOD("runtime_abi_version"),
                                &AbiTypegenCodec::runtime_abi_version);
    ClassDB::bind_static_method("AbiTypegenCodec", D_METHOD("is_topic_hex", "value"),
                                &AbiTypegenCodec::is_topic_hex);
    ClassDB::bind_static_method("AbiTypegenCodec", D_METHOD("is_address_hex", "value"),
                                &AbiTypegenCodec::is_address_hex);
    ClassDB::bind_static_method("AbiTypegenCodec", D_METHOD("is_zero_integer_string", "value"),
                                &AbiTypegenCodec::is_zero_integer_string);
    ClassDB::bind_static_method("AbiTypegenCodec", D_METHOD("word_from_u64", "value"),
                                &AbiTypegenCodec::word_from_u64);
}

Dictionary AbiTypegenCodec::ok(const Variant &value) {
    Dictionary out;
    out["ok"] = true;
    out["value"] = value;
    return out;
}

Dictionary AbiTypegenCodec::fail(const String &code, const String &message) {
    Dictionary out;
    out["ok"] = false;
    out["error"] = make_error(code, message);
    return out;
}

PackedByteArray AbiTypegenCodec::copy_bytes(const uint8_t *data, size_t len) {
    PackedByteArray out;
    if (len == 0) {
        return out;
    }
    if (data == nullptr || len > static_cast<size_t>(INT32_MAX)) {
        return out;
    }
    out.resize(static_cast<int>(len));
    std::memcpy(out.ptrw(), data, len);
    return out;
}

bool AbiTypegenCodec::parse_fixed_hex(const String &value, int bytes, PackedByteArray &out) {
    if (!value.begins_with("0x") || value.length() != 2 + bytes * 2) {
        return false;
    }
    const String body = value.substr(2);
    if (!is_all_hex(body)) {
        return false;
    }
    out = body.hex_decode();
    return out.size() == bytes;
}

int64_t AbiTypegenCodec::runtime_abi_version() {
    return ATG_ABI_VERSION;
}

bool AbiTypegenCodec::is_topic_hex(const String &value) {
    PackedByteArray ignored;
    return parse_fixed_hex(value, 32, ignored);
}

bool AbiTypegenCodec::is_address_hex(const String &value) {
    PackedByteArray ignored;
    return parse_fixed_hex(value, 20, ignored);
}

bool AbiTypegenCodec::is_zero_integer_string(const String &value) {
    String text = value.strip_edges();
    if (text.is_empty()) {
        return false;
    }
    if (text.begins_with("0x") || text.begins_with("0X")) {
        text = text.substr(2);
        if (text.is_empty() || !is_all_hex(text)) {
            return false;
        }
        for (int i = 0; i < text.length(); ++i) {
            if (text[i] != '0') {
                return false;
            }
        }
        return true;
    }
    for (int i = 0; i < text.length(); ++i) {
        if (text[i] < '0' || text[i] > '9') {
            return false;
        }
        if (text[i] != '0') {
            return false;
        }
    }
    return true;
}

PackedByteArray AbiTypegenCodec::word_from_u64(int64_t value) {
    PackedByteArray out;
    if (value < 0) {
        return out;
    }
    out.resize(32);
    uint64_t n = static_cast<uint64_t>(value);
    for (int i = 0; i < 8; ++i) {
        out.set(31 - i, static_cast<uint8_t>(n & 0xff));
        n >>= 8;
    }
    return out;
}

bool AbiTypegenCodec::validate_word(const PackedByteArray &word, const Dictionary &type,
                                    String &error) {
    if (word.size() != 32) {
        error = "integer values must be exactly 32 bytes";
        return false;
    }
    int bits = 0;
    if (!valid_bits(type, bits)) {
        error = "invalid integer bit width descriptor";
        return false;
    }
    const String kind = kind_of(type);
    const int value_bytes = bits / 8;
    const int prefix_bytes = 32 - value_bytes;
    if (kind == "uint") {
        for (int i = 0; i < prefix_bytes; ++i) {
            if (word[i] != 0) {
                error = "uint value exceeds declared bit width";
                return false;
            }
        }
        return true;
    }
    if (kind == "int") {
        const bool negative = (word[prefix_bytes] & 0x80) != 0;
        const uint8_t fill = negative ? 0xff : 0x00;
        for (int i = 0; i < prefix_bytes; ++i) {
            if (word[i] != fill) {
                error = "int value is not canonically sign-extended for declared bit width";
                return false;
            }
        }
        return true;
    }
    error = "descriptor is not an integer";
    return false;
}

atg_value *AbiTypegenCodec::to_value(const Variant &value, const Dictionary &type, String &error) {
    const String kind = kind_of(type);
    if (kind.is_empty()) {
        error = "missing type descriptor kind";
        return nullptr;
    }

    if (kind == "bool") {
        if (value.get_type() != Variant::BOOL) {
            error = "bool value must be a Godot bool";
            return nullptr;
        }
        return atg_value_bool(bool(value) ? 1 : 0);
    }

    if (kind == "uint" || kind == "int") {
        if (value.get_type() != Variant::PACKED_BYTE_ARRAY) {
            error = "Solidity integers use a 32-byte big-endian PackedByteArray";
            return nullptr;
        }
        const PackedByteArray word = value;
        if (!validate_word(word, type, error)) {
            return nullptr;
        }
        return atg_value_word(word.ptr());
    }

    if (kind == "address") {
        if (value.get_type() != Variant::STRING) {
            error = "address value must be a 20-byte 0x-prefixed hex String";
            return nullptr;
        }
        PackedByteArray address;
        if (!parse_fixed_hex(String(value), 20, address)) {
            error = "invalid address; expected exactly 20 bytes of 0x-prefixed hex";
            return nullptr;
        }
        uint8_t word[32] = {0};
        std::memcpy(word + 12, address.ptr(), 20);
        return atg_value_word(word);
    }

    if (kind == "string") {
        if (value.get_type() != Variant::STRING) {
            error = "string value must be a Godot String";
            return nullptr;
        }
        const CharString utf8 = String(value).utf8();
        return atg_value_bytes(reinterpret_cast<const uint8_t *>(utf8.ptr()), utf8.length());
    }

    if (kind == "bytes" || kind == "bytes_n") {
        if (value.get_type() != Variant::PACKED_BYTE_ARRAY) {
            error = "bytes value must be a PackedByteArray";
            return nullptr;
        }
        const PackedByteArray bytes = value;
        if (kind == "bytes_n") {
            if (!type.has("size") || type["size"].get_type() != Variant::INT) {
                error = "bytes_n descriptor is missing size";
                return nullptr;
            }
            const int size = int(type["size"]);
            if (size < 1 || size > 32 || bytes.size() != size) {
                error = "fixed bytes value has incorrect length";
                return nullptr;
            }
        }
        return atg_value_bytes(bytes.is_empty() ? nullptr : bytes.ptr(), static_cast<size_t>(bytes.size()));
    }

    if (kind == "array" || kind == "fixed_array") {
        if (value.get_type() != Variant::ARRAY) {
            error = "array value must be a Godot Array";
            return nullptr;
        }
        if (!type.has("item") || type["item"].get_type() != Variant::DICTIONARY) {
            error = "array descriptor is missing item type";
            return nullptr;
        }
        const Array values = value;
        if (kind == "fixed_array") {
            if (!type.has("size") || type["size"].get_type() != Variant::INT ||
                values.size() != int(type["size"])) {
                error = "fixed array value has incorrect length";
                return nullptr;
            }
        }
        atg_value *seq = atg_value_seq();
        if (!seq) {
            error = "runtime allocation failed";
            return nullptr;
        }
        const Dictionary item_type = type["item"];
        for (int i = 0; i < values.size(); ++i) {
            atg_value *child = to_value(values[i], item_type, error);
            if (!child || atg_value_push(seq, child) != 0) {
                if (child) {
                    atg_value_free(child);
                }
                atg_value_free(seq);
                if (error.is_empty()) {
                    error = "runtime sequence allocation failed";
                }
                return nullptr;
            }
            atg_value_free(child);
        }
        return seq;
    }

    if (kind == "tuple") {
        if (value.get_type() != Variant::DICTIONARY) {
            error = "tuple value must be a Dictionary keyed by generated field names";
            return nullptr;
        }
        if (!type.has("fields") || type["fields"].get_type() != Variant::ARRAY ||
            !type.has("items") || type["items"].get_type() != Variant::ARRAY) {
            error = "tuple descriptor is malformed";
            return nullptr;
        }
        const Dictionary tuple = value;
        const Array fields = type["fields"];
        const Array items = type["items"];
        if (fields.size() != items.size() || tuple.size() != fields.size()) {
            error = "tuple must contain exactly the generated fields";
            return nullptr;
        }
        atg_value *seq = atg_value_seq();
        if (!seq) {
            error = "runtime allocation failed";
            return nullptr;
        }
        for (int i = 0; i < fields.size(); ++i) {
            if (fields[i].get_type() != Variant::STRING || items[i].get_type() != Variant::DICTIONARY) {
                atg_value_free(seq);
                error = "tuple descriptor is malformed";
                return nullptr;
            }
            const String field = fields[i];
            if (!tuple.has(field)) {
                atg_value_free(seq);
                error = "tuple is missing field: " + field;
                return nullptr;
            }
            atg_value *child = to_value(tuple[field], Dictionary(items[i]), error);
            if (!child || atg_value_push(seq, child) != 0) {
                if (child) {
                    atg_value_free(child);
                }
                atg_value_free(seq);
                if (error.is_empty()) {
                    error = "runtime sequence allocation failed";
                }
                return nullptr;
            }
            atg_value_free(child);
        }
        return seq;
    }

    error = "unsupported type descriptor kind: " + kind;
    return nullptr;
}

bool AbiTypegenCodec::from_value(const atg_value *value, const Dictionary &type, Variant &out,
                                 String &error) {
    if (!value) {
        error = "runtime returned a null value";
        return false;
    }
    const String kind = kind_of(type);
    if (kind == "bool") {
        if (atg_value_kind(value) != 1) {
            error = "decoded value kind mismatch for bool";
            return false;
        }
        out = atg_value_get_bool(value) != 0;
        return true;
    }

    if (kind == "uint" || kind == "int") {
        if (atg_value_kind(value) != 2 || atg_value_data(value) == nullptr) {
            error = "decoded value kind mismatch for integer";
            return false;
        }
        const PackedByteArray word = copy_bytes(atg_value_data(value), 32);
        if (!validate_word(word, type, error)) {
            return false;
        }
        out = word;
        return true;
    }

    if (kind == "address") {
        if (atg_value_kind(value) != 2 || atg_value_data(value) == nullptr) {
            error = "decoded value kind mismatch for address";
            return false;
        }
        PackedByteArray address = copy_bytes(atg_value_data(value) + 12, 20);
        if (address.size() != 20) {
            error = "could not copy decoded address";
            return false;
        }
        out = "0x" + address.hex_encode();
        return true;
    }

    if (kind == "bytes" || kind == "bytes_n" || kind == "string") {
        if (atg_value_kind(value) != 3) {
            error = "decoded value kind mismatch for bytes/string";
            return false;
        }
        const size_t len = atg_value_len(value);
        const uint8_t *data = atg_value_data(value);
        if (len && !data) {
            error = "runtime returned null bytes storage";
            return false;
        }
        if (len > static_cast<size_t>(INT32_MAX)) {
            error = "decoded bytes exceed Godot's array limit";
            return false;
        }
        const PackedByteArray bytes = copy_bytes(data, len);
        if (bytes.size() != static_cast<int>(len)) {
            error = "could not copy decoded bytes";
            return false;
        }
        if (kind == "bytes_n") {
            if (!type.has("size") || type["size"].get_type() != Variant::INT ||
                bytes.size() != int(type["size"])) {
                error = "decoded fixed bytes length mismatch";
                return false;
            }
        }
        if (kind == "string") {
            const String text = String::utf8(reinterpret_cast<const char *>(data), static_cast<int>(len));
            const CharString roundtrip = text.utf8();
            if (roundtrip.length() != static_cast<int>(len) ||
                (len && std::memcmp(roundtrip.ptr(), data, len) != 0)) {
                error = "decoded Solidity string is not valid UTF-8";
                return false;
            }
            out = text;
        } else {
            out = bytes;
        }
        return true;
    }

    if (kind == "array" || kind == "fixed_array") {
        if (atg_value_kind(value) != 4 || !type.has("item") ||
            type["item"].get_type() != Variant::DICTIONARY) {
            error = "decoded value kind mismatch for array";
            return false;
        }
        const size_t len = atg_value_len(value);
        if (kind == "fixed_array" &&
            (!type.has("size") || type["size"].get_type() != Variant::INT ||
             len != static_cast<size_t>(int(type["size"])))) {
            error = "decoded fixed array length mismatch";
            return false;
        }
        if (len > static_cast<size_t>(INT32_MAX)) {
            error = "decoded array is too large for Godot Array";
            return false;
        }
        Array values;
        values.resize(static_cast<int>(len));
        const Dictionary item_type = type["item"];
        for (size_t i = 0; i < len; ++i) {
            Variant child;
            if (!from_value(atg_value_at(value, i), item_type, child, error)) {
                return false;
            }
            values[static_cast<int>(i)] = child;
        }
        out = values;
        return true;
    }

    if (kind == "tuple") {
        if (atg_value_kind(value) != 4 || !type.has("fields") ||
            type["fields"].get_type() != Variant::ARRAY || !type.has("items") ||
            type["items"].get_type() != Variant::ARRAY) {
            error = "decoded value kind mismatch for tuple";
            return false;
        }
        const Array fields = type["fields"];
        const Array items = type["items"];
        if (fields.size() != items.size() || atg_value_len(value) != static_cast<size_t>(fields.size())) {
            error = "decoded tuple arity mismatch";
            return false;
        }
        Dictionary tuple;
        for (int i = 0; i < fields.size(); ++i) {
            if (fields[i].get_type() != Variant::STRING || items[i].get_type() != Variant::DICTIONARY) {
                error = "tuple descriptor is malformed";
                return false;
            }
            Variant child;
            if (!from_value(atg_value_at(value, i), Dictionary(items[i]), child, error)) {
                return false;
            }
            tuple[fields[i]] = child;
        }
        out = tuple;
        return true;
    }

    error = "unsupported type descriptor kind: " + kind;
    return false;
}

atg_value *AbiTypegenCodec::args_to_value(const Array &args, const Array &types, String &error) {
    if (args.size() != types.size()) {
        error = "argument count does not match generated ABI metadata";
        return nullptr;
    }
    atg_value *seq = atg_value_seq();
    if (!seq) {
        error = "runtime allocation failed";
        return nullptr;
    }
    for (int i = 0; i < args.size(); ++i) {
        if (types[i].get_type() != Variant::DICTIONARY) {
            atg_value_free(seq);
            error = "generated type metadata is malformed";
            return nullptr;
        }
        atg_value *child = to_value(args[i], Dictionary(types[i]), error);
        if (!child || atg_value_push(seq, child) != 0) {
            if (child) {
                atg_value_free(child);
            }
            atg_value_free(seq);
            if (error.is_empty()) {
                error = "runtime sequence allocation failed";
            }
            return nullptr;
        }
        atg_value_free(child);
    }
    return seq;
}

bool AbiTypegenCodec::result_sequence_to_variant(const atg_value *value, const Array &types,
                                                 const Array &names, Variant &out,
                                                 String &error) {
    if (!value || atg_value_kind(value) != 4 || atg_value_len(value) != static_cast<size_t>(types.size()) ||
        names.size() != types.size()) {
        error = "decoded result arity mismatch";
        return false;
    }
    if (types.is_empty()) {
        out = Variant();
        return true;
    }
    if (types.size() == 1) {
        if (types[0].get_type() != Variant::DICTIONARY) {
            error = "generated output metadata is malformed";
            return false;
        }
        return from_value(atg_value_at(value, 0), Dictionary(types[0]), out, error);
    }
    Dictionary values;
    for (int i = 0; i < types.size(); ++i) {
        if (types[i].get_type() != Variant::DICTIONARY || names[i].get_type() != Variant::STRING) {
            error = "generated output metadata is malformed";
            return false;
        }
        Variant child;
        if (!from_value(atg_value_at(value, i), Dictionary(types[i]), child, error)) {
            return false;
        }
        values[names[i]] = child;
    }
    out = values;
    return true;
}

Dictionary AbiTypegenCodec::result_bytes(atg_result *result) {
    if (!result) {
        return fail("runtime_null_result", "abi-typegen runtime returned null result");
    }
    const char *runtime_error = atg_result_error(result);
    if (runtime_error) {
        const String message = String::utf8(runtime_error);
        atg_result_free(result);
        return fail("runtime_error", message);
    }
    const size_t len = atg_result_len(result);
    const uint8_t *data = atg_result_data(result);
    if (len && !data) {
        atg_result_free(result);
        return fail("runtime_invalid_result", "runtime returned null byte storage");
    }
    if (len > static_cast<size_t>(INT32_MAX)) {
        atg_result_free(result);
        return fail("runtime_invalid_result", "runtime byte result exceeds Godot's array limit");
    }
    PackedByteArray bytes = copy_bytes(data, len);
    atg_result_free(result);
    if (bytes.size() != static_cast<int>(len)) {
        return fail("runtime_invalid_result", "could not copy runtime byte result");
    }
    return ok(bytes);
}

Dictionary AbiTypegenCodec::result_values(atg_result *result, const Array &types,
                                          const Array &names) {
    if (!result) {
        return fail("runtime_null_result", "abi-typegen runtime returned null result");
    }
    const char *runtime_error = atg_result_error(result);
    if (runtime_error) {
        const String message = String::utf8(runtime_error);
        atg_result_free(result);
        return fail("runtime_error", message);
    }
    Variant value;
    String error;
    if (!result_sequence_to_variant(atg_result_value(result), types, names, value, error)) {
        atg_result_free(result);
        return fail("decoded_type_mismatch", error);
    }
    atg_result_free(result);
    return ok(value);
}

Dictionary AbiTypegenCodec::encode_call(const String &abi_json, const String &signature,
                                        const Array &args, const Array &types) {
    String error;
    atg_value *values = args_to_value(args, types, error);
    if (!values) {
        return fail("invalid_arguments", error);
    }
    const CharString abi = abi_json.utf8();
    const CharString sig = signature.utf8();
    atg_result *result = atg_encode(abi.ptr(), sig.ptr(), values);
    atg_value_free(values);
    return result_bytes(result);
}

Dictionary AbiTypegenCodec::encode_constructor(const String &abi_json,
                                               const PackedByteArray &bytecode,
                                               const Array &args, const Array &types) {
    String error;
    atg_value *values = args_to_value(args, types, error);
    if (!values) {
        return fail("invalid_arguments", error);
    }
    const CharString abi = abi_json.utf8();
    atg_result *result = atg_encode_constructor(abi.ptr(), bytecode.is_empty() ? nullptr : bytecode.ptr(), bytecode.size(), values);
    atg_value_free(values);
    return result_bytes(result);
}

Dictionary AbiTypegenCodec::decode_call(const String &abi_json, const String &signature,
                                        const PackedByteArray &data, const Array &types,
                                        const Array &names) {
    const CharString abi = abi_json.utf8();
    const CharString sig = signature.utf8();
    return result_values(atg_decode(abi.ptr(), sig.ptr(), data.is_empty() ? nullptr : data.ptr(), data.size()), types, names);
}

Dictionary AbiTypegenCodec::decode_error(const String &abi_json, const String &signature,
                                         const PackedByteArray &data, const Array &types,
                                         const Array &names) {
    const CharString abi = abi_json.utf8();
    const CharString sig = signature.utf8();
    return result_values(atg_decode_error(abi.ptr(), sig.ptr(), data.is_empty() ? nullptr : data.ptr(), data.size()), types, names);
}

Dictionary AbiTypegenCodec::decode_event(const String &abi_json, const String &signature,
                                         const Array &topics, const PackedByteArray &data,
                                         const Array &types, const Array &names) {
    if (topics.size() > 4) {
        return fail("too_many_topics", "Ethereum logs can contain at most four topics");
    }
    PackedByteArray packed_topics;
    packed_topics.resize(topics.size() * 32);
    for (int i = 0; i < topics.size(); ++i) {
        PackedByteArray word;
        if (topics[i].get_type() == Variant::STRING) {
            if (!parse_fixed_hex(String(topics[i]), 32, word)) {
                return fail("invalid_topic", "topic must be a 32-byte 0x-prefixed hex String");
            }
        } else if (topics[i].get_type() == Variant::PACKED_BYTE_ARRAY) {
            word = topics[i];
            if (word.size() != 32) {
                return fail("invalid_topic", "topic PackedByteArray must contain exactly 32 bytes");
            }
        } else {
            return fail("invalid_topic", "topic must be a 32-byte hex String or PackedByteArray");
        }
        std::memcpy(packed_topics.ptrw() + i * 32, word.ptr(), 32);
    }
    const CharString abi = abi_json.utf8();
    const CharString sig = signature.utf8();
    return result_values(atg_decode_event(abi.ptr(), sig.ptr(), packed_topics.is_empty() ? nullptr : packed_topics.ptr(), topics.size(),
                                          data.is_empty() ? nullptr : data.ptr(), data.size()),
                         types, names);
}
