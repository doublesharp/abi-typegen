//! Go output: ABI, selectors, and named structs in go-ethereum's type model.
//!
//! Output is gofmt-clean. Integer types follow abigen: only exact 8, 16, 32,
//! and 64-bit widths map to native Go integers, and every other width uses
//! `*big.Int`, which is what go-ethereum decodes into.

use crate::naming::{Scope, exported, param_names};
use crate::tuples::TupleRegistry;
use abi_typegen_core::types::{ContractIr, NatSpec, SolType};
use std::collections::{BTreeSet, HashMap};

/// Go imports a rendered file needs.
#[derive(Debug, Default)]
struct Imports {
    big: bool,
    common: bool,
}

/// Renders `<Name>.go` for `ir` in package `package`.
pub fn render_go_file(ir: &ContractIr, package: &str) -> String {
    let registry = TupleRegistry::new(ir);
    let mut imports = Imports::default();
    let mut scope = Scope::default();
    let contract = exported(&ir.name);

    // Name every tuple before anything else so their names are stable.
    let tuple_names: HashMap<String, String> = registry
        .defs()
        .iter()
        .map(|def| {
            (
                def.name.clone(),
                scope.claim(&format!("{contract}{}", def.name)),
            )
        })
        .collect();
    let go_type = |ty: &SolType, internal_type: Option<&str>, imports: &mut Imports| {
        sol_type_to_go(ty, internal_type, &registry, &tuple_names, imports)
    };

    let mut body = String::new();

    // Tuple structs.
    for def in registry.defs() {
        let name = &tuple_names[&def.name];
        let fields = struct_fields(
            def.components
                .iter()
                .map(|c| (c.name.as_str(), &c.ty, c.internal_type.as_deref())),
            &registry,
            &mut |ty, internal_type| go_type(ty, internal_type, &mut imports),
        );
        body.push_str(&format!(
            "// {name} is the Go form of the {} tuple.\n",
            def.name
        ));
        body.push_str(&render_struct(name, &fields));
    }

    // Function parameter structs and constants.
    let function_names = abigen_names(ir.functions.iter().map(|f| f.name.as_str()));
    let mut consts = Vec::new();
    let mut vars = Vec::new();
    for (function, base) in ir.functions.iter().zip(&function_names) {
        let signature = function.signature();
        consts.push((
            scope.claim(&format!("{contract}{base}Signature")),
            go_string(&signature),
        ));
        vars.push((
            scope.claim(&format!("{contract}{base}Selector")),
            go_bytes(function.selector().as_slice()),
        ));
        if function.inputs.is_empty() {
            continue;
        }
        let name = scope.claim(&format!("{contract}{base}Params"));
        let fields = struct_fields(
            function
                .inputs
                .iter()
                .map(|p| (p.name.as_str(), &p.ty, p.internal_type.as_deref())),
            &registry,
            &mut |ty, internal_type| go_type(ty, internal_type, &mut imports),
        );
        body.push_str(&doc_comment(
            &format!("{name} holds the arguments of {signature}."),
            function.natspec.as_ref(),
        ));
        body.push_str(&render_struct(&name, &fields));
    }

    // Event structs and topics.
    let event_names = abigen_names(ir.events.iter().map(|e| e.name.as_str()));
    for (event, base) in ir.events.iter().zip(&event_names) {
        let signature = event.signature();
        consts.push((
            scope.claim(&format!("{contract}{base}EventSignature")),
            go_string(&signature),
        ));
        imports.common = true;
        vars.push((
            scope.claim(&format!("{contract}{base}EventTopic")),
            format!("common.HexToHash(\"{}\")", event.topic0()),
        ));
        let name = scope.claim(&format!("{contract}{base}Event"));
        let fields = struct_fields(
            event
                .inputs
                .iter()
                .map(|p| (p.name.as_str(), &p.ty, p.internal_type.as_deref())),
            &registry,
            &mut |ty, internal_type| go_type(ty, internal_type, &mut imports),
        );
        body.push_str(&doc_comment(
            &format!("{name} holds the fields of the {signature} event."),
            event.natspec.as_ref(),
        ));
        body.push_str(&render_struct(&name, &fields));
    }

    // Error structs and selectors.
    let error_names = abigen_names(ir.errors.iter().map(|e| e.name.as_str()));
    for (error, base) in ir.errors.iter().zip(&error_names) {
        let signature = error.signature();
        consts.push((
            scope.claim(&format!("{contract}{base}ErrorSignature")),
            go_string(&signature),
        ));
        vars.push((
            scope.claim(&format!("{contract}{base}ErrorSelector")),
            go_bytes(error.selector().as_slice()),
        ));
        let name = scope.claim(&format!("{contract}{base}Error"));
        let fields = struct_fields(
            error
                .inputs
                .iter()
                .map(|p| (p.name.as_str(), &p.ty, p.internal_type.as_deref())),
            &registry,
            &mut |ty, internal_type| go_type(ty, internal_type, &mut imports),
        );
        body.push_str(&doc_comment(
            &format!("{name} holds the arguments of the {signature} error."),
            error.natspec.as_ref(),
        ));
        body.push_str(&render_struct(&name, &fields));
    }

    let mut out = String::new();
    out.push_str("// Code generated by abi-typegen. DO NOT EDIT.\n\n");
    out.push_str(&format!("package {package}\n\n"));
    out.push_str(&render_imports(&imports));

    // Contract NatSpec documents the ABI constant. Above the package clause it
    // would become the package doc, and every contract file would compete for it.
    let abi_name = scope.claim(&format!("{contract}ABI"));
    out.push_str(&doc_comment(
        &format!("{abi_name} is the JSON ABI of the {} contract.", ir.name),
        ir.natspec.as_ref(),
    ));
    out.push_str(&format!(
        "const {abi_name} = {}\n\n",
        go_raw_or_quoted(&serde_json::to_string(&ir.raw_abi).expect("JSON values serialize"))
    ));
    if !consts.is_empty() {
        out.push_str(&format!(
            "// Canonical signatures of the {} contract's functions, events, and errors.\n",
            ir.name
        ));
        out.push_str(&render_value_block("const", &consts));
    }
    if !vars.is_empty() {
        out.push_str(&format!(
            "// Selectors and event topics of the {} contract.\n",
            ir.name
        ));
        out.push_str(&render_value_block("var", &vars));
    }
    out.push_str(&body);

    while out.ends_with("\n\n") {
        out.pop();
    }
    out
}

