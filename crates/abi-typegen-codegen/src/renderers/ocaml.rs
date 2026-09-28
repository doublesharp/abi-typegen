//! OCaml bindings using Zarith integers and the checked shared ABI runtime.
use crate::{naming::Scope, tuples::TupleRegistry};
use abi_typegen_core::types::{ContractIr, SolType, StateMutability};
use heck::ToSnakeCase;

/// OCaml module and its companion C foreign-function stubs.
pub struct OcamlArtifacts {
    /// Contract module, saved with an OCaml-compatible module filename.
    pub source: String,
    /// C stubs; empty when wrappers are disabled. Requires `abi_typegen.h`.
    pub bridge: String,
}
fn ident(s: &str) -> String {
    let s: String = s
        .to_snake_case()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format!("atg_{s}")
}
fn literal(s: &str) -> String {
    let mut out = String::from("\"");
    for b in s.bytes() {
        match b {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            32..=126 => out.push(char::from(b)),
            _ => out.push_str(&format!("\\{b:03}")),
        }
    }
    out.push('"');
    out
}
struct Types {
    registry: TupleRegistry,
    known: Vec<(SolType, String)>,
    names: Scope,
    definitions: String,
    codecs: String,
}
impl Types {
    fn ty(&mut self, t: &SolType) -> String {
        if let Some((_, n)) = self.known.iter().find(|(k, _)| k == t) {
            return n.clone();
        }
        let candidate = if let SolType::Tuple(fields) = t {
            self.registry
                .defs()
                .iter()
                .find(|d| &d.components == fields)
                .map(|d| ident(&d.name))
                .unwrap_or_else(|| format!("atg_tuple_{}", self.known.len()))
        } else {
            format!("atg_type_{}", self.known.len())
        };
        let name = self.names.claim(&candidate);
        // Reserve before descending so primitive and enclosing type names differ.
        self.known.push((t.clone(), name.clone()));
        let (def, enc, dec) = match t {
            SolType::Uint(n) | SolType::Int(n) => {
                let signed = matches!(t, SolType::Int(_));
                (
                    "Z.t".into(),
                    format!("word {signed} {n} x"),
                    format!("integer {signed} x"),
                )
            }
            SolType::Bool => ("bool".into(), "Bool x".into(), "boolean x".into()),
            SolType::Address => (
                "string".into(),
                "Word (String.make 12 '\\000' ^ checked_bytes 20 x)".into(),
                "address x".into(),
            ),
            SolType::Bytes | SolType::StringType => {
                ("string".into(), "Bytes x".into(), "bytes x".into())
            }
            SolType::BytesN(n) => (
                "string".into(),
                format!("Bytes (checked_bytes {n} x)"),
                format!("checked_bytes {n} (bytes x)"),
            ),
            SolType::Array(inner) | SolType::FixedArray(inner, _) => {
                let child = self.ty(inner);
                let n = if let SolType::FixedArray(_, n) = t {
                    Some(n)
                } else {
                    None
                };
                let src = n
                    .map(|n| format!("(fixed {n} x)"))
                    .unwrap_or_else(|| "x".into());
                let dst = n
                    .map(|n| format!("fixed {n} (sequence x)"))
                    .unwrap_or_else(|| "sequence x".into());
                (
                    format!("{child} array"),
                    format!("Seq (Array.map __atg_encode_{child} {src})"),
                    format!("Array.map __atg_decode_{child} ({dst})"),
                )
            }
            SolType::Tuple(fields) => {
                if fields.is_empty() {
                    (
                        "unit".into(),
                        "let () = x in Seq [||]".into(),
                        "let _ = fixed 0 (sequence x) in ()".into(),
                    )
                } else {
                    let mut scope = Scope::default();
                    let fields: Vec<_> = fields
                        .iter()
                        .map(|f| {
                            (
                                scope.claim(&format!("{name}_{}", ident(&f.name))),
                                self.ty(&f.ty),
                            )
                        })
                        .collect();
                    let def = format!(
                        "{{ {} }}",
                        fields
                            .iter()
                            .map(|(f, t)| format!("{f}: {t}"))
                            .collect::<Vec<_>>()
                            .join("; ")
                    );
                    let enc = format!(
                        "Seq [| {} |]",
                        fields
                            .iter()
                            .map(|(f, t)| format!("__atg_encode_{t} x.{f}"))
                            .collect::<Vec<_>>()
                            .join("; ")
                    );
                    let dec = format!(
                        "let a = fixed {} (sequence x) in {{ {} }}",
                        fields.len(),
                        fields
                            .iter()
                            .enumerate()
                            .map(|(i, (f, t))| format!("{f} = __atg_decode_{t} a.({i})"))
                            .collect::<Vec<_>>()
                            .join("; ")
                    );
                    (def, enc, dec)
                }
            }
        };
        self.definitions.push_str(&format!("type {name} = {def}\n"));
        self.codecs.push_str(&format!(
            "let __atg_encode_{name} (x : {name}) = {enc}\nlet __atg_decode_{name} x : {name} = {dec}\n"
        ));
        name
    }
    fn params(&mut self, params: &[abi_typegen_core::types::AbiParam]) -> String {
        self.ty(&SolType::Tuple(
            params
                .iter()
                .map(|p| abi_typegen_core::types::TupleComponent {
                    name: p.name.clone(),
                    ty: p.ty.clone(),
                    internal_type: p.internal_type.clone(),
                })
                .collect(),
        ))
    }
}
/// Generates typed OCaml ABI codecs and transport callbacks, plus C runtime stubs.
pub fn render_ocaml_artifacts(ir: &ContractIr, wrappers: bool) -> OcamlArtifacts {
    let mut types = Types {
        registry: TupleRegistry::new(ir),
        known: Vec::new(),
        names: Scope::default(),
        definitions: String::new(),
        codecs: String::new(),
    };
    let mut names = Scope::default();
    let mut methods = String::new();
    for f in &ir.functions {
        let name = names.claim(&ident(&f.name));
        let input = types.params(&f.inputs);
        let output = types.params(&f.outputs);
        let signature = literal(&f.signature());
        methods.push_str(&format!("let {name}_signature = {signature}\nlet encode_{name} args = encode_request (0, abi, {name}_signature, __atg_encode_{input} args, \"\", \"\")\nlet decode_{name} data = __atg_decode_{output} (decode_request (1, abi, {name}_signature, Seq [||], data, \"\"))\n"));
        let write = !matches!(
            f.state_mutability,
            StateMutability::View | StateMutability::Pure
        );
        let check = if matches!(f.state_mutability, StateMutability::Payable) {
            ""
        } else {
            "if not (Z.equal value Z.zero) then invalid_arg \"nonpayable call\"; "
        };
        let result = if write {
            "result".into()
        } else {
            format!("decode_{name} result")
        };
        methods.push_str(&format!("let call_{name} (transport : transport) ~address ?(value=Z.zero) args = {check}let _ = word false 256 value in let result = transport ~write:{write} ~address:(checked_bytes 20 address) ~data:(encode_{name} args) ~value in {result}\n"));
    }
    for e in &ir.errors {
        let name = names.claim(&format!("{}_error", ident(&e.name)));
        let output = types.params(&e.inputs);
        methods.push_str(&format!("let decode_{name} data = __atg_decode_{output} (decode_request (2, abi, {}, Seq [||], data, \"\"))\n",literal(&e.signature())));
    }
    for e in &ir.events {
        let name = names.claim(&format!("{}_event", ident(&e.name)));
        let params = e
            .inputs
            .iter()
            .map(|p| abi_typegen_core::types::AbiParam {
                name: p.name.clone(),
                internal_type: p.internal_type.clone(),
                ty: if p.indexed
                    && matches!(
                        p.ty,
                        SolType::StringType
                            | SolType::Bytes
                            | SolType::Array(_)
                            | SolType::FixedArray(..)
                            | SolType::Tuple(_)
                    ) {
                    SolType::BytesN(32)
                } else {
                    p.ty.clone()
                },
            })
            .collect::<Vec<_>>();
        let output = types.params(&params);
        methods.push_str(&format!("let decode_{name} ~topics data = __atg_decode_{output} (decode_request (3, abi, {}, Seq [||], data, String.concat \"\" (List.map (checked_bytes 32) topics)))\n",literal(&e.signature())));
    }
    if let Some(c) = &ir.constructor {
        let input = types.params(&c.inputs);
        methods.push_str(&format!("let encode_constructor ~bytecode args = encode_request (4, abi, \"\", __atg_encode_{input} args, bytecode, \"\")\n"));
    }
    let prefix = format!("ocaml_{}", ident(&ir.name));
    let abi = serde_json::to_string(&ir.raw_abi).expect("ABI JSON serializes");
    let mut source = format!(
        "(* Generated by abi-typegen. Integers use Zarith; bytes and addresses use raw strings. *)\nlet abi = {}\n{}",
        literal(&abi),
        types.definitions
    );
    if wrappers {
        source.push_str(include_str!("ocaml/runtime.ml"));
        source.push_str(&format!("external encode_request : int * string * string * abi_value * string * string -> string = \"{prefix}_encode\"\nexternal decode_request : int * string * string * abi_value * string * string -> abi_value = \"{prefix}_decode\"\n"));
        source.push_str(&types.codecs);
        source.push_str(&methods);
    }
    OcamlArtifacts {
        source,
        bridge: if wrappers {
            include_str!("ocaml/bridge.c").replace("ATG_PREFIX", &prefix)
        } else {
            String::new()
        },
    }
}

/// Returns the generated OCaml compilation unit name.
pub fn namespace_name(name: &str) -> String {
    format!("A{}", &ident(name)[1..])
}
/// Returns the generated OCaml source filename.
pub fn file_name(name: &str) -> String {
    format!("{}.ml", namespace_name(name))
}
