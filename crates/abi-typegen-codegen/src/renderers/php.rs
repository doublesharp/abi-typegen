//! PHP 8.2 bindings with named ABI value types and optional RPC/transaction wrappers.

use crate::naming::{Scope, exported, overload_indices, param_names, repeat_indices};
use crate::tuples::TupleRegistry;
use abi_typegen_core::types::{AbiParam, ContractIr, SolType};
use heck::ToShoutySnakeCase;
use std::collections::{HashMap, HashSet};

mod layout;

/// Returns the public PHP class name for an ABI contract.
pub fn namespace_name(name: &str) -> String {
    let name = exported(name);
    if name.is_empty() {
        "Contract".to_string()
    } else if PHP_RESERVED_CLASS_NAMES
        .iter()
        .chain(PHP_RESERVED_IDENTIFIERS.iter())
        .any(|reserved| reserved.eq_ignore_ascii_case(&name))
    {
        format!("{name}Contract")
    } else {
        name
    }
}

#[derive(Default)]
struct PhpScope {
    used: HashSet<String>,
}

impl PhpScope {
    fn claim(&mut self, base: &str) -> String {
        self.claim_family(base, &["{}"])
    }

    fn claim_family(&mut self, base: &str, patterns: &[&str]) -> String {
        let mut number = 1;
        loop {
            let candidate = if number == 1 {
                base.to_string()
            } else {
                format!("{base}{number}")
            };
            let names = patterns
                .iter()
                .map(|pattern| pattern.replace("{}", &candidate).to_ascii_lowercase())
                .collect::<Vec<_>>();
            if names.iter().all(|name| !self.used.contains(name)) {
                self.used.extend(names);
                return candidate;
            }
            number += 1;
        }
    }
}

fn method_stems(
    ir: &ContractIr,
    registry: &TupleRegistry,
) -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>) {
    let mut scope = PhpScope::default();
    for fixed in [
        "decodeTop",
        "encodeSequence",
        "encodeValue",
        "decodeSequenceAt",
        "decodeValue",
        "layoutDynamic",
        "staticSize",
        "uintWord",
        "intWord",
        "validateIntegerBits",
        "pow2",
        "bigInteger",
        "lengthPrefixedBytes",
        "bytesHex",
        "wordHex",
        "smallWord",
        "ensureRange",
        "checkedAdd",
        "checkedMul",
        "logField",
    ] {
        scope.claim(fixed);
    }
    let tuple_stems = registry
        .defs()
        .iter()
        .map(|def| scope.claim_family(&exported(&def.name), &["encode{}Value", "decode{}Value"]))
        .collect();
    let function_stems = ir
        .functions
        .iter()
        .zip(overload_indices(&ir.functions))
        .map(|(function, index)| {
            let base = format!(
                "{}{}",
                exported(&function.name),
                index.map(|i| i.to_string()).unwrap_or_default()
            );
            let action = match function.state_mutability {
                abi_typegen_core::types::StateMutability::View
                | abi_typegen_core::types::StateMutability::Pure => "call{}",
                abi_typegen_core::types::StateMutability::NonPayable
                | abi_typegen_core::types::StateMutability::Payable => "send{}",
            };
            scope.claim_family(&base, &["encode{}", "decode{}Result", action])
        })
        .collect();
    let event_stems = ir
        .events
        .iter()
        .zip(repeat_indices(
            ir.events.iter().map(|event| event.name.as_str()),
        ))
        .map(|(event, index)| {
            let base = format!(
                "{}{}",
                exported(&event.name),
                index.map(|i| i.to_string()).unwrap_or_default()
            );
            scope.claim_family(&base, &["decode{}Event", "filter{}Event"])
        })
        .collect();
    let error_stems = ir
        .errors
        .iter()
        .zip(repeat_indices(
            ir.errors.iter().map(|error| error.name.as_str()),
        ))
        .map(|(error, index)| {
            let base = format!(
                "{}{}",
                exported(&error.name),
                index.map(|i| i.to_string()).unwrap_or_default()
            );
            scope.claim_family(&base, &["decode{}Error"])
        })
        .collect();
    (tuple_stems, function_stems, event_stems, error_stems)
}

struct ClassNames {
    contract: String,
    tuples: HashMap<String, String>,
    events: Vec<String>,
    decoded_events: Vec<Option<String>>,
    errors: Vec<String>,
    results: Vec<Option<String>>,
    options: String,
    client: String,
}

