//! Swift output: a public namespace per contract with its ABI, selectors,
//! and `Sendable`, `Hashable` value types for web3swift.

use crate::naming::{Scope, exported, lower_first, overload_indices, param_names, repeat_indices};
use crate::tuples::TupleRegistry;
use abi_typegen_core::types::{ContractIr, NatSpec, SolType};
use std::collections::HashMap;

/// Swift keywords that need backticks when used as identifiers.
const SWIFT_KEYWORDS: &[&str] = &[
    "Any",
    "Protocol",
    "Self",
    "Type",
    "as",
    "associatedtype",
    "break",
    "case",
    "catch",
    "class",
    "continue",
    "default",
    "defer",
    "deinit",
    "do",
    "else",
    "enum",
    "extension",
    "fallthrough",
    "false",
    "fileprivate",
    "for",
    "func",
    "guard",
    "if",
    "import",
    "in",
    "init",
    "inout",
    "internal",
    "is",
    "let",
    "nil",
    "open",
    "operator",
    "private",
    "protocol",
    "public",
    "repeat",
    "rethrows",
    "return",
    "self",
    "static",
    "struct",
    "subscript",
    "super",
    "switch",
    "throw",
    "throws",
    "true",
    "try",
    "typealias",
    "var",
    "where",
    "while",
];

/// SDK types and modules used by generated code. Contract namespaces and nested
/// structs must not shadow these names.
const SDK_NAMES: &[&str] = &[
    "Array",
    "BigInt",
    "BigUInt",
    "Bool",
    "Data",
    "EthereumAddress",
    "Foundation",
    "Hashable",
    "Sendable",
    "String",
    "Swift",
    "Web3Core",
];

/// Imports a rendered file needs beyond Foundation.
#[derive(Debug, Default)]
struct Imports {
    big_int: bool,
    web3_core: bool,
}

/// One stored property of a rendered struct.
#[derive(Debug)]
struct Field {
    name: String,
    ty: String,
}

/// Returns the public Swift namespace for a contract, avoiding SDK type and module names.
pub fn namespace_name(contract_name: &str) -> String {
    Scope::with_reserved(SDK_NAMES.iter().copied()).claim(&exported(contract_name))
}

