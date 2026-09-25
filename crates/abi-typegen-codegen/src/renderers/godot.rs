//! Godot 4 GDScript bindings backed by the shared abi-typegen C runtime.
//!
//! Generated GDScript contains ABI metadata, lossless Godot type descriptors,
//! and thin calls into the reusable AbiTypegenCodec/AbiTypegenClient GDExtension.

use crate::naming::{Scope, exported, overload_indices, param_names, repeat_indices};
use crate::tuples::TupleRegistry;
use abi_typegen_core::types::{AbiParam, ContractIr, SolType, StateMutability};
use heck::{ToShoutySnakeCase, ToSnakeCase};
use std::collections::HashSet;

const GODOT_RESERVED: &[&str] = &[
    "and",
    "as",
    "assert",
    "await",
    "break",
    "breakpoint",
    "class",
    "class_name",
    "const",
    "continue",
    "elif",
    "else",
    "enum",
    "extends",
    "false",
    "for",
    "func",
    "if",
    "in",
    "is",
    "match",
    "namespace",
    "not",
    "null",
    "or",
    "pass",
    "preload",
    "return",
    "self",
    "signal",
    "static",
    "super",
    "true",
    "var",
    "void",
    "while",
    "yield",
];

/// Returns the generated GDScript file name.
pub fn file_name(contract_name: &str) -> String {
    let stem = contract_name.to_snake_case();
    let stem = if stem.is_empty() {
        "contract".into()
    } else {
        stem
    };
    format!("{stem}.gd")
}

/// Returns a Godot global class name that cannot shadow engine builtins.
pub fn namespace_name(name: &str) -> String {
    let name = exported(name);
    let name = if name.is_empty() { "Contract" } else { &name };
    format!("AtgContract{name}")
}

fn gd_string(value: &str) -> String {
    // JSON string escaping is accepted by GDScript string literals and avoids
    // handwritten escaping of ABI/NatSpec-derived source text.
    serde_json::to_string(value).expect("string serializes")
}

fn gd_ident(name: &str) -> String {
    let mut ident = name.to_snake_case();
    ident = ident
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if ident.is_empty() {
        ident = "arg".to_string();
    }
    if ident.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        ident.insert(0, '_');
    }
    if GODOT_RESERVED.contains(&ident.as_str()) {
        ident.push('_');
    }
    ident
}

fn names(params: &[AbiParam], registry: &TupleRegistry) -> Vec<String> {
    names_with_reserved(params, registry, &[])
}

fn names_with_reserved(
    params: &[AbiParam],
    registry: &TupleRegistry,
    reserved: &[&str],
) -> Vec<String> {
    let raw = param_names(params.iter().map(|param| {
        (
            param.name.as_str(),
            &param.ty,
            registry.name_of_type(&param.ty, param.internal_type.as_deref()),
        )
    }));
    let mut scope = Scope::with_reserved(reserved.iter().copied());
    raw.into_iter()
        .map(|name| scope.claim(&gd_ident(&name)))
        .collect()
}

fn constant_ident(name: &str) -> String {
    let mut ident = name
        .to_shouty_snake_case()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    if ident.is_empty() {
        ident = "ITEM".to_string();
    }
    if ident.starts_with(|c: char| c.is_ascii_digit()) {
        ident.insert(0, '_');
    }
    ident
}

fn claim_method_stem(used: &mut HashSet<String>, base: &str, roles: &[(&str, &str)]) -> String {
    let mut candidate = base.to_string();
    let mut suffix = 2;
    while roles
        .iter()
        .any(|(prefix, postfix)| used.contains(&format!("{prefix}{candidate}{postfix}")))
    {
        candidate = format!("{base}{suffix}");
        suffix += 1;
    }
    for (prefix, postfix) in roles {
        used.insert(format!("{prefix}{candidate}{postfix}"));
    }
    candidate
}

fn tuple_field_names(components: &[AbiParam], registry: &TupleRegistry) -> Vec<String> {
    names(components, registry)
}