/// Names items the way abigen does: the first keeps its name and later
/// items with the same name get `0`, `1`, ...
fn abigen_names<'a>(names: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut seen: HashMap<String, usize> = HashMap::new();
    names
        .into_iter()
        .map(|name| {
            let base = exported(name);
            let count = seen.entry(base.clone()).or_insert(0);
            let result = if *count == 0 {
                base.clone()
            } else {
                format!("{base}{}", *count - 1)
            };
            *count += 1;
            result
        })
        .collect()
}

fn struct_fields<'a>(
    params: impl IntoIterator<Item = (&'a str, &'a SolType, Option<&'a str>)>,
    registry: &TupleRegistry,
    go_type: &mut dyn FnMut(&SolType, Option<&str>) -> String,
) -> Vec<(String, String)> {
    let params: Vec<_> = params.into_iter().collect();
    let names =
        param_names(params.iter().map(|(name, ty, internal_type)| {
            (*name, *ty, registry.name_of_type(ty, *internal_type))
        }));
    let mut scope = Scope::default();
    names
        .iter()
        .zip(&params)
        .map(|(name, (_, ty, internal_type))| {
            (scope.claim(&exported(name)), go_type(ty, *internal_type))
        })
        .collect()
}

fn sol_type_to_go(
    ty: &SolType,
    internal_type: Option<&str>,
    registry: &TupleRegistry,
    tuple_names: &HashMap<String, String>,
    imports: &mut Imports,
) -> String {
    match ty {
        SolType::Bool => "bool".to_string(),
        SolType::StringType => "string".to_string(),
        SolType::Address => {
            imports.common = true;
            "common.Address".to_string()
        }
        SolType::Bytes => "[]byte".to_string(),
        SolType::BytesN(size) => format!("[{size}]byte"),
        SolType::Uint(bits @ (8 | 16 | 32 | 64)) => format!("uint{bits}"),
        SolType::Int(bits @ (8 | 16 | 32 | 64)) => format!("int{bits}"),
        SolType::Uint(_) | SolType::Int(_) => {
            imports.big = true;
            "*big.Int".to_string()
        }
        SolType::Array(inner) => format!(
            "[]{}",
            sol_type_to_go(inner, internal_type, registry, tuple_names, imports)
        ),
        SolType::FixedArray(inner, size) => format!(
            "[{size}]{}",
            sol_type_to_go(inner, internal_type, registry, tuple_names, imports)
        ),
        SolType::Tuple(components) => tuple_names[registry.name(components, internal_type)].clone(),
    }
}

