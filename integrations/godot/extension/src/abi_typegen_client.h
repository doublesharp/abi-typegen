#pragma once

#include <godot_cpp/classes/node.hpp>
#include <godot_cpp/variant/array.hpp>
#include <godot_cpp/variant/dictionary.hpp>
#include <godot_cpp/variant/string.hpp>

namespace godot {

class AbiTypegenRequest;

class AbiTypegenClient : public Node {
    GDCLASS(AbiTypegenClient, Node)

private:
    String rpc_url_;
    double timeout_seconds_ = 10.0;
    int64_t body_size_limit_ = 16 * 1024 * 1024;

protected:
    static void _bind_methods();

public:
    void set_rpc_url(const String &value);
    String get_rpc_url() const;

    void set_timeout_seconds(double value);
    double get_timeout_seconds() const;

    void set_body_size_limit(int64_t value);
    int64_t get_body_size_limit() const;

    AbiTypegenRequest *rpc(const String &method, const Array &params = Array());
    AbiTypegenRequest *call_contract(const String &contract_address,
                                     const String &abi_json,
                                     const String &signature,
                                     const Array &args,
                                     const Array &input_types,
                                     const Array &output_types,
                                     const Array &output_names,
                                     const String &block = "latest");
    AbiTypegenRequest *failed_request(const String &code, const String &message);
};

} // namespace godot
