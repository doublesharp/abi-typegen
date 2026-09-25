//! Optional web3swift-backed contract operations.

use super::{Field, Imports, escape_keyword, render_struct, sol_type_to_swift, swift_string};
use crate::naming::{Scope, exported, param_names, repeat_indices};
use crate::tuples::TupleRegistry;
use abi_typegen_core::types::{AbiParam, ContractIr, SolType, StateMutability};
use std::collections::HashMap;

fn fields(
    params: &[AbiParam],
    registry: &TupleRegistry,
    tuples: &HashMap<String, String>,
) -> Vec<Field> {
    let names = param_names(params.iter().map(|p| {
        (
            p.name.as_str(),
            &p.ty,
            registry.name_of_type(&p.ty, p.internal_type.as_deref()),
        )
    }));
    let mut scope = Scope::with_reserved(["_", "self"]);
    params
        .iter()
        .zip(names)
        .map(|(p, name)| Field {
            name: scope.claim(&name),
            ty: sol_type_to_swift(
                &p.ty,
                p.internal_type.as_deref(),
                registry,
                tuples,
                &mut Imports::default(),
            ),
        })
        .collect()
}

fn tuple_name<'a>(
    ty: &SolType,
    internal: Option<&str>,
    registry: &TupleRegistry,
    tuples: &'a HashMap<String, String>,
) -> &'a str {
    &tuples[registry.name(
        match ty {
            SolType::Tuple(parts) => parts,
            _ => unreachable!("tuple only"),
        },
        internal,
    )]
}

fn encode(
    expr: &str,
    ty: &SolType,
    internal: Option<&str>,
    registry: &TupleRegistry,
    tuples: &HashMap<String, String>,
) -> String {
    match ty {
        SolType::Array(inner) | SolType::FixedArray(inner, _) => format!(
            "{expr}.map {{ item in {} }}",
            encode("item", inner, internal, registry, tuples)
        ),
        SolType::Tuple(_) => format!(
            "_encode{}({expr})",
            tuple_name(ty, internal, registry, tuples)
        ),
        _ => expr.to_string(),
    }
}

fn decode(
    expr: &str,
    ty: &SolType,
    internal: Option<&str>,
    registry: &TupleRegistry,
    tuples: &HashMap<String, String>,
) -> String {
    match ty {
        SolType::Array(inner) | SolType::FixedArray(inner, _) => format!(
            "try _array({expr}) {{ item in {} }}",
            decode("item", inner, internal, registry, tuples)
        ),
        SolType::Tuple(_) => format!(
            "try _decode{}({expr})",
            tuple_name(ty, internal, registry, tuples)
        ),
        _ => {
            let swift = sol_type_to_swift(ty, internal, registry, tuples, &mut Imports::default());
            format!("try _cast({expr}, as: {swift}.self)")
        }
    }
}