fn render_imports(imports: &Imports) -> String {
    let mut groups: Vec<BTreeSet<&str>> = Vec::new();
    if imports.big {
        groups.push(BTreeSet::from(["\"math/big\""]));
    }
    if imports.common {
        groups.push(BTreeSet::from([
            "\"github.com/ethereum/go-ethereum/common\"",
        ]));
    }
    if groups.is_empty() {
        return String::new();
    }
    let body = groups
        .iter()
        .map(|group| {
            group
                .iter()
                .map(|path| format!("\t{path}\n"))
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("import (\n{body})\n\n")
}

/// Renders a struct with gofmt's column alignment.
fn render_struct(name: &str, fields: &[(String, String)]) -> String {
    if fields.is_empty() {
        return format!("type {name} struct{{}}\n\n");
    }
    let width = fields
        .iter()
        .map(|(field, _)| field.len())
        .max()
        .unwrap_or(0);
    let mut out = format!("type {name} struct {{\n");
    for (field, ty) in fields {
        out.push_str(&format!("\t{field:<width$} {ty}\n"));
    }
    out.push_str("}\n\n");
    out
}

/// Renders a `const (...)` or `var (...)` block with aligned `=`.
fn render_value_block(keyword: &str, values: &[(String, String)]) -> String {
    let width = values.iter().map(|(name, _)| name.len()).max().unwrap_or(0);
    let mut out = format!("{keyword} (\n");
    for (name, value) in values {
        out.push_str(&format!("\t{name:<width$} = {value}\n"));
    }
    out.push_str(")\n\n");
    out
}

fn doc_comment(summary: &str, natspec: Option<&NatSpec>) -> String {
    let mut out = comment_line(summary);
    if let Some(natspec) = natspec {
        let lines = natspec_lines(natspec);
        if !lines.is_empty() {
            out.push_str("//\n");
            for line in lines {
                out.push_str(&comment_line(&line));
            }
        }
    }
    out
}

fn natspec_lines(natspec: &NatSpec) -> Vec<String> {
    natspec
        .notice
        .iter()
        .chain(natspec.dev.iter())
        .flat_map(|text| text.lines())
        .map(|line| line.trim().to_string())
        .collect()
}

fn comment_line(text: &str) -> String {
    if text.is_empty() {
        "//\n".to_string()
    } else {
        format!("// {text}\n")
    }
}

fn go_string(value: &str) -> String {
    format!("{value:?}")
}

fn go_raw_or_quoted(value: &str) -> String {
    if value.contains('`') {
        go_string(value)
    } else {
        format!("`{value}`")
    }
}

fn go_bytes(bytes: &[u8]) -> String {
    let items = bytes
        .iter()
        .map(|byte| format!("0x{byte:02x}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{}]byte{{{items}}}", bytes.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use abi_typegen_core::parser::parse_artifact;

    fn render(abi: &str) -> String {
        let ir = parse_artifact("Token", &format!(r#"{{"abi":{abi}}}"#)).expect("valid abi");
        render_go_file(&ir, "contracts")
    }

    fn erc20() -> String {
        let json = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/erc20.json"),
        )
        .expect("fixture exists");
        render_go_file(&parse_artifact("ERC20", &json).expect("valid"), "bindings")
    }

    #[test]
    fn header_is_separated_from_package_clause() {
        let out = erc20();
        assert!(
            out.starts_with("// Code generated by abi-typegen. DO NOT EDIT.\n\npackage bindings\n"),
            "{out}"
        );
    }

    #[test]
    fn imports_only_what_is_used() {
        let out = render(
            r#"[{"type":"function","name":"f","inputs":[{"name":"x","type":"uint64"}],"outputs":[],"stateMutability":"nonpayable"}]"#,
        );
        assert!(!out.contains("import"), "{out}");
        let out = render(
            r#"[{"type":"function","name":"f","inputs":[{"name":"x","type":"uint256"}],"outputs":[],"stateMutability":"nonpayable"}]"#,
        );
        assert!(out.contains("import (\n\t\"math/big\"\n)"), "{out}");
        assert!(!out.contains("go-ethereum/common"), "{out}");
    }

    #[test]
    fn event_topics_import_common() {
        let out = render(r#"[{"type":"event","name":"Ping","inputs":[],"anonymous":false}]"#);
        assert!(
            out.contains("\"github.com/ethereum/go-ethereum/common\""),
            "{out}"
        );
        assert!(
            out.contains("TokenPingEventTopic = common.HexToHash(\"0x"),
            "{out}"
        );
    }

    #[test]
    fn embeds_signatures_and_selectors() {
        let out = erc20();
        assert!(out.contains("ERC20TransferSignature"), "{out}");
        assert!(out.contains("= \"transfer(address,uint256)\""), "{out}");
        assert!(out.contains("= [4]byte{0xa9, 0x05, 0x9c, 0xbb}"), "{out}");
        assert!(out.contains("const ERC20ABI = `["), "{out}");
    }

    #[test]
    fn integer_widths_follow_abigen() {
        let out = render(
            r#"[{"type":"function","name":"f","inputs":[
                {"name":"a","type":"uint24"},{"name":"b","type":"uint32"},{"name":"c","type":"int128"},{"name":"d","type":"int8"}
            ],"outputs":[],"stateMutability":"nonpayable"}]"#,
        );
        assert!(out.contains("\tA *big.Int\n"), "{out}");
        assert!(out.contains("\tB uint32\n"), "{out}");
        assert!(out.contains("\tC *big.Int\n"), "{out}");
        assert!(out.contains("\tD int8\n"), "{out}");
    }

    #[test]
    fn fields_align_like_gofmt() {
        let out = render(
            r#"[{"type":"function","name":"f","inputs":[{"name":"a","type":"bool"},{"name":"longName","type":"string"}],"outputs":[],"stateMutability":"nonpayable"}]"#,
        );
        assert!(
            out.contains("\tA        bool\n\tLongName string\n"),
            "{out}"
        );
    }

    #[test]
    fn acronyms_survive_and_unnamed_params_use_types() {
        let out = render(
            r#"[{"type":"function","name":"tokenURI","inputs":[{"name":"","type":"uint256"},{"name":"","type":"address"},{"name":"","type":"address"}],"outputs":[],"stateMutability":"view"}]"#,
        );
        assert!(out.contains("type TokenTokenURIParams struct"), "{out}");
        assert!(out.contains("\tUint256  *big.Int\n"), "{out}");
        assert!(out.contains("\tAddress  common.Address\n"), "{out}");
        assert!(out.contains("\tAddress2 common.Address\n"), "{out}");
        assert!(!out.contains("Arg0"), "{out}");
    }

    #[test]
    fn overloads_follow_abigen_numbering() {
        let out = render(
            r#"[
            {"type":"function","name":"safeTransferFrom","inputs":[{"name":"from","type":"address"},{"name":"to","type":"address"},{"name":"id","type":"uint256"}],"outputs":[],"stateMutability":"nonpayable"},
            {"type":"function","name":"safeTransferFrom","inputs":[{"name":"from","type":"address"},{"name":"to","type":"address"},{"name":"id","type":"uint256"},{"name":"data","type":"bytes"}],"outputs":[],"stateMutability":"nonpayable"}
        ]"#,
        );
        assert!(
            out.contains("type TokenSafeTransferFromParams struct"),
            "{out}"
        );
        assert!(
            out.contains("type TokenSafeTransferFrom0Params struct"),
            "{out}"
        );
        assert!(!out.contains("AddressAddress"), "{out}");
    }

    #[test]
    fn tuples_become_named_structs() {
        let out = render(
            r#"[{"type":"function","name":"deposit","inputs":[{"name":"position","type":"tuple[]","internalType":"struct Vault.Position[]","components":[
                {"name":"shares","type":"uint256"},{"name":"owner","type":"address"}
            ]}],"outputs":[],"stateMutability":"nonpayable"}]"#,
        );
        assert!(
            out.contains(
                "type TokenVaultPosition struct {\n\tShares *big.Int\n\tOwner  common.Address\n}"
            ),
            "{out}"
        );
        assert!(out.contains("\tPosition []TokenVaultPosition\n"), "{out}");
        assert!(
            !out.contains("struct {\n\t\t"),
            "no anonymous nested structs:\n{out}"
        );
    }

    #[test]
    fn function_and_event_with_same_name_do_not_collide() {
        let out = render(
            r#"[
            {"type":"function","name":"transfer","inputs":[{"name":"to","type":"address"}],"outputs":[],"stateMutability":"nonpayable"},
            {"type":"event","name":"Transfer","inputs":[{"name":"to","type":"address","indexed":true}],"anonymous":false}
        ]"#,
        );
        assert!(out.contains("TokenTransferSignature "), "{out}");
        assert!(out.contains("TokenTransferEventSignature "), "{out}");
        assert!(out.contains("type TokenTransferParams struct"), "{out}");
        assert!(out.contains("type TokenTransferEvent struct"), "{out}");
    }

    #[test]
    fn colliding_field_names_get_suffixes() {
        let out = render(
            r#"[{"type":"function","name":"f","inputs":[{"name":"_owner","type":"address"},{"name":"owner","type":"address"}],"outputs":[],"stateMutability":"nonpayable"}]"#,
        );
        assert!(
            out.contains("\tOwner  common.Address\n\tOwner2 common.Address\n"),
            "{out}"
        );
    }

    #[test]
    fn empty_event_renders_empty_struct() {
        let out = render(r#"[{"type":"event","name":"Ping","inputs":[],"anonymous":false}]"#);
        assert!(out.contains("type TokenPingEvent struct{}"), "{out}");
    }

    #[test]
    fn backtick_in_abi_falls_back_to_quoted_string() {
        assert_eq!(go_raw_or_quoted("a`b"), "\"a`b\"");
        assert_eq!(go_raw_or_quoted("ab"), "`ab`");
    }

    #[test]
    fn output_ends_with_single_newline() {
        let out = erc20();
        assert!(out.ends_with("}\n") && !out.ends_with("\n\n"), "{out}");
    }
}