fn class_names(
    ir: &ContractIr,
    registry: &TupleRegistry,
    function_stems: &[String],
    event_stems: &[String],
    error_stems: &[String],
) -> ClassNames {
    let contract = namespace_name(&ir.name);
    let mut scope = PhpScope::default();
    scope.claim(&contract);
    let tuples = registry
        .defs()
        .iter()
        .map(|def| {
            (
                def.name.clone(),
                scope.claim(&format!("{contract}{}", exported(&def.name))),
            )
        })
        .collect();
    let events = event_stems
        .iter()
        .map(|stem| scope.claim(&format!("{contract}{stem}Event")))
        .collect();
    let decoded_events = ir
        .events
        .iter()
        .zip(event_stems)
        .map(|(event, stem)| {
            event
                .inputs
                .iter()
                .any(|param| param.indexed && indexed_hash(&param.ty))
                .then(|| scope.claim(&format!("{contract}Decoded{stem}Event")))
        })
        .collect();
    let errors = error_stems
        .iter()
        .map(|stem| scope.claim(&format!("{contract}{stem}Error")))
        .collect();
    let results = ir
        .functions
        .iter()
        .zip(function_stems)
        .map(|(function, stem)| {
            (function.outputs.len() > 1).then(|| scope.claim(&format!("{contract}{stem}Result")))
        })
        .collect();
    let options = scope.claim(&format!("{contract}TransactionOptions"));
    let client = scope.claim(&format!("{contract}Client"));
    ClassNames {
        contract,
        tuples,
        events,
        decoded_events,
        errors,
        results,
        options,
        client,
    }
}

/// Returns every global PHP class name emitted for a contract in one namespace.
///
/// PHP resolves class names without regard to case, so callers should compare
/// these names case-insensitively when generating several contracts together.
pub fn declared_class_names(ir: &ContractIr, wrappers: bool) -> Vec<String> {
    let registry = TupleRegistry::new(ir);
    let (_, functions, events, errors) = method_stems(ir, &registry);
    let names = class_names(ir, &registry, &functions, &events, &errors);
    let mut result = vec![names.contract];
    result.extend(names.tuples.into_values());
    result.extend(names.events);
    result.extend(names.decoded_events.into_iter().flatten());
    result.extend(names.errors);
    result.extend(names.results.into_iter().flatten());
    if wrappers {
        result.push(names.options);
        result.push(names.client);
    }
    result.sort_by_key(|name| name.to_ascii_lowercase());
    result
}

