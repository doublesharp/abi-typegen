#include "abi_typegen_client.h"

#include "abi_typegen_codec.h"
#include "abi_typegen_request.h"

#include <godot_cpp/core/class_db.hpp>

using namespace godot;

void AbiTypegenClient::_bind_methods() {
    ClassDB::bind_method(D_METHOD("set_rpc_url", "value"), &AbiTypegenClient::set_rpc_url);
    ClassDB::bind_method(D_METHOD("get_rpc_url"), &AbiTypegenClient::get_rpc_url);
    ClassDB::bind_method(D_METHOD("set_timeout_seconds", "value"),
                         &AbiTypegenClient::set_timeout_seconds);
    ClassDB::bind_method(D_METHOD("get_timeout_seconds"),
                         &AbiTypegenClient::get_timeout_seconds);
    ClassDB::bind_method(D_METHOD("set_body_size_limit", "value"),
                         &AbiTypegenClient::set_body_size_limit);
    ClassDB::bind_method(D_METHOD("get_body_size_limit"),
                         &AbiTypegenClient::get_body_size_limit);

    ClassDB::bind_method(D_METHOD("rpc", "method", "params"), &AbiTypegenClient::rpc,
                         DEFVAL(Array()));
    ClassDB::bind_method(
        D_METHOD("call_contract", "contract_address", "abi_json", "signature", "args",
                 "input_types", "output_types", "output_names", "block"),
        &AbiTypegenClient::call_contract, DEFVAL("latest"));
    ClassDB::bind_method(D_METHOD("failed_request", "code", "message"),
                         &AbiTypegenClient::failed_request);

    ADD_PROPERTY(PropertyInfo(Variant::STRING, "rpc_url"), "set_rpc_url", "get_rpc_url");
    ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "timeout_seconds"), "set_timeout_seconds",
                 "get_timeout_seconds");
    ADD_PROPERTY(PropertyInfo(Variant::INT, "body_size_limit"), "set_body_size_limit",
                 "get_body_size_limit");
}

void AbiTypegenClient::set_rpc_url(const String &value) {
    rpc_url_ = value;
}

String AbiTypegenClient::get_rpc_url() const {
    return rpc_url_;
}

void AbiTypegenClient::set_timeout_seconds(double value) {
    timeout_seconds_ = value < 0.0 ? 0.0 : value;
}

double AbiTypegenClient::get_timeout_seconds() const {
    return timeout_seconds_;
}

void AbiTypegenClient::set_body_size_limit(int64_t value) {
    body_size_limit_ = value;
}

int64_t AbiTypegenClient::get_body_size_limit() const {
    return body_size_limit_;
}

AbiTypegenRequest *AbiTypegenClient::failed_request(const String &code,
                                                    const String &message) {
    AbiTypegenRequest *request = memnew(AbiTypegenRequest);
    add_child(request);
    request->fail_deferred(code, message);
    return request;
}

AbiTypegenRequest *AbiTypegenClient::rpc(const String &method, const Array &params) {
    AbiTypegenRequest *request = memnew(AbiTypegenRequest);
    add_child(request);
    request->start_rpc(rpc_url_, method, params, timeout_seconds_, body_size_limit_);
    return request;
}

AbiTypegenRequest *AbiTypegenClient::call_contract(
    const String &contract_address, const String &abi_json, const String &signature,
    const Array &args, const Array &input_types, const Array &output_types,
    const Array &output_names, const String &block) {
    if (!AbiTypegenCodec::is_address_hex(contract_address)) {
        return failed_request("invalid_address",
                              "contract address must be exactly 20 bytes of 0x-prefixed hex");
    }
    if (block.is_empty()) {
        return failed_request("invalid_block", "block parameter cannot be empty");
    }
    const Dictionary encoded = AbiTypegenCodec::encode_call(abi_json, signature, args, input_types);
    if (!bool(encoded.get("ok", false))) {
        Dictionary codec_error = encoded.get("error", Dictionary());
        return failed_request("encode_failed", String(codec_error.get("message", "ABI encoding failed")));
    }
    const Variant encoded_value = encoded.get("value", Variant());
    if (encoded_value.get_type() != Variant::PACKED_BYTE_ARRAY) {
        return failed_request("encode_failed", "ABI codec returned a non-byte result");
    }
    const PackedByteArray data = encoded_value;
    AbiTypegenRequest *request = memnew(AbiTypegenRequest);
    add_child(request);
    request->start_contract_call(rpc_url_, contract_address, "0x" + data.hex_encode(), block,
                                 abi_json, signature, output_types, output_names,
                                 timeout_seconds_, body_size_limit_);
    return request;
}