/// Renders `<Name>.swift` for `ir`.
pub fn render_swift_file(ir: &ContractIr) -> String {
    let registry = TupleRegistry::new(ir);
    let mut imports = Imports::default();
    // The namespace must not shadow SDK types or imported modules used below.
    let contract = namespace_name(&ir.name);
    let mut scope = Scope::with_reserved(
        [contract.as_str(), "abi"]
            .into_iter()
            .chain(SDK_NAMES.iter().copied()),
    );

    let tuple_names: HashMap<String, String> = registry
        .defs()
        .iter()
        .map(|def| (def.name.clone(), scope.claim(&def.name)))
        .collect();
    let swift_type = |ty: &SolType, internal_type: Option<&str>, imports: &mut Imports| {
        sol_type_to_swift(ty, internal_type, &registry, &tuple_names, imports)
    };
    let fields_of = |params: Vec<(&str, &SolType, Option<&str>)>, imports: &mut Imports| {
        let names = param_names(params.iter().map(|(name, ty, internal_type)| {
            (*name, *ty, registry.name_of_type(ty, *internal_type))
        }));
        // `self` would shadow the initializer receiver even when backticked.
        let mut field_scope = Scope::with_reserved(["_", "self"]);
        names
            .iter()
            .zip(&params)
            .map(|(name, (_, ty, internal_type))| Field {
                name: field_scope.claim(name),
                ty: swift_type(ty, *internal_type, imports),
            })
            .collect::<Vec<_>>()
    };

    let mut constants: Vec<(String, String, String)> = Vec::new();
    let mut types = Vec::new();

    for def in registry.defs() {
        let fields = fields_of(
            def.components
                .iter()
                .map(|c| (c.name.as_str(), &c.ty, c.internal_type.as_deref()))
                .collect(),
            &mut imports,
        );
        types.push(render_struct(
            &tuple_names[&def.name],
            &format!("The `{}` tuple.", def.name),
            None,
            &fields,
        ));
    }

    let overloads = overload_indices(&ir.functions);
    for (function, overload) in ir.functions.iter().zip(overloads) {
        let index = overload.map(|i| i.to_string()).unwrap_or_default();
        let signature = function.signature();
        let constant = scope.claim_family(
            &format!("{}{index}", function.name),
            &["Signature", "Selector"],
        );
        constants.push((
            format!("{constant}Signature"),
            format!("Canonical signature of `{signature}`."),
            swift_string(&signature),
        ));
        constants.push((
            format!("{constant}Selector"),
            format!("Selector of `{signature}`."),
            swift_data(function.selector().as_slice()),
        ));
        if function.inputs.is_empty() {
            continue;
        }
        let fields = fields_of(
            function
                .inputs
                .iter()
                .map(|p| (p.name.as_str(), &p.ty, p.internal_type.as_deref()))
                .collect(),
            &mut imports,
        );
        types.push(render_struct(
            &format!(
                "{}Params",
                scope.claim_family(&format!("{}{index}", exported(&function.name)), &["Params"])
            ),
            &format!("Arguments of `{signature}`."),
            function.natspec.as_ref(),
            &fields,
        ));
    }

    let event_overloads = suffixes(ir.events.iter().map(|e| e.name.as_str()));
    for (event, index) in ir.events.iter().zip(event_overloads) {
        let signature = event.signature();
        let constant = scope.claim_family(
            &format!("{}{index}", lower_first(&exported(&event.name))),
            &["EventSignature", "EventTopic"],
        );
        constants.push((
            format!("{constant}EventSignature"),
            format!("Canonical signature of the `{signature}` event."),
            swift_string(&signature),
        ));
        if !event.anonymous {
            constants.push((
                format!("{constant}EventTopic"),
                format!("Topic 0 of the `{signature}` event."),
                swift_data(event.topic0().as_slice()),
            ));
        }
        let fields = fields_of(
            event
                .inputs
                .iter()
                .map(|p| (p.name.as_str(), &p.ty, p.internal_type.as_deref()))
                .collect(),
            &mut imports,
        );
        types.push(render_struct(
            &format!(
                "{}Event",
                scope.claim_family(&format!("{}{index}", exported(&event.name)), &["Event"])
            ),
            &format!("Fields of the `{signature}` event."),
            event.natspec.as_ref(),
            &fields,
        ));
    }

    let error_overloads = suffixes(ir.errors.iter().map(|e| e.name.as_str()));
    for (error, index) in ir.errors.iter().zip(error_overloads) {
        let signature = error.signature();
        let constant = scope.claim_family(
            &format!("{}{index}", lower_first(&exported(&error.name))),
            &["ErrorSignature", "ErrorSelector"],
        );
        constants.push((
            format!("{constant}ErrorSignature"),
            format!("Canonical signature of the `{signature}` error."),
            swift_string(&signature),
        ));
        constants.push((
            format!("{constant}ErrorSelector"),
            format!("Selector of the `{signature}` error."),
            swift_data(error.selector().as_slice()),
        ));
        let fields = fields_of(
            error
                .inputs
                .iter()
                .map(|p| (p.name.as_str(), &p.ty, p.internal_type.as_deref()))
                .collect(),
            &mut imports,
        );
        types.push(render_struct(
            &format!(
                "{}Error",
                scope.claim_family(&format!("{}{index}", exported(&error.name)), &["Error"])
            ),
            &format!("Arguments of the `{signature}` error."),
            error.natspec.as_ref(),
            &fields,
        ));
    }

    let mut out = String::from("// Generated by abi-typegen. Do not edit.\n\n");
    if imports.big_int {
        out.push_str("import BigInt\n");
    }
    out.push_str("import Foundation\n");
    if imports.web3_core {
        out.push_str("import Web3Core\n");
    }
    out.push('\n');

    out.push_str(&doc_lines(
        "",
        &format!("Types and constants of the `{}` contract.", ir.name),
        ir.natspec.as_ref(),
    ));
    out.push_str(&format!("public enum {contract} {{\n"));
    out.push_str(&format!(
        "    /// JSON ABI of the `{}` contract.\n    public static let abi = {}\n",
        ir.name,
        swift_raw_string(&serde_json::to_string(&ir.raw_abi).expect("JSON values serialize"))
    ));
    for (name, doc, value) in &constants {
        out.push_str(&format!(
            "\n    /// {doc}\n    public static let {} = {value}\n",
            escape_keyword(name)
        ));
    }
    for ty in &types {
        out.push('\n');
        out.push_str(ty);
    }
    out.push_str("}\n");
    out
}