fn type_descriptor(ty: &SolType, internal: Option<&str>, registry: &TupleRegistry) -> String {
    match ty {
        SolType::Bool => r#"{"kind":"bool"}"#.into(),
        SolType::Address => r#"{"kind":"address"}"#.into(),
        SolType::StringType => r#"{"kind":"string"}"#.into(),
        SolType::Uint(bits) => format!(r#"{{"kind":"uint","bits":{bits}}}"#),
        SolType::Int(bits) => format!(r#"{{"kind":"int","bits":{bits}}}"#),
        SolType::Bytes => r#"{"kind":"bytes"}"#.into(),
        SolType::BytesN(size) => format!(r#"{{"kind":"bytes_n","size":{size}}}"#),
        SolType::Array(inner) => format!(
            r#"{{"kind":"array","item":{}}}"#,
            type_descriptor(inner, internal, registry)
        ),
        SolType::FixedArray(inner, size) => format!(
            r#"{{"kind":"fixed_array","size":{size},"item":{}}}"#,
            type_descriptor(inner, internal, registry)
        ),
        SolType::Tuple(components) => {
            let params = components
                .iter()
                .map(|part| AbiParam {
                    name: part.name.clone(),
                    ty: part.ty.clone(),
                    internal_type: part.internal_type.clone(),
                })
                .collect::<Vec<_>>();
            let field_names = tuple_field_names(&params, registry)
                .into_iter()
                .map(|name| gd_string(&name))
                .collect::<Vec<_>>()
                .join(",");
            let items = components
                .iter()
                .map(|part| type_descriptor(&part.ty, part.internal_type.as_deref(), registry))
                .collect::<Vec<_>>()
                .join(",");
            let abi_name = registry.name(components, internal);
            format!(
                r#"{{"kind":"tuple","name":{},"fields":[{field_names}],"items":[{items}]}}"#,
                gd_string(abi_name)
            )
        }
    }
}

fn descriptors(params: &[AbiParam], registry: &TupleRegistry) -> String {
    params
        .iter()
        .map(|param| type_descriptor(&param.ty, param.internal_type.as_deref(), registry))
        .collect::<Vec<_>>()
        .join(",")
}

fn indexed_hash(ty: &SolType) -> bool {
    matches!(
        ty,
        SolType::StringType
            | SolType::Bytes
            | SolType::Array(_)
            | SolType::FixedArray(_, _)
            | SolType::Tuple(_)
    )
}

fn params_decl(names: &[String]) -> String {
    names.join(", ")
}

fn params_values(names: &[String]) -> String {
    format!("[{}]", names.join(", "))
}

fn output_names_literal(params: &[AbiParam], registry: &TupleRegistry) -> String {
    names(params, registry)
        .into_iter()
        .map(|name| gd_string(&name))
        .collect::<Vec<_>>()
        .join(",")
}

fn error_dict(code: &str, message: &str) -> String {
    format!(
        r#"{{"ok":false,"error":{{"code":{},"message":{}}}}}"#,
        gd_string(code),
        gd_string(message)
    )
}

/// Render one standalone GDScript contract wrapper.
///
/// When `wrappers` is false, only ABI/signature/selector/topic metadata and type
/// descriptors are emitted. Callable methods reference the reusable GDExtension
/// only when wrappers are enabled.
pub fn render_godot_file(ir: &ContractIr, wrappers: bool) -> String {
    let registry = TupleRegistry::new(ir);
    let class_name = namespace_name(&ir.name);
    let abi_json = serde_json::to_string(&ir.raw_abi).expect("ABI serializes");

    let mut out = format!(
        "# Generated by abi-typegen. Do not edit.\n# Lossless ABI representations: address=0x hex String, integers=32-byte big-endian PackedByteArray, bytes=PackedByteArray.\nclass_name {class_name}\nextends RefCounted\n\nconst ABI_RUNTIME_VERSION := 1\nconst ABI_JSON := {}\n",
        gd_string(&abi_json)
    );

    let mut constant_scope = Scope::with_reserved([
        "ABI_JSON",
        "ABI_RUNTIME_VERSION",
        "CONSTRUCTOR_SIGNATURE",
        "CONSTRUCTOR_INPUT_TYPES",
        "HAS_FALLBACK",
        "HAS_RECEIVE",
    ]);
    let mut function_meta = Vec::new();
    for (function, index) in ir.functions.iter().zip(overload_indices(&ir.functions)) {
        let suffix = index.map(|i| format!("_{}", i + 1)).unwrap_or_default();
        let base = constant_scope.claim_family(
            &format!("{}{}", constant_ident(&function.name), suffix),
            &[
                "_SIGNATURE",
                "_SELECTOR",
                "_INPUT_TYPES",
                "_OUTPUT_TYPES",
                "_OUTPUT_NAMES",
            ],
        );
        out.push_str(&format!(
            "\nconst {base}_SIGNATURE := {}\nconst {base}_SELECTOR := {}\nconst {base}_INPUT_TYPES := [{}]\nconst {base}_OUTPUT_TYPES := [{}]\nconst {base}_OUTPUT_NAMES := [{}]\n",
            gd_string(&function.signature()),
            gd_string(&function.selector().to_string()),
            descriptors(&function.inputs, &registry),
            descriptors(&function.outputs, &registry),
            output_names_literal(&function.outputs, &registry),
        ));
        function_meta.push((function, base, index));
    }

    let constructor_names = ir
        .constructor
        .as_ref()
        .map(|constructor| names_with_reserved(&constructor.inputs, &registry, &["bytecode"]))
        .unwrap_or_default();
    let constructor_inputs = ir
        .constructor
        .as_ref()
        .map(|constructor| descriptors(&constructor.inputs, &registry))
        .unwrap_or_default();
    out.push_str(&format!(
        "\nconst CONSTRUCTOR_SIGNATURE := {}\nconst CONSTRUCTOR_INPUT_TYPES := [{}]\n",
        gd_string(
            &ir.constructor
                .as_ref()
                .map(|constructor| {
                    format!(
                        "constructor({})",
                        constructor
                            .inputs
                            .iter()
                            .map(|param| param.ty.canonical())
                            .collect::<Vec<_>>()
                            .join(",")
                    )
                })
                .unwrap_or_else(|| "constructor()".to_string())
        ),
        constructor_inputs
    ));
    out.push_str(&format!(
        "const HAS_FALLBACK := {}\nconst HAS_RECEIVE := {}\n",
        if ir.has_fallback { "true" } else { "false" },
        if ir.has_receive { "true" } else { "false" },
    ));

    let mut event_meta = Vec::new();
    for (event, index) in ir.events.iter().zip(repeat_indices(
        ir.events.iter().map(|event| event.name.as_str()),
    )) {
        let suffix = index.map(|i| format!("_{}", i + 1)).unwrap_or_default();
        let base = constant_scope.claim_family(
            &format!("{}{}", constant_ident(&event.name), suffix),
            &[
                "_EVENT_SIGNATURE",
                "_EVENT_TOPIC",
                "_EVENT_TYPES",
                "_EVENT_NAMES",
            ],
        );
        out.push_str(&format!(
            "\nconst {base}_EVENT_SIGNATURE := {}\n",
            gd_string(&event.signature())
        ));
        if !event.anonymous {
            out.push_str(&format!(
                "const {base}_EVENT_TOPIC := {}\n",
                gd_string(&event.topic0().to_string())
            ));
        }
        let params = event
            .inputs
            .iter()
            .map(|param| AbiParam {
                name: param.name.clone(),
                ty: param.ty.clone(),
                internal_type: param.internal_type.clone(),
            })
            .collect::<Vec<_>>();
        out.push_str(&format!(
            "const {base}_EVENT_TYPES := [{}]\nconst {base}_EVENT_NAMES := [{}]\n",
            event
                .inputs
                .iter()
                .map(|param| {
                    if param.indexed && indexed_hash(&param.ty) {
                        r#"{"kind":"bytes_n","size":32}"#.to_string()
                    } else {
                        type_descriptor(&param.ty, param.internal_type.as_deref(), &registry)
                    }
                })
                .collect::<Vec<_>>()
                .join(","),
            output_names_literal(&params, &registry)
        ));
        event_meta.push((event, base, index));
    }

    let mut error_meta = Vec::new();
    for (error, index) in ir.errors.iter().zip(repeat_indices(
        ir.errors.iter().map(|error| error.name.as_str()),
    )) {
        let suffix = index.map(|i| format!("_{}", i + 1)).unwrap_or_default();
        let base = constant_scope.claim_family(
            &format!("{}{}", constant_ident(&error.name), suffix),
            &[
                "_ERROR_SIGNATURE",
                "_ERROR_SELECTOR",
                "_ERROR_TYPES",
                "_ERROR_NAMES",
            ],
        );
        out.push_str(&format!(
            "\nconst {base}_ERROR_SIGNATURE := {}\nconst {base}_ERROR_SELECTOR := {}\nconst {base}_ERROR_TYPES := [{}]\nconst {base}_ERROR_NAMES := [{}]\n",
            gd_string(&error.signature()),
            gd_string(&error.selector().to_string()),
            descriptors(&error.inputs, &registry),
            output_names_literal(&error.inputs, &registry),
        ));
        error_meta.push((error, base, index));
    }

    if !wrappers {
        return out;
    }

    out.push_str(
        "\n# Callable wrappers below require the reusable abi-typegen Godot GDExtension.\n",
    );

    if ir.constructor.is_some() {
        let decl = params_decl(&constructor_names);
        let signature_tail = if decl.is_empty() {
            String::new()
        } else {
            format!(", {decl}")
        };
        out.push_str(&format!(
            r#"
static func encode_constructor(bytecode: PackedByteArray{signature_tail}) -> Dictionary:
    return AbiTypegenCodec.encode_constructor(ABI_JSON, bytecode, {values}, CONSTRUCTOR_INPUT_TYPES)
"#,
            values = params_values(&constructor_names),
        ));
    }

    if ir.has_fallback {
        out.push_str(
            r#"
static func send_fallback(client: AbiTypegenClient, _signer: Object, _contract_address: String, _data: PackedByteArray = PackedByteArray(), _options: Dictionary = {}):
    if client == null:
        return null
    return client.failed_request("unsupported_fallback", "generic fallback transaction wrappers are not implemented in the first-pass Godot backend")
"#,
        );
    }
    if ir.has_receive {
        out.push_str(
            r#"
static func send_receive(client: AbiTypegenClient, _signer: Object, _contract_address: String, _options: Dictionary = {}):
    if client == null:
        return null
    return client.failed_request("unsupported_receive", "generic receive transaction wrappers are not implemented in the first-pass Godot backend")
"#,
        );
    }

    let mut used_methods = HashSet::from([
        "encode_constructor".to_string(),
        "send_fallback".to_string(),
        "send_receive".to_string(),
    ]);
    for (function, base, overload_index) in function_meta {
        let raw_stem = match overload_index {
            Some(index) => format!("{}_{}", gd_ident(&function.name), index + 1),
            None => gd_ident(&function.name),
        };
        let action = if matches!(
            function.state_mutability,
            StateMutability::View | StateMutability::Pure
        ) {
            "call_"
        } else {
            "send_"
        };
        let stem = claim_method_stem(
            &mut used_methods,
            &raw_stem,
            &[("encode_", ""), ("decode_", "_result"), (action, "")],
        );
        let arg_names = names_with_reserved(
            &function.inputs,
            &registry,
            &[
                "client",
                "contract_address",
                "block",
                "signer",
                "options",
                "encoded",
                "tx",
                "encoded_bytes",
                "request",
            ],
        );
        let decl = params_decl(&arg_names);
        let values = params_values(&arg_names);
        let args_with_prefix = if decl.is_empty() {
            String::new()
        } else {
            format!(", {decl}")
        };

        out.push_str(&format!(
            r#"
static func encode_{stem}({decl}) -> Dictionary:
    return AbiTypegenCodec.encode_call(ABI_JSON, {base}_SIGNATURE, {values}, {base}_INPUT_TYPES)

static func decode_{stem}_result(data: PackedByteArray) -> Dictionary:
    return AbiTypegenCodec.decode_call(ABI_JSON, {base}_SIGNATURE, data, {base}_OUTPUT_TYPES, {base}_OUTPUT_NAMES)
"#
        ));

        match function.state_mutability {
            StateMutability::View | StateMutability::Pure => {
                out.push_str(&format!(
                    r#"
static func call_{stem}(client: AbiTypegenClient, contract_address: String{args_with_prefix}, block: String = "latest"):
    if client == null:
        push_error("abi-typegen: call_{stem} requires an AbiTypegenClient")
        return null
    return client.call_contract(contract_address, ABI_JSON, {base}_SIGNATURE, {values}, {base}_INPUT_TYPES, {base}_OUTPUT_TYPES, {base}_OUTPUT_NAMES, block)
"#
                ));
            }
            StateMutability::NonPayable | StateMutability::Payable => {
                let nonpayable = function.state_mutability == StateMutability::NonPayable;
                let value_guard = if nonpayable {
                    r#"
    if tx.has("value") and not AbiTypegenCodec.is_zero_integer_string(str(tx["value"])):
        return client.failed_request("nonpayable_value", "nonpayable function cannot receive value")"#
                } else {
                    ""
                };
                out.push_str(&format!(
                    r#"
static func send_{stem}(client: AbiTypegenClient, signer: Object, contract_address: String{args_with_prefix}, options: Dictionary = {{}}):
    if client == null:
        push_error("abi-typegen: send_{stem} requires an AbiTypegenClient")
        return null
    if signer == null or not signer.has_method("send_transaction"):
        return client.failed_request("missing_signer", "signer must implement send_transaction(client, tx)")
    if not AbiTypegenCodec.is_address_hex(contract_address):
        return client.failed_request("invalid_address", "contract address must be exactly 20 bytes of 0x-prefixed hex")
    var encoded := encode_{stem}({call_args})
    if not encoded.get("ok", false):
        return client.failed_request("encode_failed", str(encoded.get("error", {{}})))
    var tx := options.duplicate(true)
    tx["to"] = contract_address
    var encoded_bytes: PackedByteArray = encoded["value"]
    tx["data"] = "0x" + encoded_bytes.hex_encode()
    if not tx.has("value"):
        tx["value"] = "0x0"{value_guard}
    var request = signer.call("send_transaction", client, tx)
    if request == null or not (request is AbiTypegenRequest):
        return client.failed_request("signer_contract", "signer must return AbiTypegenRequest")
    return request
"#,
                    call_args = arg_names.join(", "),
                ));
            }
        }
    }

    for (event, base, repeat_index) in event_meta {
        let raw_stem = match repeat_index {
            Some(index) => format!("{}_{}", gd_ident(&event.name), index + 1),
            None => gd_ident(&event.name),
        };
        let stem = claim_method_stem(
            &mut used_methods,
            &raw_stem,
            &[("decode_", "_event"), ("", "_event_filter")],
        );
        let indexed_count = event.inputs.iter().filter(|input| input.indexed).count();
        let initial_topic = if event.anonymous {
            "[]".to_string()
        } else {
            format!("[{base}_EVENT_TOPIC]")
        };
        out.push_str(&format!(
            r#"
static func decode_{stem}_event(topics: Array, data: PackedByteArray) -> Dictionary:
    return AbiTypegenCodec.decode_event(ABI_JSON, {base}_EVENT_SIGNATURE, topics, data, {base}_EVENT_TYPES, {base}_EVENT_NAMES)

# Indexed topics are positional. Supply null for a wildcard or a pre-encoded 32-byte 0x topic.
# Dynamic indexed values must be supplied as their already-computed topic hash.
static func {stem}_event_filter(contract_address: String = "", indexed_topics: Array = []) -> Dictionary:
    if indexed_topics.size() > {indexed_count}:
        return {too_many}
    var topics: Array = {initial_topic}
    for topic in indexed_topics:
        if topic == null:
            topics.append(null)
        elif topic is String and AbiTypegenCodec.is_topic_hex(topic):
            topics.append(topic)
        else:
            return {bad_topic}
    var filter := {{"topics": topics}}
    if not contract_address.is_empty():
        if not AbiTypegenCodec.is_address_hex(contract_address):
            return {bad_address}
        filter["address"] = contract_address
    return {{"ok": true, "value": filter}}
"#,
            too_many = error_dict("too_many_topics", "too many indexed topics for event"),
            bad_topic = error_dict("invalid_topic", "event topic must be null or a 32-byte 0x hex string"),
            bad_address = error_dict("invalid_address", "event filter address must be exactly 20 bytes of 0x-prefixed hex"),
        ));
    }

    for (error, base, repeat_index) in error_meta {
        let raw_stem = match repeat_index {
            Some(index) => format!("{}_{}", gd_ident(&error.name), index + 1),
            None => gd_ident(&error.name),
        };
        let stem = claim_method_stem(&mut used_methods, &raw_stem, &[("decode_", "_error")]);
        out.push_str(&format!(
            r#"
static func decode_{stem}_error(data: PackedByteArray) -> Dictionary:
    return AbiTypegenCodec.decode_error(ABI_JSON, {base}_ERROR_SIGNATURE, data, {base}_ERROR_TYPES, {base}_ERROR_NAMES)
"#
        ));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use abi_typegen_core::parser::parse_artifact;

    #[test]
    fn metadata_and_wrappers_are_deterministic() {
        let ir = parse_artifact(
            "Token",
            r#"{"abi":[
                {"type":"function","name":"balanceOf","inputs":[{"name":"owner","type":"address"}],"outputs":[{"name":"amount","type":"uint256"}],"stateMutability":"view"},
                {"type":"function","name":"f","inputs":[{"name":"x","type":"uint256"}],"outputs":[],"stateMutability":"pure"},
                {"type":"function","name":"f","inputs":[{"name":"x","type":"address"}],"outputs":[],"stateMutability":"pure"},
                {"type":"event","name":"Changed","inputs":[{"name":"owner","type":"address","indexed":true},{"name":"amount","type":"uint256","indexed":false}],"anonymous":false},
                {"type":"error","name":"Denied","inputs":[{"name":"owner","type":"address"}]}
            ]}"#,
        )
        .expect("fixture");
        let a = render_godot_file(&ir, true);
        let b = render_godot_file(&ir, true);
        assert_eq!(a, b);
        assert!(a.contains("class_name AtgContractToken"), "{a}");
        assert!(
            a.contains("BALANCE_OF_SIGNATURE := \"balanceOf(address)\""),
            "{a}"
        );
        assert!(a.contains("static func encode_balance_of(owner)"), "{a}");
        assert!(a.contains("static func encode_f_1(x)"), "{a}");
        assert!(a.contains("static func encode_f_2(x)"), "{a}");
        assert!(a.contains("static func decode_changed_event"), "{a}");
        assert!(a.contains("static func decode_denied_error"), "{a}");
    }

    #[test]
    fn descriptors_cover_tuples_arrays_and_large_ints() {
        let ir = parse_artifact(
            "Cases",
            r#"{"abi":[{"type":"function","name":"store","inputs":[
                {"name":"amount","type":"uint256"},
                {"name":"rows","type":"uint256[][]"},
                {"name":"item","type":"tuple","internalType":"struct Cases.Item","components":[{"name":"owner","type":"address"},{"name":"delta","type":"int256"}]}
            ],"outputs":[],"stateMutability":"nonpayable"}]}"#,
        )
        .expect("fixture");
        let out = render_godot_file(&ir, true);
        assert!(out.contains("\"kind\":\"uint\",\"bits\":256"), "{out}");
        assert!(out.contains("\"kind\":\"array\""), "{out}");
        assert!(out.contains("\"kind\":\"tuple\""), "{out}");
        assert!(out.contains("\"kind\":\"int\",\"bits\":256"), "{out}");
    }

    #[test]
    fn metadata_only_omits_callable_runtime_references() {
        let ir = parse_artifact(
            "Token",
            r#"{"abi":[{"type":"function","name":"balanceOf","inputs":[{"name":"owner","type":"address"}],"outputs":[{"name":"amount","type":"uint256"}],"stateMutability":"view"}]}"#,
        )
        .expect("fixture");
        let out = render_godot_file(&ir, false);
        assert!(out.contains("ABI_JSON"), "{out}");
        assert!(out.contains("BALANCE_OF_SELECTOR"), "{out}");
        assert!(!out.contains("AbiTypegenCodec"), "{out}");
        assert!(!out.contains("static func encode_balance_of"), "{out}");
    }

    #[test]
    fn file_and_namespace_names_are_stable() {
        assert_eq!(file_name("TupleCases"), "tuple_cases.gd");
        assert_eq!(file_name("ERC20"), "erc20.gd");
        assert_eq!(namespace_name("String"), "AtgContractString");
        assert_eq!(namespace_name("Vector2"), "AtgContractVector2");
        assert_eq!(namespace_name("Token"), "AtgContractToken");
    }
    #[test]
    fn duplicate_output_names_are_allocated_once_and_remain_stable() {
        let ir = parse_artifact(
            "Dupes",
            r#"{"abi":[{"type":"function","name":"f","inputs":[],"outputs":[{"name":"value","type":"uint256"},{"name":"value","type":"uint256"}],"stateMutability":"view"}]}"#,
        )
        .expect("fixture");
        let out = render_godot_file(&ir, true);
        assert!(
            out.contains("F_OUTPUT_NAMES := [\"value\",\"value2\"]"),
            "{out}"
        );
    }

    #[test]
    fn abi_arguments_do_not_shadow_generated_wrapper_parameters() {
        let ir = parse_artifact(
            "Collisions",
            r#"{"abi":[
                {"type":"constructor","inputs":[{"name":"bytecode","type":"bytes"}]},
                {"type":"function","name":"read","inputs":[{"name":"client","type":"address"},{"name":"contract_address","type":"address"},{"name":"block","type":"uint256"}],"outputs":[],"stateMutability":"view"},
                {"type":"function","name":"write","inputs":[{"name":"signer","type":"address"},{"name":"options","type":"uint256"}],"outputs":[],"stateMutability":"nonpayable"}
            ]}"#,
        )
        .expect("fixture");
        let out = render_godot_file(&ir, true);
        assert!(
            out.contains("encode_constructor(bytecode: PackedByteArray, bytecode2)"),
            "{out}"
        );
        assert!(out.contains("call_read(client: AbiTypegenClient, contract_address: String, client2, contract_address2, block2, block: String"), "{out}");
        assert!(out.contains("send_write(client: AbiTypegenClient, signer: Object, contract_address: String, signer2, options2, options: Dictionary"), "{out}");
    }

    #[test]
    fn generated_helper_names_are_not_redeclared_by_abi_functions() {
        let ir = parse_artifact(
            "Helpers",
            r#"{"abi":[
                {"type":"constructor","inputs":[]},
                {"type":"function","name":"constructor","inputs":[],"outputs":[],"stateMutability":"pure"},
                {"type":"fallback","stateMutability":"payable"},
                {"type":"function","name":"fallback","inputs":[],"outputs":[],"stateMutability":"nonpayable"}
            ]}"#,
        )
        .expect("fixture");
        let out = render_godot_file(&ir, true);
        assert_eq!(
            out.matches("static func encode_constructor(").count(),
            1,
            "{out}"
        );
        assert_eq!(
            out.matches("static func send_fallback(").count(),
            1,
            "{out}"
        );
    }

    #[test]
    fn event_filter_names_do_not_shadow_function_encoders() {
        let ir = parse_artifact(
            "CrossKinds",
            r#"{"abi":[
                {"type":"function","name":"xEventFilter","inputs":[],"outputs":[],"stateMutability":"view"},
                {"type":"event","name":"encodeX","inputs":[],"anonymous":false}
            ]}"#,
        )
        .expect("fixture");
        let out = render_godot_file(&ir, true);
        assert_eq!(
            out.matches("static func encode_x_event_filter(").count(),
            1,
            "{out}"
        );
        assert!(out.contains("static func encode_x2_event_filter("), "{out}");
    }
}
