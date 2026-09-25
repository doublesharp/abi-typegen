//! C11 contract bindings backed by the shared ABI runtime.
use crate::naming::Scope;
use abi_typegen_core::types::{AbiParam, ContractIr, SolType, StateMutability};

/// Shared C runtime declarations shipped alongside generated contract headers.
pub const RUNTIME_HEADER: &str = include_str!("c/abi_typegen.h");

fn ident(name: &str) -> String {
    let name: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let name = name
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    format!("atg_{}", if name.is_empty() { "field" } else { &name })
}
fn literal(s: &str) -> String {
    serde_json::to_string(s).expect("string serializes")
}
struct Types {
    prefix: String,
    definitions: String,
    codecs: String,
    count: usize,
    known: Vec<(SolType, String)>,
    tuples: crate::tuples::TupleRegistry,
}
impl Types {
    fn new(prefix: String, ir: &ContractIr) -> Self {
        Self {
            prefix,
            definitions: String::new(),
            codecs: String::new(),
            count: 0,
            known: Vec::new(),
            tuples: crate::tuples::TupleRegistry::new(ir),
        }
    }
    fn ty(&mut self, ty: &SolType) -> String {
        if let Some((_, name)) = self.known.iter().find(|(known, _)| known == ty) {
            return name.clone();
        }
        self.count += 1;
        let name = if let SolType::Tuple(fields) = ty {
            self.tuples
                .defs()
                .iter()
                .find(|def| &def.components == fields)
                .map(|def| format!("{}_tuple_{}", self.prefix, ident(&def.name)))
                .unwrap_or_else(|| format!("{}_type_{}", self.prefix, self.count))
        } else {
            format!("{}_type_{}", self.prefix, self.count)
        };
        let (definition, encode, decode) = match ty {
            SolType::Bool => (format!("typedef bool {name};\n"), "return atg_value_bool(*x);".into(), "if (atg_value_kind(v) != 1) return false; *x = atg_value_get_bool(v) != 0; return true;".into()),
            SolType::Uint(_) | SolType::Int(_) => (format!("typedef atg_word {name};\n"), "return atg_value_word(x->bytes);".into(), "if (atg_value_kind(v) != 2) return false; memcpy(x->bytes, atg_value_data(v), 32); return true;".into()),
            SolType::Address => (format!("typedef atg_address {name};\n"), "uint8_t word[32] = {0}; memcpy(word + 12, x->bytes, 20); return atg_value_word(word);".into(), "if (atg_value_kind(v) != 2) return false; memcpy(x->bytes, atg_value_data(v) + 12, 20); return true;".into()),
            SolType::Bytes | SolType::StringType => (format!("typedef atg_bytes {name};\n"), "return atg_value_bytes(x->data, x->len);".into(), "if (atg_value_kind(v) != 3) return false; x->data = atg_value_data(v); x->len = atg_value_len(v); return true;".into()),
            SolType::BytesN(n) => (format!("typedef struct {{ uint8_t bytes[{n}]; }} {name};\n"), format!("return atg_value_bytes(x->bytes, {n});"), format!("if (atg_value_kind(v) != 3 || atg_value_len(v) != {n}) return false; memcpy(x->bytes, atg_value_data(v), {n}); return true;")),
            SolType::Tuple(fields) => {
                let mut scope = Scope::default();
                let fields: Vec<_> = fields.iter().enumerate().map(|(i,f)| (scope.claim(&ident(if f.name.is_empty() { "field" } else { &f.name })), self.ty(&f.ty), i)).collect();
                self.structure(&name,&fields)
            }
            SolType::Array(inner) | SolType::FixedArray(inner, _) => {
                let child = self.ty(inner);
                let (definition,len,allocate) = match ty {
                    SolType::FixedArray(_, n) => (format!("typedef struct {{ {child} data[{}]; }} {name};\n",(*n).max(1)),n.to_string(),format!("if (atg_value_len(v) != {n}) return false;")),
                    _ => (format!("typedef struct {{ {child} *data; size_t len; }} {name};\n"),"x->len".into(),format!("x->len = atg_value_len(v); if (x->len > SIZE_MAX / sizeof({child})) return false; x->data = ({child} *)atg_result_alloc(owner, x->len * sizeof({child}), ATG_ALIGNOF({child})); if (x->len && !x->data) return false;")),
                };
                let encode = format!("atg_value *seq = atg_value_seq(); if (!seq) return NULL; if ({len} && !x->data) {{ atg_value_free(seq); return NULL; }} for (size_t i = 0; i < {len}; ++i) {{ atg_value *item = {child}_to_value(&x->data[i]); if (!item || atg_value_push(seq, item)) {{ atg_value_free(item); atg_value_free(seq); return NULL; }} atg_value_free(item); }} return seq;");
                // Fixed arrays cannot be NULL; omit the redundant condition to keep -Werror builds clean.
                let encode = if matches!(ty,SolType::FixedArray(..)) { encode.replace(&format!("if ({len} && !x->data) {{ atg_value_free(seq); return NULL; }} "),"") } else { encode };
                let decode = format!("if (atg_value_kind(v) != 4) return false; {allocate} for (size_t i = 0; i < {len}; ++i) {{ if (!{child}_from_value(owner, atg_value_at(v,i), &x->data[i])) return false; }} return true;");
                (definition,encode,decode)
            }
        };
        self.known.push((ty.clone(), name.clone()));
        self.definitions.push_str(&definition);
        self.codecs.push_str(&format!("static inline atg_value *{name}_to_value(const {name} *x) {{ {encode} }}\nstatic inline bool {name}_from_value(atg_result *owner, const atg_value *v, {name} *x) {{ (void)owner; {decode} }}\n"));
        name
    }
    fn structure(
        &self,
        name: &str,
        fields: &[(String, String, usize)],
    ) -> (String, String, String) {
        let mut definition = String::from("typedef struct {\n");
        for (field, ty, _) in fields {
            definition.push_str(&format!("    {ty} {field};\n"));
        }
        if fields.is_empty() {
            definition.push_str("    unsigned char reserved;\n");
        }
        definition.push_str(&format!("}} {name};\n"));
        let mut encode =
            "(void)x; atg_value *seq = atg_value_seq(); if (!seq) return NULL;".to_string();
        let mut decode = format!(
            "(void)x; if (atg_value_kind(v) != 4 || atg_value_len(v) != {}) return false;",
            fields.len()
        );
        for (field, ty, index) in fields {
            encode.push_str(&format!(" {{ atg_value *item = {ty}_to_value(&x->{field}); if (!item || atg_value_push(seq,item)) {{ atg_value_free(item); atg_value_free(seq); return NULL; }} atg_value_free(item); }}"));
            decode.push_str(&format!(
                " if (!{ty}_from_value(owner,atg_value_at(v,{index}),&x->{field})) return false;"
            ));
        }
        encode.push_str(" return seq;");
        decode.push_str(" return true;");
        (definition, encode, decode)
    }
    fn params(&mut self, name: &str, params: &[AbiParam]) {
        let mut scope = Scope::default();
        let fields: Vec<_> = params
            .iter()
            .enumerate()
            .map(|(i, p)| {
                (
                    scope.claim(&ident(if p.name.is_empty() { "field" } else { &p.name })),
                    self.ty(&p.ty),
                    i,
                )
            })
            .collect();
        let (definition, encode, decode) = self.structure(name, &fields);
        self.definitions.push_str(&definition);
        self.codecs.push_str(&format!("static inline atg_value *{name}_to_value(const {name} *x) {{ {encode} }}\nstatic inline bool {name}_from_value(atg_result *owner, const atg_value *v, {name} *x) {{ (void)owner; {decode} }}\n"));
    }
}
pub(super) fn claim_item(scope: &mut Scope, base: &str) -> String {
    scope.claim_family(
        base,
        &[
            "",
            "_params",
            "_returns",
            "_fields",
            "_signature",
            "_selector",
            "_topic",
            "_encode",
            "_decode",
            "_filter",
            "_params_to_value",
            "_params_from_value",
            "_returns_to_value",
            "_returns_from_value",
            "_fields_to_value",
            "_fields_from_value",
        ],
    )
}