fn render_struct(name: &str, summary: &str, natspec: Option<&NatSpec>, fields: &[Field]) -> String {
    let mut out = doc_lines("    ", summary, natspec);
    out.push_str(&format!(
        "    public struct {name}: Sendable, Hashable {{\n"
    ));
    for field in fields {
        out.push_str(&format!(
            "        public let {}: {}\n",
            escape_keyword(&field.name),
            field.ty
        ));
    }
    if !fields.is_empty() {
        out.push('\n');
    }
    let params = fields
        .iter()
        .map(|field| format!("{}: {}", escape_keyword(&field.name), field.ty))
        .collect::<Vec<_>>()
        .join(", ");
    if fields.is_empty() {
        out.push_str("        public init() {}\n");
    } else {
        out.push_str(&format!("        public init({params}) {{\n"));
        for field in fields {
            let name = escape_keyword(&field.name);
            out.push_str(&format!("            self.{name} = {name}\n"));
        }
        out.push_str("        }\n");
    }
    out.push_str("    }\n");
    out
}

fn sol_type_to_swift(
    ty: &SolType,
    internal_type: Option<&str>,
    registry: &TupleRegistry,
    tuple_names: &HashMap<String, String>,
    imports: &mut Imports,
) -> String {
    match ty {
        SolType::Bool => "Bool".to_string(),
        SolType::StringType => "String".to_string(),
        SolType::Address => {
            imports.web3_core = true;
            "EthereumAddress".to_string()
        }
        SolType::Bytes | SolType::BytesN(_) => "Data".to_string(),
        SolType::Uint(_) => {
            imports.big_int = true;
            "BigUInt".to_string()
        }
        SolType::Int(_) => {
            imports.big_int = true;
            "BigInt".to_string()
        }
        SolType::Array(inner) | SolType::FixedArray(inner, _) => format!(
            "[{}]",
            sol_type_to_swift(inner, internal_type, registry, tuple_names, imports)
        ),
        SolType::Tuple(components) => tuple_names[registry.name(components, internal_type)].clone(),
    }
}

fn escape_keyword(name: &str) -> String {
    if SWIFT_KEYWORDS.contains(&name) {
        format!("`{name}`")
    } else {
        name.to_string()
    }
}

fn doc_lines(indent: &str, summary: &str, natspec: Option<&NatSpec>) -> String {
    let mut out = format!("{indent}/// {summary}\n");
    if let Some(natspec) = natspec {
        let lines: Vec<String> = natspec
            .notice
            .iter()
            .chain(natspec.dev.iter())
            .flat_map(|text| text.lines())
            .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect();
        if !lines.is_empty() {
            out.push_str(&format!("{indent}///\n"));
            for line in lines {
                if line.is_empty() {
                    out.push_str(&format!("{indent}///\n"));
                } else {
                    out.push_str(&format!("{indent}/// {line}\n"));
                }
            }
        }
    }
    out
}

