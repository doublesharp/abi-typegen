#pragma once

#include "abi_typegen.h"

#include <godot_cpp/classes/ref_counted.hpp>
#include <godot_cpp/variant/array.hpp>
#include <godot_cpp/variant/dictionary.hpp>
#include <godot_cpp/variant/packed_byte_array.hpp>
#include <godot_cpp/variant/string.hpp>
#include <godot_cpp/variant/variant.hpp>

namespace godot {

class AbiTypegenCodec : public RefCounted {
    GDCLASS(AbiTypegenCodec, RefCounted)

protected:
    static void _bind_methods();

public:
    static Dictionary encode_call(const String &abi_json, const String &signature,
                                  const Array &args, const Array &types);
    static Dictionary encode_constructor(const String &abi_json,
                                         const PackedByteArray &bytecode,
                                         const Array &args, const Array &types);
    static Dictionary decode_call(const String &abi_json, const String &signature,
                                  const PackedByteArray &data, const Array &types,
                                  const Array &names);
    static Dictionary decode_error(const String &abi_json, const String &signature,
                                   const PackedByteArray &data, const Array &types,
                                   const Array &names);
    static Dictionary decode_event(const String &abi_json, const String &signature,
                                   const Array &topics, const PackedByteArray &data,
                                   const Array &types, const Array &names);

    static int64_t runtime_abi_version();
    static bool is_topic_hex(const String &value);
    static bool is_address_hex(const String &value);
    static bool is_zero_integer_string(const String &value);
    static PackedByteArray word_from_u64(int64_t value);

private:
    static Dictionary ok(const Variant &value);
    static Dictionary fail(const String &code, const String &message);
    static atg_value *to_value(const Variant &value, const Dictionary &type, String &error);
    static bool from_value(const atg_value *value, const Dictionary &type, Variant &out,
                           String &error);
    static atg_value *args_to_value(const Array &args, const Array &types, String &error);
    static bool result_sequence_to_variant(const atg_value *value, const Array &types,
                                           const Array &names, Variant &out, String &error);
    static bool validate_word(const PackedByteArray &word, const Dictionary &type,
                              String &error);
    static bool parse_fixed_hex(const String &value, int bytes, PackedByteArray &out);
    static PackedByteArray copy_bytes(const uint8_t *data, size_t len);
    static Dictionary result_bytes(atg_result *result);
    static Dictionary result_values(atg_result *result, const Array &types,
                                    const Array &names);
};

} // namespace godot
