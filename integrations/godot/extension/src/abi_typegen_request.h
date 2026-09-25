#pragma once

#include <godot_cpp/classes/http_request.hpp>
#include <godot_cpp/classes/node.hpp>
#include <godot_cpp/variant/array.hpp>
#include <godot_cpp/variant/dictionary.hpp>
#include <godot_cpp/variant/packed_byte_array.hpp>
#include <godot_cpp/variant/packed_string_array.hpp>
#include <godot_cpp/variant/string.hpp>
#include <godot_cpp/variant/variant.hpp>

namespace godot {

class AbiTypegenRequest : public Node {
    GDCLASS(AbiTypegenRequest, Node)

public:
    enum DecodeMode {
        MODE_RAW = 0,
        MODE_CONTRACT_CALL = 1,
    };

private:
    HTTPRequest *http_ = nullptr;
    bool pending_ = false;
    bool done_ = false;
    DecodeMode mode_ = MODE_RAW;
    String abi_json_;
    String signature_;
    Array output_types_;
    Array output_names_;
    String request_id_;

    static Dictionary make_error(const String &code, const String &message);
    static Dictionary make_failure(const Dictionary &error);
    static Dictionary make_success(const Variant &value);
    static bool decode_hex_data(const String &hex, PackedByteArray &out);

    void ensure_http(double timeout_seconds, int64_t body_size_limit);
    void emit_success(const Variant &value);
    void emit_failure(const Dictionary &error);
    void emit_cancelled();
    void _deliver_failure(const Dictionary &error);
    void _on_request_completed(int64_t result, int64_t response_code,
                               const PackedStringArray &headers,
                               const PackedByteArray &body);

protected:
    static void _bind_methods();

public:
    AbiTypegenRequest() = default;
    ~AbiTypegenRequest() override;

    void start_rpc(const String &rpc_url, const String &method, const Array &params,
                   double timeout_seconds, int64_t body_size_limit);
    void start_contract_call(const String &rpc_url, const String &contract_address,
                             const String &data_hex, const String &block,
                             const String &abi_json, const String &signature,
                             const Array &output_types, const Array &output_names,
                             double timeout_seconds, int64_t body_size_limit);
    void fail_deferred(const String &code, const String &message);

    void cancel();
    bool is_pending() const;
    bool is_done() const;
};

} // namespace godot