/// Renders one standalone PHP 8.2 source file.
///
/// The generated file uses Brick Math for arbitrary-width Solidity integers and,
/// when wrappers are enabled, web3p/ethereum-tx for local transaction signing.
/// ABI encoding/decoding and JSON-RPC transport are generated directly so tuple,
/// array, overload, event, and custom-error behavior does not depend on a PHP Web3
/// contract abstraction.
pub fn render_php_file(ir: &ContractIr, namespace: &str, wrappers: bool) -> String {
    let registry = TupleRegistry::new(ir);
    let (tuple_stems, function_stems, event_stems, error_stems) = method_stems(ir, &registry);
    let names = class_names(ir, &registry, &function_stems, &event_stems, &error_stems);
    let contract = &names.contract;
    let tuple_names = &names.tuples;
    let event_names = &names.events;
    let decoded_event_names = &names.decoded_events;
    let error_names = &names.errors;
    let result_names = &names.results;
    let transaction_options = &names.options;
    let client = &names.client;

    let php = Php {
        registry: &registry,
        tuple_names,
        tuple_stems: &tuple_stems,
        function_stems: &function_stems,
        event_stems: &event_stems,
        event_names,
        decoded_event_names,
        error_stems: &error_stems,
        error_names,
        result_names,
        transaction_options,
        client,
    };

    let mut out = String::from(
        "<?php\ndeclare(strict_types=1);\n\n// Generated by abi-typegen. Do not edit.\n",
    );
    if !namespace.trim().is_empty() {
        out.push_str(&format!("namespace {};\n\n", namespace.trim_matches('\\')));
    }

    for def in registry.defs() {
        let name = &tuple_names[&def.name];
        let params = def
            .components
            .iter()
            .map(|component| AbiParam {
                name: component.name.clone(),
                ty: component.ty.clone(),
                internal_type: component.internal_type.clone(),
            })
            .collect::<Vec<_>>();
        out.push_str(&php.record(name, &params, "ABI tuple value"));
    }

    for (index, event) in ir.events.iter().enumerate() {
        let params = event
            .inputs
            .iter()
            .map(|param| AbiParam {
                name: param.name.clone(),
                ty: param.ty.clone(),
                internal_type: param.internal_type.clone(),
            })
            .collect::<Vec<_>>();
        out.push_str(&php.record(&event_names[index], &params, "Decoded ABI event value"));

        if let Some(decoded_name) = &decoded_event_names[index] {
            let names = param_names(event.inputs.iter().map(|param| {
                (
                    param.name.as_str(),
                    &param.ty,
                    registry.name_of_type(&param.ty, param.internal_type.as_deref()),
                )
            }));
            let mut field_scope = Scope::default();
            let fields = event
                .inputs
                .iter()
                .zip(names)
                .map(|(param, name)| Field {
                    name: field_scope.claim(&php_ident(&name)),
                    ty: if param.indexed && indexed_hash(&param.ty) {
                        "string".into()
                    } else {
                        php.field_type(&param.ty, param.internal_type.as_deref())
                    },
                })
                .collect::<Vec<_>>();
            out.push_str(&php.record_fields(
                decoded_name,
                &fields,
                "Decoded event log; indexed reference types contain their topic hash",
            ));
        }
    }

    for (index, error) in ir.errors.iter().enumerate() {
        out.push_str(&php.record(
            &error_names[index],
            &error.inputs,
            "Decoded custom error value",
        ));
    }

    for (index, function) in ir.functions.iter().enumerate() {
        if let Some(result_name) = &result_names[index] {
            out.push_str(&php.record(
                result_name,
                &function.outputs,
                "Decoded multi-value function result",
            ));
        }
    }

    if wrappers {
        out.push_str(&layout::render_support(&php));
    }

    out.push_str(&format!(
        "\n/** Types and bindings for the `{}` contract. */\nfinal class {contract}\n{{\n",
        ir.name
    ));
    let abi = serde_json::to_string(&ir.raw_abi).expect("JSON values serialize");
    out.push_str(&format!(
        "    /** JSON ABI for the contract. */\n    public const ABI = {};\n",
        php_string(&abi)
    ));

    let mut constants = Scope::with_reserved(["ABI"]);
    for (function, index) in ir.functions.iter().zip(overload_indices(&ir.functions)) {
        let suffix = index.map(|i| format!("_{i}")).unwrap_or_default();
        let base = constants.claim_family(
            &format!("{}{}", function.name.to_shouty_snake_case(), suffix),
            &["_SIGNATURE", "_SELECTOR"],
        );
        out.push_str(&format!(
            "    /** Canonical function signature. */\n    public const {base}_SIGNATURE = {};\n",
            php_string(&function.signature())
        ));
        out.push_str(&format!(
            "    /** Four-byte function selector. */\n    public const {base}_SELECTOR = {};\n",
            php_string(&function.selector().to_string())
        ));
    }
    for (event, index) in ir.events.iter().zip(repeat_indices(
        ir.events.iter().map(|event| event.name.as_str()),
    )) {
        let suffix = index.map(|i| format!("_{i}")).unwrap_or_default();
        let base = constants.claim_family(
            &format!("{}{}", event.name.to_shouty_snake_case(), suffix),
            &["_EVENT_SIGNATURE", "_EVENT_TOPIC"],
        );
        out.push_str(&format!(
            "    /** Canonical event signature. */\n    public const {base}_EVENT_SIGNATURE = {};\n",
            php_string(&event.signature())
        ));
        if !event.anonymous {
            out.push_str(&format!(
                "    /** Event topic zero. */\n    public const {base}_EVENT_TOPIC = {};\n",
                php_string(&event.topic0().to_string())
            ));
        }
    }
    for (error, index) in ir.errors.iter().zip(repeat_indices(
        ir.errors.iter().map(|error| error.name.as_str()),
    )) {
        let suffix = index.map(|i| format!("_{i}")).unwrap_or_default();
        let base = constants.claim_family(
            &format!("{}{}", error.name.to_shouty_snake_case(), suffix),
            &["_ERROR_SIGNATURE", "_ERROR_SELECTOR"],
        );
        out.push_str(&format!(
            "    /** Canonical error signature. */\n    public const {base}_ERROR_SIGNATURE = {};\n",
            php_string(&error.signature())
        ));
        out.push_str(&format!(
            "    /** Error selector. */\n    public const {base}_ERROR_SELECTOR = {};\n",
            php_string(&error.selector().to_string())
        ));
    }

    if wrappers {
        out.push_str(&layout::render_methods(ir, &php));
    }
    out.push_str("}\n");
    out
}