/// Return the C prefix, also used as the C++ namespace.
pub fn namespace_name(name: &str) -> String {
    ident(name)
}
/// Render a C11 header with metadata, value types and optional callable bindings.
pub fn render_c_file(ir: &ContractIr, wrappers: bool) -> String {
    let prefix = ident(&ir.name);
    let mut types = Types::new(prefix.clone(), ir);
    let mut body = String::new();
    let mut names = Scope::default();
    let constructor_params = format!("{prefix}_constructor_params");
    let inputs = ir
        .constructor
        .as_ref()
        .map_or(&[][..], |c| c.inputs.as_slice());
    types.params(&constructor_params, inputs);
    if wrappers {
        body.push_str(&format!("static inline atg_result *{prefix}_encode_constructor(const uint8_t *bytecode, size_t len, const {constructor_params} *args) {{ if (!args) return atg_result_failure(\"null arguments\"); atg_value *v = {constructor_params}_to_value(args); if (!v) return atg_result_failure(\"invalid constructor arguments\"); atg_result *r = atg_encode_constructor({prefix}_abi,bytecode,len,v); atg_value_free(v); return r; }}\n"));
        let value_check = if ir
            .constructor
            .as_ref()
            .is_some_and(|c| c.state_mutability == StateMutability::Payable)
        {
            ""
        } else {
            "if (options && options->value) { for (size_t i=0;i<32;++i) if (options->value->bytes[i]) return atg_result_failure(\"nonpayable constructor cannot receive value\"); }"
        };
        body.push_str(&format!("static inline atg_result *{prefix}_deploy(atg_transport_fn transport, void *context, const uint8_t *bytecode, size_t len, const {constructor_params} *args, const atg_call_options *options) {{ if (!transport) return atg_result_failure(\"missing transport\"); {value_check} atg_result *encoded = {prefix}_encode_constructor(bytecode,len,args); if (atg_result_error(encoded)) return encoded; atg_result *response = transport(context,NULL,atg_result_data(encoded),atg_result_len(encoded),2,options); atg_result_free(encoded); return response; }}\n"));
    }
    for f in &ir.functions {
        let name = claim_item(&mut names, &format!("{prefix}_{}", ident(&f.name)));
        let params = format!("{name}_params");
        let returns = format!("{name}_returns");
        types.params(&params, &f.inputs);
        types.params(&returns, &f.outputs);
        body.push_str(&format!(
            "static const char {name}_signature[] = {};\n",
            literal(&f.signature())
        ));
        body.push_str(&format!(
            "static const uint8_t {name}_selector[4] = {{{}}};\n",
            f.selector()
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(",")
        ));
        if wrappers {
            body.push_str(&format!("static inline atg_result *{name}_encode(const {params} *args) {{ if (!args) return atg_result_failure(\"null arguments\"); atg_value *v = {params}_to_value(args); if (!v) return atg_result_failure(\"invalid arguments or allocation failure\"); atg_result *r = atg_encode({prefix}_abi, {name}_signature, v); atg_value_free(v); return r; }}\n"));
            body.push_str(&decode_wrapper(
                &name,
                &returns,
                &format!("atg_decode({prefix}_abi,{name}_signature,data,len)"),
                "const uint8_t *data, size_t len",
            ));
            let write = !matches!(
                f.state_mutability,
                StateMutability::View | StateMutability::Pure
            );
            let value_check = if f.state_mutability == StateMutability::Payable {
                ""
            } else {
                "if (options && options->value) { for (size_t i = 0; i < 32; ++i) { if (options->value->bytes[i]) return atg_result_failure(\"nonpayable function cannot receive value\"); } }"
            };
            let out_param = if write {
                String::new()
            } else {
                format!(", {returns} *out")
            };
            let finish = if write {
                "return response;".into()
            } else {
                format!(
                    "atg_result *decoded = {name}_decode(atg_result_data(response),atg_result_len(response),out); atg_result_free(response); return decoded;"
                )
            };
            body.push_str(&format!("static inline atg_result *{name}({prefix}_client *client, const {params} *args, const atg_call_options *options{out_param}) {{ if (!client || !client->transport) return atg_result_failure(\"missing transport\"); {value_check} atg_result *encoded = {name}_encode(args); if (atg_result_error(encoded)) return encoded; atg_result *response = client->transport(client->context,&client->address,atg_result_data(encoded),atg_result_len(encoded),{},options); atg_result_free(encoded); if (atg_result_error(response)) return response; {finish} }}\n",i32::from(write)));
        }
    }
    for e in &ir.errors {
        let name = claim_item(&mut names, &format!("{prefix}_{}_error", ident(&e.name)));
        let params = format!("{name}_fields");
        types.params(&params, &e.inputs);
        body.push_str(&format!(
            "static const char {name}_signature[] = {};\n",
            literal(&e.signature())
        ));
        body.push_str(&format!(
            "static const uint8_t {name}_selector[4] = {{{}}};\n",
            e.selector()
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(",")
        ));
        if wrappers {
            body.push_str(&decode_wrapper(
                &name,
                &params,
                &format!("atg_decode_error({prefix}_abi,{name}_signature,data,len)"),
                "const uint8_t *data, size_t len",
            ));
        }
    }
    for e in &ir.events {
        let name = claim_item(&mut names, &format!("{prefix}_{}_event", ident(&e.name)));
        let params = format!("{name}_fields");
        let fields: Vec<_> = e
            .inputs
            .iter()
            .map(|p| AbiParam {
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
            .collect();
        types.params(&params, &fields);
        body.push_str(&format!(
            "static const char {name}_signature[] = {};\n",
            literal(&e.signature())
        ));
        if !e.anonymous {
            body.push_str(&format!(
                "static const uint8_t {name}_topic[32] = {{{}}};\n",
                e.topic0()
                    .iter()
                    .map(u8::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        if wrappers {
            let indexed = e.inputs.iter().filter(|p| p.indexed).count();
            let start = usize::from(!e.anonymous);
            let filter_params = (0..indexed)
                .map(|i| format!(", const atg_word *topic{i}"))
                .collect::<String>();
            let mut filter_body = format!(
                "if (!out) return false; memset(out,0,sizeof(*out)); out->address = address; out->topic_count = {};",
                indexed + start
            );
            if indexed + start > 4 {
                filter_body.push_str(" return false;");
            } else {
                if !e.anonymous {
                    filter_body.push_str(&format!(
                        " memcpy(out->topics[0].bytes,{name}_topic,32); out->has_topic[0] = 1;"
                    ));
                }
                for i in 0..indexed {
                    let slot = i + start;
                    filter_body.push_str(&format!(" if (topic{i}) {{ out->topics[{slot}] = *topic{i}; out->has_topic[{slot}] = 1; }}"));
                }
                filter_body.push_str(" return true;");
            }
            body.push_str(&format!("/* Indexed dynamic fields require their topic hash; NULL means wildcard. */\nstatic inline bool {name}_filter(atg_address address{filter_params}, atg_log_filter *out) {{ {filter_body} }}\n"));
            body.push_str(&decode_wrapper(
                &name,
                &params,
                &format!("atg_decode_event({prefix}_abi,{name}_signature,topics,count,data,len)"),
                "const uint8_t *topics, size_t count, const uint8_t *data, size_t len",
            ));
        }
    }
    let guard = format!("{}_BINDINGS_H", prefix.to_ascii_uppercase());
    let mut out = format!(
        "/* Generated by abi-typegen. Link abi-typegen-runtime for callable bindings. */\n#ifndef {guard}\n#define {guard}\n#include \"abi_typegen.h\"\n#include <stdbool.h>\n#include <string.h>\n#ifndef ATG_ALIGNOF\n#ifdef __cplusplus\n#define ATG_ALIGNOF(T) alignof(T)\n#else\n#define ATG_ALIGNOF(T) _Alignof(T)\n#endif\n#endif\nstatic const char {prefix}_abi[] = {};\n",
        literal(&serde_json::to_string(&ir.raw_abi).expect("ABI serializes"))
    );
    if wrappers {
        out.push_str(&format!("typedef struct {{ atg_address address; void *context; atg_transport_fn transport; }} {prefix}_client;\n"));
    }
    out.push_str(&types.definitions);
    if wrappers {
        out.push_str(&types.codecs);
    }
    out.push_str(&body);
    out.push_str("#endif\n");
    out
}
fn decode_wrapper(name: &str, ty: &str, call: &str, params: &str) -> String {
    format!(
        "/* Fields borrow storage from the returned result. Keep it alive while using out. */\nstatic inline atg_result *{name}_decode({params}, {ty} *out) {{ if (!out) return atg_result_failure(\"null output\"); atg_result *r = {call}; if (atg_result_error(r)) return r; if (!{ty}_from_value(r,atg_result_value(r),out)) {{ atg_result_free(r); return atg_result_failure(\"decoded type conversion failed\"); }} return r; }}\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_survives_without_wrappers() {
        let ir = abi_typegen_core::parser::parse_artifact("Token",r#"{"abi":[{"type":"function","name":"balanceOf","inputs":[{"name":"owner","type":"address"}],"outputs":[{"name":"amount","type":"uint256"}],"stateMutability":"view"}]}"#).expect("fixture");
        let output = render_c_file(&ir, false);
        assert!(output.contains("atg_Token_abi"));
        assert!(output.contains("atg_Token_atg_balanceOf_returns"));
        assert!(!output.contains("atg_encode("));
        assert!(render_c_file(&ir, true).contains("client->transport"));
    }
}
