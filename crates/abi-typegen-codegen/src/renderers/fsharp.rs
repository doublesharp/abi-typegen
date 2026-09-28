//! F# records and Nethereum contract functions.
use crate::tuples::TupleRegistry;
use abi_typegen_core::types::{ContractIr, SolType};
use heck::ToUpperCamelCase;
use std::collections::HashSet;
mod wrappers;

/// Returns a safe F# module and filename stem.
pub fn namespace_name(name: &str) -> String {
    let stem = name.to_upper_camel_case();
    if stem.is_empty() || !stem.starts_with(|c: char| c.is_ascii_alphabetic()) {
        format!("Contract{stem}")
    } else {
        stem
    }
}
fn unique(base: String, used: &mut HashSet<String>) -> String {
    let mut name = base.clone();
    let mut i = 2;
    while !used.insert(name.clone()) {
        name = format!("{base}{i}");
        i += 1;
    }
    name
}
fn fields<'a>(names: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut used = HashSet::from(["Tag".into()]);
    names
        .enumerate()
        .map(|(i, n)| {
            unique(
                if n.is_empty() {
                    format!("Arg{i}")
                } else {
                    namespace_name(n)
                },
                &mut used,
            )
        })
        .collect()
}
fn quoted(s: &str) -> String {
    serde_json::to_string(s).expect("strings serialize")
}
fn solidity(ty: &SolType) -> String {
    match ty {
        SolType::Uint(n) => format!("uint{n}"),
        SolType::Int(n) => format!("int{n}"),
        SolType::Bool => "bool".into(),
        SolType::Address => "address".into(),
        SolType::StringType => "string".into(),
        SolType::Bytes => "bytes".into(),
        SolType::BytesN(n) => format!("bytes{n}"),
        SolType::Tuple(_) => "tuple".into(),
        SolType::Array(t) => format!("{}[]", solidity(t)),
        SolType::FixedArray(t, n) => format!("{}[{n}]", solidity(t)),
    }
}
fn map_type(ty: &SolType, internal: Option<&str>, registry: &TupleRegistry) -> String {
    match ty {
        SolType::Uint(_) | SolType::Int(_) => "BigInteger".into(),
        SolType::Bool => "bool".into(),
        SolType::Address | SolType::StringType => "string".into(),
        SolType::Bytes | SolType::BytesN(_) => "byte array".into(),
        SolType::Array(t) | SolType::FixedArray(t, _) => {
            format!("ResizeArray<{}>", map_type(t, internal, registry))
        }
        SolType::Tuple(c) => format!("Tuple{}", registry.name(c, internal)),
    }
}
struct Field<'a> {
    name: &'a str,
    ty: &'a SolType,
    internal: Option<&'a str>,
    indexed: bool,
}
fn indexed_hash(ty: &SolType) -> bool {
    matches!(
        ty,
        SolType::Array(_)
            | SolType::FixedArray(_, _)
            | SolType::Tuple(_)
            | SolType::Bytes
            | SolType::StringType
    )
}
fn record(
    out: &mut String,
    name: &str,
    params: &[Field<'_>],
    attr: &str,
    interface: &str,
    registry: &TupleRegistry,
) {
    if !attr.is_empty() {
        out.push_str(&format!("[<{attr}>]\n"));
    }
    if params.is_empty() {
        out.push_str(&format!("type {name}() =\n"));
        if interface.is_empty() {
            out.push_str("    class end\n\n");
        } else {
            out.push_str(&format!("    interface {interface}\n\n"));
        }
        return;
    }
    out.push_str(&format!("[<CLIMutable>]\ntype {name} =\n    {{\n"));
    for (i, (p, n)) in params
        .iter()
        .zip(fields(params.iter().map(|p| p.name)))
        .enumerate()
    {
        let hash = p.indexed && indexed_hash(p.ty);
        let ty = if hash {
            "string".into()
        } else {
            map_type(p.ty, p.internal, registry)
        };
        out.push_str(&format!(
            "        [<Parameter({}, {}, {}, {})>]\n        {}: {}\n",
            quoted(&solidity(p.ty)),
            quoted(p.name),
            i + 1,
            p.indexed,
            n,
            ty
        ));
    }
    out.push_str("    }\n");
    if !interface.is_empty() {
        out.push_str(&format!("    interface {interface}\n"));
    }
    out.push('\n');
}
fn params(ps: &[abi_typegen_core::types::AbiParam]) -> Vec<Field<'_>> {
    ps.iter()
        .map(|p| Field {
            name: &p.name,
            ty: &p.ty,
            internal: p.internal_type.as_deref(),
            indexed: false,
        })
        .collect()
}
fn stems<'a>(
    names: impl Iterator<Item = &'a str>,
    suffixes: &[&str],
    used: &mut HashSet<String>,
) -> Vec<String> {
    names
        .map(|n| {
            let base = namespace_name(n);
            let mut stem = base.clone();
            let mut i = 2;
            while suffixes
                .iter()
                .any(|suffix| used.contains(&format!("{stem}{suffix}")))
            {
                stem = format!("{base}{i}");
                i += 1;
            }
            for suffix in suffixes {
                used.insert(format!("{stem}{suffix}"));
            }
            stem
        })
        .collect()
}
/// Generates F# ABI records and optional Nethereum encoding, decoding and RPC functions.
pub fn render_fsharp_file(ir: &ContractIr, wrappers: bool) -> String {
    let registry = TupleRegistry::new(ir);
    let mut out = format!(
        "// Auto-generated by abi-typegen.\nmodule Contracts.{}\n\nopen System\nopen System.Numerics\nopen Nethereum.ABI.FunctionEncoding.Attributes\nopen Nethereum.Contracts\nopen Nethereum.Web3\nopen Nethereum.RPC.Eth.DTOs\nopen Nethereum.Hex.HexTypes\n\nlet abi = {}\n\n",
        namespace_name(&ir.name),
        quoted(&serde_json::to_string(&ir.raw_abi).expect("ABI serializes"))
    );
    for def in registry.defs() {
        let ps = def
            .components
            .iter()
            .map(|p| Field {
                name: &p.name,
                ty: &p.ty,
                internal: p.internal_type.as_deref(),
                indexed: false,
            })
            .collect::<Vec<_>>();
        record(
            &mut out,
            &format!("Tuple{}", def.name),
            &ps,
            &format!("Struct({})", quoted(&def.name)),
            "",
            &registry,
        );
    }
    if let Some(c) = &ir.constructor {
        record(
            &mut out,
            "ConstructorParams",
            &params(&c.inputs),
            "",
            "",
            &registry,
        );
    }
    let mut used = registry
        .defs()
        .iter()
        .map(|d| format!("Tuple{}", d.name))
        .collect::<HashSet<_>>();
    used.insert("ConstructorParams".into());
    if ir.constructor.is_some() {
        used.insert("DeploymentParams".into());
    }
    let functions = stems(
        ir.functions.iter().map(|f| f.name.as_str()),
        &["Params", "Result"],
        &mut used,
    );
    for (f, n) in ir.functions.iter().zip(&functions) {
        record(
            &mut out,
            &format!("{n}Params"),
            &params(&f.inputs),
            "",
            "",
            &registry,
        );
        record(
            &mut out,
            &format!("{n}Result"),
            &params(&f.outputs),
            "FunctionOutput",
            "IFunctionOutputDTO",
            &registry,
        );
    }
    let events = stems(
        ir.events.iter().map(|e| e.name.as_str()),
        &["Event", "TopicDecoder"],
        &mut used,
    );
    for (e, n) in ir.events.iter().zip(&events) {
        let ps = e
            .inputs
            .iter()
            .map(|p| Field {
                name: &p.name,
                ty: &p.ty,
                internal: p.internal_type.as_deref(),
                indexed: p.indexed,
            })
            .collect::<Vec<_>>();
        record(
            &mut out,
            &format!("{n}Event"),
            &ps,
            &format!("Event({}, {})", quoted(&e.name), e.anonymous),
            "IEventDTO",
            &registry,
        );
        if wrappers && e.inputs.iter().any(|p| p.indexed && indexed_hash(&p.ty)) {
            // Nethereum decodes static arrays as values instead of topic hashes.
            // Keep the public DTO's original ABI attributes and adapt only the
            // private transport DTO. Wrappers select events using the raw ABI.
            let transport = ps
                .iter()
                .map(|p| Field {
                    name: p.name,
                    ty: if p.indexed && indexed_hash(p.ty) {
                        &SolType::Bytes
                    } else {
                        p.ty
                    },
                    internal: p.internal,
                    indexed: p.indexed,
                })
                .collect::<Vec<_>>();
            record(
                &mut out,
                &format!("private {n}TopicDecoder"),
                &transport,
                &format!("Event({}, {})", quoted(&e.name), e.anonymous),
                "IEventDTO",
                &registry,
            );
        }
    }
    let errors = stems(
        ir.errors.iter().map(|e| e.name.as_str()),
        &["Error"],
        &mut used,
    );
    for (e, n) in ir.errors.iter().zip(&errors) {
        record(
            &mut out,
            &format!("{n}Error"),
            &params(&e.inputs),
            &format!("Error({})", quoted(&e.name)),
            "IErrorDTO",
            &registry,
        );
    }
    if wrappers {
        out.push_str(&wrappers::render(ir, &functions, &events, &errors));
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    use abi_typegen_core::parser::parse_artifact;
    #[test]
    fn fsharp_constructor_helpers_do_not_collide_with_abi_functions() {
        let ir = parse_artifact("Sample", r#"{"abi":[{"type":"constructor","inputs":[]},{"type":"function","name":"deployment","inputs":[],"outputs":[],"stateMutability":"view"}]}"#).expect("valid ABI");
        let source = render_fsharp_file(&ir, true);
        assert_eq!(source.matches("let encodeDeployment (").count(), 1);
    }
    #[test]
    fn fsharp_resolves_generated_type_collisions() {
        let ir = parse_artifact("Sample", r#"{"abi":[{"type":"constructor","inputs":[]},{"type":"function","name":"constructor","inputs":[],"outputs":[],"stateMutability":"view"},{"type":"function","name":"tupleFoo","inputs":[{"name":"foo","type":"tuple","internalType":"struct FooParams","components":[{"name":"tag","type":"uint8"},{"name":"tag","type":"uint8"}]}],"outputs":[],"stateMutability":"view"}]}"#).expect("valid ABI");
        let source = render_fsharp_file(&ir, true);
        let definitions = source
            .lines()
            .filter(|line| line.starts_with("type "))
            .map(|line| {
                line.split_whitespace()
                    .nth(1)
                    .expect("type name")
                    .trim_end_matches("()")
            })
            .collect::<Vec<_>>();
        let unique = definitions.iter().collect::<HashSet<_>>();
        assert_eq!(
            definitions.len(),
            unique.len(),
            "duplicate generated F# type"
        );
        assert!(source.contains("Tag2: BigInteger"));
        assert!(source.contains("Tag3: BigInteger"));
    }
    #[test]
    fn fsharp_preserves_types_without_wrappers() {
        let ir = parse_artifact("Sample", r#"{"abi":[{"type":"function","name":"balance","inputs":[{"name":"type","type":"address"}],"outputs":[{"name":"amount","type":"uint256"}],"stateMutability":"view"}]}"#).expect("valid ABI");
        let plain = render_fsharp_file(&ir, false);
        assert!(plain.contains("let abi ="));
        assert!(plain.contains("type BalanceParams"));
        assert!(plain.contains("[<CLIMutable>]"));
        assert!(!plain.contains("let encodeBalance"));
        let wrapped = render_fsharp_file(&ir, true);
        assert!(wrapped.contains("let encodeBalance"));
        assert!(wrapped.contains("let callBalance"));
        assert!(wrapped.contains("let decodeBalanceResult"));
    }
}