fn values_expr(
    fields: &[Field],
    params: &[AbiParam],
    registry: &TupleRegistry,
    tuples: &HashMap<String, String>,
    prefix: &str,
) -> String {
    params
        .iter()
        .zip(fields)
        .map(|(p, f)| {
            encode(
                &format!("{prefix}.{}", escape_keyword(&f.name)),
                &p.ty,
                p.internal_type.as_deref(),
                registry,
                tuples,
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn validate(
    expr: &str,
    ty: &SolType,
    internal: Option<&str>,
    registry: &TupleRegistry,
    tuples: &HashMap<String, String>,
    indent: &str,
) -> String {
    match ty {
        SolType::Uint(bits) => format!(
            "{indent}guard {expr} < (BigUInt(1) << {bits}) else {{ throw WrapperError.encodingFailed(\"uint{bits} out of range\") }}\n"
        ),
        SolType::Int(bits) => format!(
            "{indent}guard {expr} >= -(BigInt(1) << {}) && {expr} < (BigInt(1) << {}) else {{ throw WrapperError.encodingFailed(\"int{bits} out of range\") }}\n",
            bits - 1,
            bits - 1
        ),
        SolType::BytesN(len) => format!(
            "{indent}guard {expr}.count == {len} else {{ throw WrapperError.encodingFailed(\"bytes{len} length\") }}\n"
        ),
        SolType::FixedArray(inner, len) => {
            let nested = validate(
                "item",
                inner,
                internal,
                registry,
                tuples,
                &format!("{indent}    "),
            );
            let count = format!(
                "{indent}guard {expr}.count == {len} else {{ throw WrapperError.encodingFailed(\"fixed array length {len}\") }}\n"
            );
            if nested.is_empty() {
                count
            } else {
                format!("{count}{indent}for item in {expr} {{\n{nested}{indent}}}\n")
            }
        }
        SolType::Array(inner) => {
            let nested = validate(
                "item",
                inner,
                internal,
                registry,
                tuples,
                &format!("{indent}    "),
            );
            if nested.is_empty() {
                String::new()
            } else {
                format!("{indent}for item in {expr} {{\n{nested}{indent}}}\n")
            }
        }
        SolType::Tuple(_) => format!(
            "{indent}try _validate{}({expr})\n",
            tuple_name(ty, internal, registry, tuples)
        ),
        SolType::Bool | SolType::StringType | SolType::Address | SolType::Bytes => String::new(),
    }
}

fn unsupported_indexed(ty: &SolType) -> bool {
    matches!(
        ty,
        SolType::Bytes
            | SolType::StringType
            | SolType::Array(_)
            | SolType::FixedArray(_, _)
            | SolType::Tuple(_)
    )
}

pub(super) fn render(
    ir: &ContractIr,
    registry: &TupleRegistry,
    tuples: &HashMap<String, String>,
    params_names: &[Option<String>],
    event_names: &[String],
    error_names: &[String],
    scope: &mut Scope,
) -> String {
    let mut out = String::new();
    out.push_str("\n    /// Errors from ABI encoding and typed decoding.\n    public enum WrapperError: Error {\n        /// The SDK could not encode a value.\n        case encodingFailed(String)\n        /// The decoded value differs from the ABI type.\n        case invalidValue(String)\n        /// The SDK cannot represent this ABI shape.\n        case unsupported(String)\n        /// The log does not match this event.\n        case eventMismatch\n    }\n");
    out.push_str("\n    private static func _cast<T>(_ raw: Any, as type: T.Type) throws -> T {\n        guard let value = raw as? T else { throw WrapperError.invalidValue(String(describing: type)) }\n        return value\n    }\n    private static func _array<T>(_ raw: Any, _ convert: (Any) throws -> T) throws -> [T] {\n        guard let values = raw as? [Any] else { throw WrapperError.invalidValue(\"array\") }\n        return try values.map(convert)\n    }\n    private static func _contract() throws -> EthereumContract { try EthereumContract(abi) }\n");

    for def in registry.defs() {
        let name = &tuples[&def.name];
        let params: Vec<AbiParam> = def
            .components
            .iter()
            .map(|p| AbiParam {
                name: p.name.clone(),
                ty: p.ty.clone(),
                internal_type: p.internal_type.clone(),
            })
            .collect();
        let fs = fields(&params, registry, tuples);
        out.push_str(&format!(
            "\n    private static func _encode{name}(_ value: {name}) -> [Any] {{ [{}] }}\n",
            values_expr(&fs, &params, registry, tuples, "value")
        ));
        out.push_str(&format!(
            "    private static func _validate{name}(_ value: {name}) throws {{\n"
        ));
        let checks = params
            .iter()
            .zip(&fs)
            .map(|(p, f)| {
                validate(
                    &format!("value.{}", escape_keyword(&f.name)),
                    &p.ty,
                    p.internal_type.as_deref(),
                    registry,
                    tuples,
                    "        ",
                )
            })
            .collect::<String>();
        if checks.is_empty() {
            out.push_str("        _ = value\n");
        } else {
            out.push_str(&checks);
        }
        out.push_str("    }\n");
        out.push_str(&format!("    private static func _decode{name}(_ raw: Any) throws -> {name} {{\n        guard let values = raw as? [Any], values.count == {} else {{ throw WrapperError.invalidValue({}) }}\n        return {name}(", fs.len(), swift_string(name)));
        out.push_str(
            &params
                .iter()
                .zip(&fs)
                .enumerate()
                .map(|(i, (p, f))| {
                    format!(
                        "{}: {}",
                        f.name,
                        decode(
                            &format!("values[{i}]"),
                            &p.ty,
                            p.internal_type.as_deref(),
                            registry,
                            tuples
                        )
                    )
                })
                .collect::<Vec<_>>()
                .join(", "),
        );
        out.push_str(")\n    }\n");
    }

    let function_suffixes = repeat_indices(ir.functions.iter().map(|f| f.name.as_str()));
    let mut read_methods = String::new();
    let mut write_methods = String::new();
    for ((function, suffix), params_name) in
        ir.functions.iter().zip(function_suffixes).zip(params_names)
    {
        let suffix = suffix.map(|n| n.to_string()).unwrap_or_default();
        let stem = format!("{}{}", exported(&function.name), suffix);
        let signature = swift_string(&function.signature());
        let fs = fields(&function.inputs, registry, tuples);
        let arg = params_name
            .as_ref()
            .map(|name| format!("_ args: {name}"))
            .unwrap_or_default();
        let args_expr = if params_name.is_some() {
            values_expr(&fs, &function.inputs, registry, tuples, "args")
        } else {
            String::new()
        };
        let args_call = if params_name.is_some() { "args" } else { "" };
        let checks = function
            .inputs
            .iter()
            .zip(&fs)
            .map(|(p, f)| {
                validate(
                    &format!("args.{}", escape_keyword(&f.name)),
                    &p.ty,
                    p.internal_type.as_deref(),
                    registry,
                    tuples,
                    "        ",
                )
            })
            .collect::<String>();
        out.push_str(&format!("\n    /// ABI-encodes `{}` for an offline transaction.\n    public static func encode{stem}({arg}) throws -> Data {{\n{checks}        guard let data = try _contract().method({signature}, parameters: [{args_expr}], extraData: nil) else {{ throw WrapperError.encodingFailed({signature}) }}\n        return data\n    }}\n", function.signature()));
        let result_name = format!("{}Result", scope.claim_family(&stem, &["Result"]));
        let output_fields = fields(&function.outputs, registry, tuples);
        out.push('\n');
        out.push_str(&render_struct(
            &result_name,
            &format!("Decoded result of `{}`.", function.signature()),
            function.natspec.as_ref(),
            &output_fields,
        ));
        out.push_str(&format!("\n    private static func _decode{stem}Values(_ values: [String: Any]) throws -> {result_name} {{\n"));
        for (i, (p, f)) in function.outputs.iter().zip(&output_fields).enumerate() {
            out.push_str(&format!("        guard let raw{i} = values[\"{i}\"] else {{ throw WrapperError.invalidValue({}) }}\n        let value{i} = {}\n", swift_string(&format!("output {i}")), decode(&format!("raw{i}"), &p.ty, p.internal_type.as_deref(), registry, tuples)));
            let _ = f;
        }
        out.push_str(&format!(
            "        return {result_name}({})\n    }}\n",
            output_fields
                .iter()
                .enumerate()
                .map(|(i, f)| format!("{}: value{i}", f.name))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        out.push_str(&format!("\n    /// Decodes raw return data for `{}` offline.\n    public static func decode{stem}Result(_ data: Data) throws -> {result_name} {{\n", function.signature()));
        if function.outputs.is_empty() {
            out.push_str("        guard data.isEmpty else { throw WrapperError.invalidValue(\"unexpected return bytes\") }\n");
            out.push_str(&format!("        return {result_name}()\n    }}\n"));
        } else {
            out.push_str(&format!("        return try _decode{stem}Values(_contract().decodeReturnData({signature}, data: data))\n    }}\n"));
        }
        match function.state_mutability {
            StateMutability::View | StateMutability::Pure => read_methods.push_str(&format!("\n        /// Calls `{}` and decodes its result.\n        public func read{stem}({arg}{comma}transaction: CodableTransaction? = nil) async throws -> {result_name} {{\n            var tx = transaction ?? CodableTransaction(to: address, data: Data())\n            tx.to = address\n            tx.data = try {name}.encode{stem}({args_call})\n            let operation = ReadOperation(transaction: tx, web3: web3, contract: contract, method: {signature})\n            return try {name}._decode{stem}Values(await operation.callContractMethod())\n        }}\n", function.signature(), comma = if arg.is_empty() { "" } else { ", " }, name = super::namespace_name(&ir.name))),
            StateMutability::NonPayable | StateMutability::Payable => {
                let payable = matches!(function.state_mutability, StateMutability::Payable);
                let value_arg = if payable { ", value: BigUInt = 0" } else { "" };
                let value_expr = if payable { "value" } else { "0" };
                let value_guard = if payable { "" } else { "            guard tx.value == 0 else { throw WrapperError.encodingFailed(\"nonpayable value\") }\n" };
                write_methods.push_str(&format!("\n        /// Prepares `{}` with transaction options for signing or submission.\n        public func prepare{stem}({arg}{comma}transaction: CodableTransaction? = nil{value_arg}) throws -> WriteOperation {{\n            var tx = transaction ?? CodableTransaction(to: address, data: Data())\n{value_guard}            tx.to = address\n            tx.data = try {name}.encode{stem}({args_call})\n            tx.value = {value_expr}\n            return WriteOperation(transaction: tx, web3: web3, contract: contract, method: {signature})\n        }}\n", function.signature(), comma = if arg.is_empty() { "" } else { ", " }, name = super::namespace_name(&ir.name)));
            }
        }
    }

    out.push_str(&format!("\n    /// A web3swift contract instance for typed reads and writes.\n    public struct Client {{\n        private let web3: Web3\n        private let contract: EthereumContract\n        private let address: EthereumAddress\n\n        /// Binds this ABI to a provider and deployed address.\n        public init(web3: Web3, at address: EthereumAddress) throws {{\n            self.web3 = web3\n            self.address = address\n            self.contract = try EthereumContract({}.abi, at: address)\n        }}\n{read_methods}{write_methods}    }}\n", super::namespace_name(&ir.name)));

    let event_suffixes = repeat_indices(ir.events.iter().map(|e| e.name.as_str()));
    for ((event, suffix), type_name) in ir.events.iter().zip(event_suffixes).zip(event_names) {
        let suffix = suffix.map(|n| n.to_string()).unwrap_or_default();
        let stem = format!("{}{}", exported(&event.name), suffix);
        let signature = swift_string(&event.signature());
        out.push_str(&format!("\n    private static func _event{stem}() throws -> ABI.Element.Event {{\n        guard let event = try _contract().abi.compactMap({{ element -> ABI.Element.Event? in\n            if case .event(let value) = element {{ return value }}\n            return nil\n        }}).first(where: {{ $0.signature == {signature} }}) else {{ throw WrapperError.invalidValue({signature}) }}\n        return event\n    }}\n"));
        let indexed: Vec<_> = event
            .inputs
            .iter()
            .enumerate()
            .filter(|(_, p)| p.indexed)
            .collect();
        let hashed = indexed.iter().any(|(_, p)| unsupported_indexed(&p.ty));
        let filters = indexed
            .iter()
            .map(|(i, p)| {
                let ty = if unsupported_indexed(&p.ty) {
                    "Data".to_string()
                } else {
                    sol_type_to_swift(
                        &p.ty,
                        p.internal_type.as_deref(),
                        registry,
                        tuples,
                        &mut Imports::default(),
                    )
                };
                format!("filter{i}: {ty}? = nil")
            })
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("\n    /// Builds indexed topics for `{}`. Reference values use their 32-byte topic hashes.\n    public static func filter{stem}({filters}) throws -> [EventFilterParameters.Topic?] {{\n", event.signature()));
        if event.anonymous && indexed.is_empty() {
            out.push_str("        return []\n");
        } else {
            out.push_str(&format!("        let event = try _event{stem}()\n"));
            if event.anonymous {
                out.push_str("        var topics: [EventFilterParameters.Topic?] = []\n");
            } else {
                out.push_str(&format!("        {} topics: [EventFilterParameters.Topic?] = [.string(\"0x\" + event.topic.map {{ String(format: \"%02x\", $0) }}.joined())]\n", if indexed.is_empty() { "let" } else { "var" }));
            }
            for (i, p) in &indexed {
                if unsupported_indexed(&p.ty) {
                    out.push_str(&format!("        if let value = filter{i} {{\n            guard value.count == 32 else {{ throw WrapperError.encodingFailed(\"indexed topic hash must be 32 bytes\") }}\n            topics.append(.string(\"0x\" + value.map {{ String(format: \"%02x\", $0) }}.joined()))\n        }} else {{ topics.append(nil) }}\n"));
                } else {
                    let checks = validate(
                        "value",
                        &p.ty,
                        p.internal_type.as_deref(),
                        registry,
                        tuples,
                        "            ",
                    );
                    out.push_str(&format!("        if let value = filter{i} {{\n{checks}            guard let topic = ABI.Element.Event.encodeTopic(input: event.inputs[{i}], value: value) else {{ throw WrapperError.encodingFailed({signature}) }}\n            topics.append(topic)\n        }} else {{ topics.append(nil) }}\n"));
                }
            }
            out.push_str("        return topics\n");
        }
        out.push_str("    }\n");
        let params: Vec<AbiParam> = event
            .inputs
            .iter()
            .map(|p| AbiParam {
                name: p.name.clone(),
                ty: p.ty.clone(),
                internal_type: p.internal_type.clone(),
            })
            .collect();
        let mut fs = fields(&params, registry, tuples);
        let decoded_name = if hashed {
            for (field, p) in fs.iter_mut().zip(&event.inputs) {
                if p.indexed && unsupported_indexed(&p.ty) {
                    field.ty = "Data".to_string();
                }
            }
            let name = format!("Decoded{stem}Event");
            out.push('\n');
            out.push_str(&render_struct(
                &name,
                &format!(
                    "Decoded `{}` event. Indexed reference fields are 32-byte topic hashes.",
                    event.signature()
                ),
                event.natspec.as_ref(),
                &fs,
            ));
            name
        } else {
            type_name.clone()
        };
        out.push_str(&format!("\n    /// Decodes a `{}` log.\n    public static func decode{stem}Event(_ log: EventLog) throws -> {decoded_name} {{\n        let event = try _event{stem}()\n        guard log.topics.count == {} else {{ throw WrapperError.eventMismatch }}\n", event.signature(), indexed.len() + if event.anonymous { 0 } else { 1 }));
        if !event.anonymous {
            out.push_str("        guard log.topics.first == event.topic else { throw WrapperError.eventMismatch }\n");
        }
        if event.inputs.iter().any(|p| !p.indexed) {
            out.push_str(&format!("        guard let decoded = ABIDecoder.decode(types: event.inputs.filter {{ !$0.indexed }}.map {{ $0.type }}, data: log.data) else {{ throw WrapperError.invalidValue({signature}) }}\n"));
        } else {
            out.push_str("        guard log.data.isEmpty else { throw WrapperError.invalidValue(\"unexpected event data\") }\n");
        }
        let mut topic_index = if event.anonymous { 0 } else { 1 };
        let mut data_index = 0;
        for (i, p) in event.inputs.iter().enumerate() {
            if p.indexed {
                out.push_str(&format!("        let topic{i} = log.topics[{topic_index}]\n        guard topic{i}.count == 32 else {{ throw WrapperError.invalidValue(\"event topic {i}\") }}\n"));
                if unsupported_indexed(&p.ty) {
                    out.push_str(&format!("        let value{i} = topic{i}\n"));
                } else {
                    out.push_str(&format!("        guard let raw{i} = ABIDecoder.decode(types: [event.inputs[{i}].type], data: topic{i})?.first else {{ throw WrapperError.invalidValue(\"event field {i}\") }}\n        let value{i} = {}\n", decode(&format!("raw{i}"), &p.ty, p.internal_type.as_deref(), registry, tuples)));
                }
                topic_index += 1;
            } else {
                out.push_str(&format!(
                    "        let value{i} = {}\n",
                    decode(
                        &format!("decoded[{data_index}]"),
                        &p.ty,
                        p.internal_type.as_deref(),
                        registry,
                        tuples
                    )
                ));
                data_index += 1;
            }
        }
        if event.inputs.is_empty() {
            out.push_str(&format!("        return {decoded_name}()\n"));
        } else {
            out.push_str(&format!(
                "        return {decoded_name}({})\n",
                fs.iter()
                    .enumerate()
                    .map(|(i, f)| format!("{}: value{i}", f.name))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        out.push_str("    }\n");
    }

    out.push_str("\n    /// A decoded custom ABI error.\n    public enum DecodedCustomError {\n");
    let error_suffixes = repeat_indices(ir.errors.iter().map(|e| e.name.as_str()));
    for ((error, suffix), name) in ir.errors.iter().zip(error_suffixes).zip(error_names) {
        let suffix = suffix.map(|n| n.to_string()).unwrap_or_default();
        out.push_str(&format!(
            "        /// `{}`.\n        case {}({name})\n",
            error.signature(),
            escape_keyword(&format!(
                "{}{}",
                crate::naming::lower_first(&exported(&error.name)),
                suffix
            ))
        ));
    }
    out.push_str("    }\n\n    /// Decodes a custom revert payload, or returns nil for an unknown selector.\n    public static func decodeCustomError(_ data: Data) throws -> DecodedCustomError? {\n        guard data.count >= 4 else { throw WrapperError.invalidValue(\"error selector\") }\n");
    if !ir.errors.is_empty() {
        out.push_str(
            "        let contract = try _contract()\n        let selector = Data(data.prefix(4))\n",
        );
    }
    let error_suffixes = repeat_indices(ir.errors.iter().map(|e| e.name.as_str()));
    for ((error, suffix), name) in ir.errors.iter().zip(error_suffixes).zip(error_names) {
        let suffix = suffix.map(|n| n.to_string()).unwrap_or_default();
        let case_name = escape_keyword(&format!(
            "{}{}",
            crate::naming::lower_first(&exported(&error.name)),
            suffix
        ));
        let signature = swift_string(&error.signature());
        let decoded = if error.inputs.is_empty() {
            "error.decodeEthError(Data(data.dropFirst(4))) != nil"
        } else {
            "let values = error.decodeEthError(Data(data.dropFirst(4)))"
        };
        out.push_str(&format!("        if selector == {} {{\n            guard let error = contract.abi.compactMap({{ element -> ABI.Element.EthError? in\n                if case .error(let value) = element {{ return value }}\n                return nil\n            }}).first(where: {{ $0.signature == {signature} }}), {decoded} else {{ throw WrapperError.invalidValue({signature}) }}\n", super::swift_data(error.selector().as_slice())));
        let fs = fields(&error.inputs, registry, tuples);
        for (i, p) in error.inputs.iter().enumerate() {
            out.push_str(&format!("            guard let raw{i} = values[\"{i}\"] else {{ throw WrapperError.invalidValue(\"error field {i}\") }}\n            let value{i} = {}\n", decode(&format!("raw{i}"), &p.ty, p.internal_type.as_deref(), registry, tuples)));
        }
        out.push_str(&format!(
            "            return .{case_name}({name}({}))\n        }}\n",
            fs.iter()
                .enumerate()
                .map(|(i, f)| format!("{}: value{i}", f.name))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    out.push_str("        return nil\n    }\n");
    out
}
