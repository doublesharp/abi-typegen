#include "abi_typegen_request.h"

#include "abi_typegen_codec.h"

#include <godot_cpp/classes/http_client.hpp>
#include <godot_cpp/classes/json.hpp>
#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/variant/callable.hpp>

#include <atomic>
#include <cstring>

using namespace godot;

namespace {

std::atomic<uint64_t> NEXT_REQUEST_ID{1};

bool is_hex_body(const String &body) {
    for (int i = 0; i < body.length(); ++i) {
        const char32_t c = body[i];
        if (!((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f') ||
              (c >= 'A' && c <= 'F'))) {
            return false;
        }
    }
    return true;
}

// Ethereum JSON-RPC quantities are hex strings. Reject numeric Variants before
// JSON::stringify so caller-supplied large values can never be coerced through
// JSON numbers/floating point. This walks nested arrays/dictionaries as well.
bool contains_json_number(const Variant &value) {
    switch (value.get_type()) {
        case Variant::INT:
        case Variant::FLOAT:
            return true;
        case Variant::ARRAY: {
            const Array array = value;
            for (int i = 0; i < array.size(); ++i) {
                if (contains_json_number(array[i])) {
                    return true;
                }
            }
            return false;
        }
        case Variant::DICTIONARY: {
            const Dictionary dict = value;
            const Array keys = dict.keys();
            for (int i = 0; i < keys.size(); ++i) {
                const Variant key = keys[i];
                if (key.get_type() != Variant::STRING || contains_json_number(dict[key])) {
                    return true;
                }
            }
            return false;
        }
        default:
            return false;
    }
}

} // namespace

AbiTypegenRequest::~AbiTypegenRequest() {
    // Node owns the HTTPRequest child and destroys it before this native
    // instance. Never dereference the child here.
    pending_ = false;
    done_ = true;
}

void AbiTypegenRequest::_bind_methods() {
    ClassDB::bind_method(D_METHOD("cancel"), &AbiTypegenRequest::cancel);
    ClassDB::bind_method(D_METHOD("is_pending"), &AbiTypegenRequest::is_pending);
    ClassDB::bind_method(D_METHOD("is_done"), &AbiTypegenRequest::is_done);

    ClassDB::bind_method(D_METHOD("_deliver_failure", "error"),
                         &AbiTypegenRequest::_deliver_failure);
    ClassDB::bind_method(
        D_METHOD("_on_request_completed", "result", "response_code", "headers", "body"),
        &AbiTypegenRequest::_on_request_completed);

    ADD_SIGNAL(MethodInfo("finished", PropertyInfo(Variant::DICTIONARY, "outcome")));
    ADD_SIGNAL(MethodInfo("completed", PropertyInfo(Variant::NIL, "value")));
    ADD_SIGNAL(MethodInfo("failed", PropertyInfo(Variant::DICTIONARY, "error")));
    ADD_SIGNAL(MethodInfo("cancelled"));
}

Dictionary AbiTypegenRequest::make_error(const String &code, const String &message) {
    Dictionary error;
    error["code"] = code;
    error["message"] = message;
    return error;
}

Dictionary AbiTypegenRequest::make_failure(const Dictionary &error) {
    Dictionary out;
    out["ok"] = false;
    out["error"] = error;
    return out;
}

Dictionary AbiTypegenRequest::make_success(const Variant &value) {
    Dictionary out;
    out["ok"] = true;
    out["value"] = value;
    return out;
}

bool AbiTypegenRequest::decode_hex_data(const String &hex, PackedByteArray &out) {
    if (!hex.begins_with("0x")) {
        return false;
    }
    const String body = hex.substr(2);
    if ((body.length() % 2) != 0 || !is_hex_body(body)) {
        return false;
    }
    if (body.is_empty()) {
        out = PackedByteArray();
        return true;
    }
    out = body.hex_decode();
    return out.size() * 2 == body.length();
}

void AbiTypegenRequest::ensure_http(double timeout_seconds, int64_t body_size_limit) {
    if (http_ != nullptr) {
        return;
    }
    http_ = memnew(HTTPRequest);
    http_->set_timeout(timeout_seconds);
    http_->set_body_size_limit(body_size_limit);
    add_child(http_);
    http_->connect("request_completed", Callable(this, "_on_request_completed"));
}

void AbiTypegenRequest::emit_success(const Variant &value) {
    if (done_) {
        return;
    }
    pending_ = false;
    done_ = true;
    const Dictionary outcome = make_success(value);
    emit_signal("finished", outcome);
    emit_signal("completed", value);
    queue_free();
}

void AbiTypegenRequest::emit_failure(const Dictionary &error) {
    if (done_) {
        return;
    }
    pending_ = false;
    done_ = true;
    const Dictionary outcome = make_failure(error);
    emit_signal("finished", outcome);
    emit_signal("failed", error);
    queue_free();
}

void AbiTypegenRequest::emit_cancelled() {
    if (done_) {
        return;
    }
    pending_ = false;
    done_ = true;
    const Dictionary error = make_error("cancelled", "request cancelled");
    emit_signal("finished", make_failure(error));
    emit_signal("failed", error);
    emit_signal("cancelled");
    queue_free();
}

void AbiTypegenRequest::_deliver_failure(const Dictionary &error) {
    emit_failure(error);
}

void AbiTypegenRequest::fail_deferred(const String &code, const String &message) {
    if (done_) {
        return;
    }
    pending_ = true;
    call_deferred("_deliver_failure", make_error(code, message));
}

void AbiTypegenRequest::start_rpc(const String &rpc_url, const String &method,
                                  const Array &params, double timeout_seconds,
                                  int64_t body_size_limit) {
    if (done_ || pending_) {
        fail_deferred("request_state", "request has already been started");
        return;
    }
    if (rpc_url.is_empty()) {
        fail_deferred("missing_rpc_url", "RPC URL is empty");
        return;
    }
    if (method.is_empty()) {
        fail_deferred("invalid_rpc_method", "JSON-RPC method is empty");
        return;
    }
    if (contains_json_number(params)) {
        fail_deferred(
            "numeric_json_param",
            "JSON-RPC numeric values are rejected; encode Ethereum quantities as lossless 0x hex Strings");
        return;
    }

    mode_ = MODE_RAW;
    ensure_http(timeout_seconds, body_size_limit);

    Dictionary payload;
    payload["jsonrpc"] = "2.0";
    request_id_ = "abi-typegen-" + String::num_uint64(NEXT_REQUEST_ID.fetch_add(1));
    payload["id"] = request_id_;
    payload["method"] = method;
    payload["params"] = params;

    PackedStringArray headers;
    headers.append("Content-Type: application/json");
    headers.append("Accept: application/json");

    pending_ = true;
    const Error started = http_->request(rpc_url, headers, HTTPClient::METHOD_POST,
                                         JSON::stringify(payload));
    if (started != OK) {
        pending_ = false;
        fail_deferred("http_start_error",
                      "HTTPRequest.request failed with Error code " + String::num_int64(static_cast<int64_t>(started)));
    }
}

void AbiTypegenRequest::start_contract_call(
    const String &rpc_url, const String &contract_address, const String &data_hex,
    const String &block, const String &abi_json, const String &signature,
    const Array &output_types, const Array &output_names, double timeout_seconds,
    int64_t body_size_limit) {
    mode_ = MODE_CONTRACT_CALL;
    abi_json_ = abi_json;
    signature_ = signature;
    output_types_ = output_types.duplicate(true);
    output_names_ = output_names.duplicate(true);

    Dictionary call;
    call["to"] = contract_address;
    call["data"] = data_hex;
    Array params;
    params.append(call);
    params.append(block);

    // start_rpc initializes the HTTPRequest but would otherwise reset mode_.
    if (done_ || pending_) {
        fail_deferred("request_state", "request has already been started");
        return;
    }
    if (rpc_url.is_empty()) {
        fail_deferred("missing_rpc_url", "RPC URL is empty");
        return;
    }
    ensure_http(timeout_seconds, body_size_limit);

    Dictionary payload;
    payload["jsonrpc"] = "2.0";
    request_id_ = "abi-typegen-" + String::num_uint64(NEXT_REQUEST_ID.fetch_add(1));
    payload["id"] = request_id_;
    payload["method"] = "eth_call";
    payload["params"] = params;

    PackedStringArray headers;
    headers.append("Content-Type: application/json");
    headers.append("Accept: application/json");

    pending_ = true;
    const Error started = http_->request(rpc_url, headers, HTTPClient::METHOD_POST,
                                         JSON::stringify(payload));
    if (started != OK) {
        pending_ = false;
        fail_deferred("http_start_error",
                      "HTTPRequest.request failed with Error code " + String::num_int64(static_cast<int64_t>(started)));
    }
}

void AbiTypegenRequest::_on_request_completed(int64_t result, int64_t response_code,
                                              const PackedStringArray &headers,
                                              const PackedByteArray &body) {
    (void)headers;
    if (!pending_ || done_) {
        return;
    }
    pending_ = false;

    if (result != HTTPRequest::RESULT_SUCCESS) {
        Dictionary error = make_error("http_error",
                                      "HTTPRequest completed with result " + String::num_int64(result));
        error["http_result"] = result;
        emit_failure(error);
        return;
    }
    if (response_code < 200 || response_code >= 300) {
        Dictionary error = make_error("http_status",
                                      "RPC server returned HTTP status " + String::num_int64(response_code));
        error["response_code"] = response_code;
        emit_failure(error);
        return;
    }
    if (body.size() == 0) {
        emit_failure(make_error("empty_response", "RPC server returned an empty response body"));
        return;
    }

    const String text = String::utf8(reinterpret_cast<const char *>(body.ptr()), body.size());
    const CharString roundtrip = text.utf8();
    if (roundtrip.length() != body.size() ||
        std::memcmp(roundtrip.ptr(), body.ptr(), static_cast<size_t>(body.size())) != 0) {
        emit_failure(make_error("invalid_utf8", "RPC response body is not valid UTF-8"));
        return;
    }
    Ref<JSON> json;
    json.instantiate();
    const Error parse_error = json->parse(text);
    if (parse_error != OK) {
        Dictionary error = make_error("invalid_json", "RPC response is not valid JSON");
        error["json_error"] = json->get_error_message();
        error["json_error_line"] = json->get_error_line();
        emit_failure(error);
        return;
    }

    const Variant parsed = json->get_data();
    if (parsed.get_type() != Variant::DICTIONARY) {
        emit_failure(make_error("invalid_rpc_response", "JSON-RPC response must be an object"));
        return;
    }
    const Dictionary response = parsed;
    if (!response.has("jsonrpc") || response["jsonrpc"].get_type() != Variant::STRING ||
        String(response["jsonrpc"]) != "2.0" || !response.has("id") ||
        response["id"].get_type() != Variant::STRING || String(response["id"]) != request_id_) {
        emit_failure(make_error("invalid_rpc_response", "JSON-RPC version or request id does not match"));
        return;
    }
    if (response.has("error") && response["error"].get_type() != Variant::NIL) {
        Dictionary error = make_error("rpc_error", "JSON-RPC returned an error");
        error["rpc"] = response["error"];
        emit_failure(error);
        return;
    }
    if (!response.has("result")) {
        emit_failure(make_error("invalid_rpc_response", "JSON-RPC response is missing result"));
        return;
    }

    const Variant value = response["result"];
    if (mode_ == MODE_RAW) {
        emit_success(value);
        return;
    }

    if (value.get_type() != Variant::STRING) {
        emit_failure(make_error("invalid_call_result", "eth_call result must be a 0x-prefixed hex String"));
        return;
    }
    PackedByteArray data;
    if (!decode_hex_data(String(value), data)) {
        emit_failure(make_error("invalid_call_result", "eth_call returned malformed hex data"));
        return;
    }
    const Dictionary decoded = AbiTypegenCodec::decode_call(
        abi_json_, signature_, data, output_types_, output_names_);
    if (!bool(decoded.get("ok", false))) {
        Dictionary error = make_error("decode_failed", "ABI result decoding failed");
        error["codec"] = decoded.get("error", Dictionary());
        emit_failure(error);
        return;
    }
    emit_success(decoded.get("value", Variant()));
}

void AbiTypegenRequest::cancel() {
    if (done_) {
        return;
    }
    if (pending_ && http_ != nullptr) {
        http_->cancel_request();
    }
    emit_cancelled();
}

bool AbiTypegenRequest::is_pending() const {
    return pending_ && !done_;
}

bool AbiTypegenRequest::is_done() const {
    return done_;
}