fn swift_string(value: &str) -> String {
    let mut out = String::from("\"");
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Renders a single-line raw string that the content cannot close or escape.
fn swift_raw_string(value: &str) -> String {
    let mut hashes = 1;
    while value.contains(&format!("\"{}", "#".repeat(hashes)))
        || value.contains(&format!("\\{}", "#".repeat(hashes)))
    {
        hashes += 1;
    }
    let fence = "#".repeat(hashes);
    format!("{fence}\"{value}\"{fence}")
}

fn swift_data(bytes: &[u8]) -> String {
    let items = bytes
        .iter()
        .map(|byte| format!("0x{byte:02x}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("Data([{items}])")
}

/// Overload suffixes: `"0"`, `"1"`, ... for repeated names, `""` otherwise.
fn suffixes<'a>(names: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    repeat_indices(names)
        .into_iter()
        .map(|index| index.map(|i| i.to_string()).unwrap_or_default())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use abi_typegen_core::parser::parse_artifact;

    fn render(abi: &str) -> String {
        let ir = parse_artifact("Token", &format!(r#"{{"abi":{abi}}}"#)).expect("valid abi");
        render_swift_file(&ir)
    }

    #[test]
    fn everything_is_public_and_imports_web3core() {
        let out = render(
            r#"[{"type":"function","name":"transfer","inputs":[{"name":"to","type":"address"},{"name":"amount","type":"uint256"}],"outputs":[],"stateMutability":"nonpayable"}]"#,
        );
        assert!(
            out.contains("import BigInt\nimport Foundation\nimport Web3Core\n"),
            "{out}"
        );
        assert!(!out.contains("import web3swift"), "{out}");
        assert!(out.contains("public enum Token {"), "{out}");
        assert!(
            out.contains("    public struct TransferParams: Sendable, Hashable {\n        public let to: EthereumAddress\n        public let amount: BigUInt\n\n        public init(to: EthereumAddress, amount: BigUInt) {\n            self.to = to\n            self.amount = amount\n        }\n    }"),
            "{out}"
        );
    }

    #[test]
    fn embeds_abi_signatures_and_selectors() {
        let out = render(
            r#"[{"type":"function","name":"transfer","inputs":[{"name":"to","type":"address"},{"name":"amount","type":"uint256"}],"outputs":[],"stateMutability":"nonpayable"},
               {"type":"event","name":"Transfer","inputs":[],"anonymous":false}]"#,
        );
        assert!(out.contains("public static let abi = #\"[{"), "{out}");
        assert!(
            out.contains("public static let transferSignature = \"transfer(address,uint256)\""),
            "{out}"
        );
        assert!(
            out.contains("public static let transferSelector = Data([0xa9, 0x05, 0x9c, 0xbb])"),
            "{out}"
        );
        assert!(
            out.contains("public static let transferEventTopic = Data([0x"),
            "{out}"
        );
    }

    #[test]
    fn tuples_are_named_nested_structs() {
        let out = render(
            r#"[{"type":"function","name":"deposit","inputs":[{"name":"p","type":"tuple[]","internalType":"struct Token.Position[]","components":[{"name":"account","type":"address"}]}],"outputs":[],"stateMutability":"nonpayable"}]"#,
        );
        assert!(
            out.contains("    public struct Position: Sendable, Hashable {"),
            "{out}"
        );
        assert!(out.contains("public let p: [Position]"), "{out}");
    }

    #[test]
    fn overloads_number_type_names() {
        let out = render(
            r#"[
            {"type":"function","name":"safeTransferFrom","inputs":[{"name":"from","type":"address"}],"outputs":[],"stateMutability":"nonpayable"},
            {"type":"function","name":"safeTransferFrom","inputs":[{"name":"from","type":"address"},{"name":"data","type":"bytes"}],"outputs":[],"stateMutability":"nonpayable"}
        ]"#,
        );
        assert!(out.contains("struct SafeTransferFrom0Params"), "{out}");
        assert!(out.contains("struct SafeTransferFrom1Params"), "{out}");
        assert!(out.contains("safeTransferFrom1Selector"), "{out}");
    }

    #[test]
    fn keywords_are_escaped_and_unnamed_params_use_types() {
        let out = render(
            r#"[{"type":"function","name":"f","inputs":[{"name":"default","type":"bool"},{"name":"","type":"uint8"}],"outputs":[],"stateMutability":"nonpayable"}]"#,
        );
        assert!(out.contains("public let `default`: Bool"), "{out}");
        assert!(out.contains("self.`default` = `default`"), "{out}");
        assert!(out.contains("public let uint8: BigUInt"), "{out}");
    }

    #[test]
    fn empty_struct_has_public_init() {
        let out = render(r#"[{"type":"event","name":"Ping","inputs":[],"anonymous":true}]"#);
        assert!(
            out.contains(
                "public struct PingEvent: Sendable, Hashable {\n        public init() {}\n    }"
            ),
            "{out}"
        );
        assert!(!out.contains("pingEventTopic"), "{out}");
        assert!(!out.contains("import BigInt"), "{out}");
    }

    #[test]
    fn tuples_named_like_sdk_types_do_not_shadow_them() {
        let out = render(
            r#"[{"type":"function","name":"f","inputs":[{"name":"d","type":"tuple","internalType":"struct Token.Data","components":[{"name":"x","type":"bytes"}]}],"outputs":[],"stateMutability":"nonpayable"}]"#,
        );
        assert!(
            out.contains("public struct Data2: Sendable, Hashable {\n        public let x: Data\n"),
            "{out}"
        );
        assert!(out.contains("public let d: Data2"), "{out}");
    }

    #[test]
    fn raw_string_fence_outgrows_content() {
        assert_eq!(swift_raw_string("a"), "#\"a\"#");
        assert_eq!(swift_raw_string("a\"#b"), "##\"a\"#b\"##");
        assert_eq!(swift_raw_string("a\\#(b)"), "##\"a\\#(b)\"##");
    }
}