#[derive(Clone)]
pub(super) struct Field {
    pub(super) name: String,
    pub(super) ty: String,
}

pub(super) struct Php<'a> {
    pub(super) registry: &'a TupleRegistry,
    pub(super) tuple_names: &'a HashMap<String, String>,
    pub(super) tuple_stems: &'a [String],
    pub(super) function_stems: &'a [String],
    pub(super) event_stems: &'a [String],
    pub(super) event_names: &'a [String],
    pub(super) decoded_event_names: &'a [Option<String>],
    pub(super) error_stems: &'a [String],
    pub(super) error_names: &'a [String],
    pub(super) result_names: &'a [Option<String>],
    pub(super) transaction_options: &'a str,
    pub(super) client: &'a str,
}

impl Php<'_> {
    pub(super) fn fields(&self, params: &[AbiParam]) -> Vec<Field> {
        self.fields_with_reserved(params, &[])
    }

    pub(super) fn fields_with_reserved(
        &self,
        params: &[AbiParam],
        reserved: &[&str],
    ) -> Vec<Field> {
        let names = param_names(params.iter().map(|param| {
            (
                param.name.as_str(),
                &param.ty,
                self.registry
                    .name_of_type(&param.ty, param.internal_type.as_deref()),
            )
        }));
        let mut scope = Scope::with_reserved(reserved.iter().copied());
        params
            .iter()
            .zip(names)
            .map(|(param, name)| Field {
                name: scope.claim(&php_ident(&name)),
                ty: self.field_type(&param.ty, param.internal_type.as_deref()),
            })
            .collect()
    }

    pub(super) fn field_type(&self, ty: &SolType, internal: Option<&str>) -> String {
        match ty {
            SolType::Bool => "bool".into(),
            SolType::Address | SolType::StringType | SolType::Bytes | SolType::BytesN(_) => {
                "string".into()
            }
            SolType::Uint(_) | SolType::Int(_) => "\\Brick\\Math\\BigInteger".into(),
            SolType::Array(_) | SolType::FixedArray(_, _) => "array".into(),
            SolType::Tuple(components) => {
                self.tuple_names[self.registry.name(components, internal)].clone()
            }
        }
    }

    pub(super) fn record(&self, name: &str, params: &[AbiParam], description: &str) -> String {
        self.record_fields(name, &self.fields(params), description)
    }

    pub(super) fn record_fields(&self, name: &str, fields: &[Field], description: &str) -> String {
        let mut out = format!("\n/** {description}. */\nfinal readonly class {name}\n{{\n");
        if fields.is_empty() {
            out.push_str("    public function __construct() {}\n");
        } else {
            out.push_str("    public function __construct(\n");
            for field in fields {
                out.push_str(&format!("        public {} ${},\n", field.ty, field.name));
            }
            out.push_str("    ) {}\n");
        }
        out.push_str("}\n");
        out
    }

    pub(super) fn layout_expr(&self, ty: &SolType, _internal: Option<&str>) -> String {
        match ty {
            SolType::Bool => "['kind' => 'atom', 'type' => 'bool']".into(),
            SolType::Address => "['kind' => 'atom', 'type' => 'address']".into(),
            SolType::StringType => "['kind' => 'atom', 'type' => 'string']".into(),
            SolType::Uint(bits) => format!("['kind' => 'atom', 'type' => 'uint{bits}']"),
            SolType::Int(bits) => format!("['kind' => 'atom', 'type' => 'int{bits}']"),
            SolType::Bytes => "['kind' => 'atom', 'type' => 'bytes']".into(),
            SolType::BytesN(size) => format!("['kind' => 'atom', 'type' => 'bytes{size}']"),
            SolType::Array(inner) => format!(
                "['kind' => 'array', 'element' => {}, 'size' => null]",
                self.layout_expr(inner, _internal)
            ),
            SolType::FixedArray(inner, size) => format!(
                "['kind' => 'array', 'element' => {}, 'size' => {size}]",
                self.layout_expr(inner, _internal)
            ),
            SolType::Tuple(components) => {
                let fields = components
                    .iter()
                    .map(|component| {
                        self.layout_expr(&component.ty, component.internal_type.as_deref())
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("['kind' => 'tuple', 'fields' => [{fields}]]")
            }
        }
    }

    pub(super) fn to_abi(
        &self,
        expr: &str,
        ty: &SolType,
        _internal: Option<&str>,
        depth: usize,
    ) -> String {
        match ty {
            SolType::Bool
            | SolType::Address
            | SolType::StringType
            | SolType::Uint(_)
            | SolType::Int(_)
            | SolType::Bytes
            | SolType::BytesN(_) => expr.into(),
            SolType::Array(inner) | SolType::FixedArray(inner, _) => {
                let item = format!("$item{depth}");
                let converted = self.to_abi(&item, inner, _internal, depth + 1);
                format!("array_map(static fn($item{depth}) => {converted}, {expr})")
            }
            SolType::Tuple(components) => {
                let params = components
                    .iter()
                    .map(|component| AbiParam {
                        name: component.name.clone(),
                        ty: component.ty.clone(),
                        internal_type: component.internal_type.clone(),
                    })
                    .collect::<Vec<_>>();
                let fields = self.fields(&params);
                let values = components
                    .iter()
                    .zip(fields)
                    .map(|(component, field)| {
                        self.to_abi(
                            &format!("{expr}->{}", field.name),
                            &component.ty,
                            component.internal_type.as_deref(),
                            depth + 1,
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{values}]")
            }
        }
    }

    pub(super) fn decode_abi_expr(
        &self,
        expr: &str,
        ty: &SolType,
        internal: Option<&str>,
        depth: usize,
    ) -> String {
        match ty {
            SolType::Bool
            | SolType::Address
            | SolType::StringType
            | SolType::Uint(_)
            | SolType::Int(_)
            | SolType::Bytes
            | SolType::BytesN(_) => expr.into(),
            SolType::Array(inner) | SolType::FixedArray(inner, _) => {
                let converted =
                    self.decode_abi_expr(&format!("$item{depth}"), inner, internal, depth + 1);
                format!("array_map(static fn($item{depth}) => {converted}, {expr})")
            }
            SolType::Tuple(components) => {
                let name = &self.tuple_names[self.registry.name(components, internal)];
                let args = components
                    .iter()
                    .enumerate()
                    .map(|(index, component)| {
                        self.decode_abi_expr(
                            &format!("{expr}[{index}]"),
                            &component.ty,
                            component.internal_type.as_deref(),
                            depth + 1,
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("new {name}({args})")
            }
        }
    }
}

pub(super) fn php_ident(name: &str) -> String {
    let name = if name.is_empty() { "arg" } else { name };
    if PHP_RESERVED_IDENTIFIERS
        .iter()
        .any(|reserved| reserved.eq_ignore_ascii_case(name))
        || name == "_"
    {
        format!("{name}_")
    } else {
        name.into()
    }
}

pub(super) fn indexed_hash(ty: &SolType) -> bool {
    matches!(
        ty,
        SolType::StringType
            | SolType::Bytes
            | SolType::Array(_)
            | SolType::FixedArray(_, _)
            | SolType::Tuple(_)
    )
}

pub(super) fn php_string(value: &str) -> String {
    let mut out = String::from("'");
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            c => out.push(c),
        }
    }
    out.push('\'');
    out
}

const PHP_RESERVED_CLASS_NAMES: &[&str] = &[
    "self", "parent", "static", "int", "float", "bool", "string", "true", "false", "null", "void",
    "iterable", "object", "resource", "mixed", "numeric", "never", "callable", "array", "enum",
    "readonly",
];

const PHP_RESERVED_IDENTIFIERS: &[&str] = &[
    "__halt_compiler",
    "abstract",
    "and",
    "array",
    "as",
    "break",
    "callable",
    "case",
    "catch",
    "class",
    "clone",
    "const",
    "continue",
    "declare",
    "default",
    "die",
    "do",
    "echo",
    "else",
    "elseif",
    "empty",
    "enddeclare",
    "endfor",
    "endforeach",
    "endif",
    "endswitch",
    "endwhile",
    "enum",
    "eval",
    "exit",
    "extends",
    "final",
    "finally",
    "fn",
    "for",
    "foreach",
    "function",
    "global",
    "goto",
    "if",
    "implements",
    "include",
    "include_once",
    "instanceof",
    "insteadof",
    "interface",
    "isset",
    "list",
    "match",
    "namespace",
    "new",
    "or",
    "print",
    "private",
    "protected",
    "public",
    "readonly",
    "require",
    "require_once",
    "return",
    "static",
    "switch",
    "throw",
    "trait",
    "try",
    "unset",
    "use",
    "var",
    "while",
    "xor",
    "yield",
    "yield_from",
    "this",
    "true",
    "false",
    "null",
    "value",
    "into",
];

#[cfg(test)]
mod tests {
    use super::*;
    use abi_typegen_core::parser::parse_artifact;

    #[test]
    fn renders_wrappers_and_metadata_mode() {
        let ir = parse_artifact(
            "Token",
            r#"{"abi":[{"type":"function","name":"balanceOf","inputs":[{"name":"owner","type":"address"}],"outputs":[{"name":"","type":"uint256"}],"stateMutability":"view"},{"type":"function","name":"mint","inputs":[{"name":"to","type":"address"},{"name":"amount","type":"uint256"}],"outputs":[],"stateMutability":"nonpayable"}]}"#,
        )
        .expect("abi");
        let full = render_php_file(&ir, "Example\\Contracts", true);
        assert!(full.contains("function encodeBalanceOf"), "{full}");
        assert!(full.contains("function decodeBalanceOfResult"), "{full}");
        assert!(full.contains("function callBalanceOf"), "{full}");
        assert!(full.contains("function sendMint"), "{full}");
        let plain = render_php_file(&ir, "Example\\Contracts", false);
        assert!(plain.contains("public const ABI"));
        assert!(!plain.contains("function encodeBalanceOf"));
    }

    #[test]
    fn php_method_families_and_parameters_do_not_collide() {
        let ir = parse_artifact(
            "Collisions",
            r#"{"abi":[{"type":"function","name":"fooBar","inputs":[{"name":"client","type":"uint256"}],"outputs":[],"stateMutability":"view"},{"type":"function","name":"foo_bar","inputs":[{"name":"options","type":"uint256"}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"FOOBAR","inputs":[],"outputs":[],"stateMutability":"view"},{"type":"event","name":"Changed","inputs":[{"name":"address","type":"address","indexed":true},{"name":"fromBlock","type":"uint256","indexed":true},{"name":"toBlock","type":"uint256","indexed":true}],"anonymous":false}]}"#,
        )
        .expect("ABI");
        let output = render_php_file(&ir, "NativeBindings", true);
        let lower = output.to_ascii_lowercase();
        assert_eq!(lower.matches("function encodefoobar(").count(), 1);
        assert_eq!(lower.matches("function encodefoobar2(").count(), 1);
        assert_eq!(lower.matches("function encodefoobar3(").count(), 1);
        assert!(output.contains("$client, \\Brick\\Math\\BigInteger $client2"));
        assert!(output.contains(
            "\\Brick\\Math\\BigInteger $options2, CollisionsTransactionOptions $options"
        ));
        assert!(output.contains("?string $address2 = null"));
    }

    #[test]
    fn class_inventory_includes_only_emitted_php_classes() {
        let ir = parse_artifact("Token", r#"{"abi":[{"type":"function","name":"numbers","inputs":[],"outputs":[{"name":"a","type":"uint256"},{"name":"b","type":"uint256"}],"stateMutability":"view"}]}"#).expect("ABI");
        let full = declared_class_names(&ir, true);
        assert!(full.contains(&"Token".to_string()));
        assert!(full.contains(&"TokenNumbersResult".to_string()));
        assert!(full.contains(&"TokenClient".to_string()));
        assert!(full.contains(&"TokenTransactionOptions".to_string()));
        let plain = declared_class_names(&ir, false);
        assert!(!plain.contains(&"TokenClient".to_string()));
        assert!(!plain.contains(&"TokenTransactionOptions".to_string()));
    }

    #[test]
    fn php_function_names_do_not_shadow_codec_methods() {
        let ir = parse_artifact("CodecNames", r#"{"abi":[{"type":"function","name":"Sequence","inputs":[],"outputs":[],"stateMutability":"view"},{"type":"function","name":"Value","inputs":[],"outputs":[],"stateMutability":"view"}]}"#).expect("ABI");
        let output = render_php_file(&ir, "NativeBindings", true);
        assert!(output.contains("function encodeSequence2()"));
        assert!(output.contains("function encodeValue2()"));
    }
}
