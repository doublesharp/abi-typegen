//! GnuCOBOL bindings backed by the shared abi-typegen C ABI.
//!
//! This first pass intentionally targets a small, useful interoperability slice:
//! read-only functions with one `address` argument and one `uint256` result.
//! The generated COBOL stays thin; ABI encoding/decoding remains in the Rust
//! runtime exposed through `c/abi_typegen.h`, while a generated C bridge flattens
//! pointers and result ownership into a COBOL-friendly ABI.

use crate::naming::overload_indices;
use abi_typegen_core::types::{ContractIr, SolType, StateMutability};
use std::collections::HashSet;

mod layout;

/// Pair of source files required by the current COBOL backend.
#[derive(Debug, Clone)]
pub struct CobolArtifacts {
    /// Free-form GnuCOBOL source containing callable subprograms.
    pub cobol: String,
    /// C11 bridge linked with abi-typegen-runtime. Define
    /// `ATG_COBOL_WITH_CURL=1` and link libcurl plus json-c to enable `eth_call`.
    pub bridge: String,
}

#[derive(Debug, Clone)]
pub(super) struct PocFunction {
    pub cobol_stem: String,
    pub bridge_stem: String,
    pub signature: String,
    pub selector_hex: String,
}

fn ascii_words(name: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut previous_lower = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            if c.is_ascii_uppercase() && previous_lower && !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            current.push(c.to_ascii_uppercase());
            previous_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
        } else if !current.is_empty() {
            words.push(std::mem::take(&mut current));
            previous_lower = false;
        } else {
            previous_lower = false;
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn fnv1a32(value: &str) -> u32 {
    let mut hash = 0x811c9dc5u32;
    for byte in value.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

/// A conservative COBOL identifier. Long names get a deterministic suffix so
/// generated PROGRAM-ID names stay friendly to older toolchains as well.
pub fn namespace_name(name: &str) -> String {
    let mut out = ascii_words(name).join("-");
    if out.is_empty() {
        out = "CONTRACT".into();
    }
    if !out.as_bytes()[0].is_ascii_alphabetic() {
        out.insert_str(0, "C-");
    }
    if out.len() > 22 {
        let hash = fnv1a32(&out);
        out = format!("{}-{:08X}", &out[..13], hash);
    }
    out
}

fn c_ident(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str("contract");
    }
    if out.as_bytes()[0].is_ascii_digit() {
        out.insert(0, '_');
    }
    out
}

fn cobol_function_stem(contract: &str, function: &str, overload: Option<usize>) -> String {
    let mut parts = ascii_words(contract);
    parts.extend(ascii_words(function));
    if let Some(index) = overload {
        parts.push(format!("O{index}"));
    }
    let mut raw = parts.join("-");
    if raw.is_empty() || !raw.as_bytes()[0].is_ascii_alphabetic() {
        raw.insert_str(0, "C-");
    }
    shorten_cobol_stem(&raw)
}

fn shorten_cobol_stem(raw: &str) -> String {
    if raw.len() <= 22 {
        raw.to_string()
    } else {
        let hash = fnv1a32(raw);
        format!("{}-{:08X}", &raw[..13], hash)
    }
}

fn bridge_function_stem(contract: &str, function: &str, overload: Option<usize>) -> String {
    let mut out = format!("atg_cb_{}_{}", c_ident(contract), c_ident(function));
    if let Some(index) = overload {
        out.push('_');
        out.push_str(&index.to_string());
    }
    shorten_bridge_stem(&out)
}

fn shorten_bridge_stem(raw: &str) -> String {
    if raw.len() <= 24 {
        raw.to_string()
    } else {
        format!("{}_{:08x}", &raw[..15], fnv1a32(raw))
    }
}

fn poc_functions(ir: &ContractIr) -> Vec<PocFunction> {
    let mut cobol_used = HashSet::new();
    let mut bridge_used = HashSet::new();
    ir.functions
        .iter()
        .zip(overload_indices(&ir.functions))
        .filter(|(f, _)| {
            matches!(
                f.state_mutability,
                StateMutability::View | StateMutability::Pure
            ) && f.inputs.len() == 1
                && matches!(f.inputs[0].ty, SolType::Address)
                && f.outputs.len() == 1
                && matches!(f.outputs[0].ty, SolType::Uint(256))
        })
        .map(|(f, overload)| {
            let cobol_base = cobol_function_stem(&ir.name, &f.name, overload);
            let mut number = 1;
            let cobol_stem = loop {
                let candidate = if number == 1 {
                    cobol_base.clone()
                } else {
                    shorten_cobol_stem(&format!("{cobol_base}-N{number}"))
                };
                if cobol_used.insert(candidate.clone()) {
                    break candidate;
                }
                number += 1;
            };
            let bridge_base = bridge_function_stem(&ir.name, &f.name, overload);
            let mut number = 1;
            let bridge_stem = loop {
                let candidate = if number == 1 {
                    bridge_base.clone()
                } else {
                    shorten_bridge_stem(&format!("{bridge_base}_{number}"))
                };
                if bridge_used.insert(candidate.clone()) {
                    break candidate;
                }
                number += 1;
            };
            PocFunction {
                cobol_stem,
                bridge_stem,
                signature: f.signature(),
                selector_hex: f.selector().iter().map(|b| format!("{b:02X}")).collect(),
            }
        })
        .collect()
}

/// Returns the callable GnuCOBOL `PROGRAM-ID` names emitted for this contract.
///
/// Consumers generating several contracts together can reject collisions
/// across contracts before writing files. COBOL resolves these names without
/// regard to case.
pub fn declared_program_names(ir: &ContractIr, wrappers: bool) -> Vec<String> {
    if !wrappers {
        return Vec::new();
    }
    poc_functions(ir)
        .into_iter()
        .flat_map(|function| {
            ["ENC", "DEC", "CALL"].map(|role| format!("{}-{role}", function.cobol_stem))
        })
        .collect()
}

fn emit_encode_program(out: &mut String, f: &PocFunction) {
    let program = format!("{}-ENC", f.cobol_stem);
    out.push_str(&format!(
        r#"
IDENTIFICATION DIVISION.
PROGRAM-ID. {program}.
DATA DIVISION.
WORKING-STORAGE SECTION.
01  WS-C-STATUS               PIC S9(9) COMP-5.
01  WS-OWNER-CAP              PIC 9(9) COMP-5 VALUE 42.
01  WS-CALLDATA-CAP           PIC 9(9) COMP-5 VALUE 4096.
01  WS-ERROR-CAP              PIC 9(9) COMP-5 VALUE 512.
LINKAGE SECTION.
01  LK-OWNER                  PIC X(42).
01  LK-CALLDATA               PIC X(4096).
01  LK-CALLDATA-LEN           PIC 9(9) COMP-5.
01  LK-ERROR                  PIC X(512).
01  LK-STATUS                 PIC S9(9) COMP-5.
PROCEDURE DIVISION USING
    LK-OWNER
    LK-CALLDATA
    LK-CALLDATA-LEN
    LK-ERROR
    LK-STATUS.
    MOVE SPACES TO LK-ERROR
    MOVE ZERO TO LK-CALLDATA-LEN
    CALL "{bridge}_encode" USING
        BY REFERENCE LK-OWNER
        BY VALUE WS-OWNER-CAP
        BY REFERENCE LK-CALLDATA
        BY VALUE WS-CALLDATA-CAP
        BY REFERENCE LK-CALLDATA-LEN
        BY REFERENCE LK-ERROR
        BY VALUE WS-ERROR-CAP
        RETURNING WS-C-STATUS
    END-CALL
    MOVE WS-C-STATUS TO LK-STATUS
    GOBACK.
END PROGRAM {program}.
"#,
        bridge = f.bridge_stem
    ));
}

fn emit_decode_program(out: &mut String, f: &PocFunction) {
    let program = format!("{}-DEC", f.cobol_stem);
    out.push_str(&format!(
        r#"
IDENTIFICATION DIVISION.
PROGRAM-ID. {program}.
DATA DIVISION.
WORKING-STORAGE SECTION.
01  WS-C-STATUS               PIC S9(9) COMP-5.
01  WS-RESULT-CAP             PIC 9(9) COMP-5 VALUE 4096.
01  WS-DECIMAL-CAP            PIC 9(9) COMP-5 VALUE 78.
01  WS-ERROR-CAP              PIC 9(9) COMP-5 VALUE 512.
LINKAGE SECTION.
01  LK-RESULT                 PIC X(4096).
01  LK-RESULT-LEN             PIC 9(9) COMP-5.
01  LK-DECIMAL                PIC X(78).
01  LK-DECIMAL-LEN            PIC 9(9) COMP-5.
01  LK-ERROR                  PIC X(512).
01  LK-STATUS                 PIC S9(9) COMP-5.
PROCEDURE DIVISION USING
    LK-RESULT
    LK-RESULT-LEN
    LK-DECIMAL
    LK-DECIMAL-LEN
    LK-ERROR
    LK-STATUS.
    MOVE SPACES TO LK-DECIMAL LK-ERROR
    MOVE ZERO TO LK-DECIMAL-LEN
    IF LK-RESULT-LEN > WS-RESULT-CAP
        MOVE "ABI result length exceeds 4096 bytes" TO LK-ERROR
        MOVE -1 TO LK-STATUS
        GOBACK
    END-IF
    CALL "{bridge}_decode" USING
        BY REFERENCE LK-RESULT
        BY VALUE LK-RESULT-LEN
        BY REFERENCE LK-DECIMAL
        BY VALUE WS-DECIMAL-CAP
        BY REFERENCE LK-DECIMAL-LEN
        BY REFERENCE LK-ERROR
        BY VALUE WS-ERROR-CAP
        RETURNING WS-C-STATUS
    END-CALL
    MOVE WS-C-STATUS TO LK-STATUS
    GOBACK.
END PROGRAM {program}.
"#,
        bridge = f.bridge_stem
    ));
}

fn emit_call_program(out: &mut String, f: &PocFunction) {
    let program = format!("{}-CALL", f.cobol_stem);
    out.push_str(&format!(
        r#"
IDENTIFICATION DIVISION.
PROGRAM-ID. {program}.
DATA DIVISION.
WORKING-STORAGE SECTION.
01  WS-C-STATUS               PIC S9(9) COMP-5.
01  WS-RPC-CAP                PIC 9(9) COMP-5 VALUE 256.
01  WS-ADDRESS-CAP            PIC 9(9) COMP-5 VALUE 42.
01  WS-OWNER-CAP              PIC 9(9) COMP-5 VALUE 42.
01  WS-DECIMAL-CAP            PIC 9(9) COMP-5 VALUE 78.
01  WS-ERROR-CAP              PIC 9(9) COMP-5 VALUE 512.
LINKAGE SECTION.
01  LK-RPC-URL                PIC X(256).
01  LK-CONTRACT               PIC X(42).
01  LK-OWNER                  PIC X(42).
01  LK-DECIMAL                PIC X(78).
01  LK-DECIMAL-LEN            PIC 9(9) COMP-5.
01  LK-ERROR                  PIC X(512).
01  LK-STATUS                 PIC S9(9) COMP-5.
PROCEDURE DIVISION USING
    LK-RPC-URL
    LK-CONTRACT
    LK-OWNER
    LK-DECIMAL
    LK-DECIMAL-LEN
    LK-ERROR
    LK-STATUS.
    MOVE SPACES TO LK-DECIMAL LK-ERROR
    MOVE ZERO TO LK-DECIMAL-LEN
    CALL "{bridge}_call" USING
        BY REFERENCE LK-RPC-URL
        BY VALUE WS-RPC-CAP
        BY REFERENCE LK-CONTRACT
        BY VALUE WS-ADDRESS-CAP
        BY REFERENCE LK-OWNER
        BY VALUE WS-OWNER-CAP
        BY REFERENCE LK-DECIMAL
        BY VALUE WS-DECIMAL-CAP
        BY REFERENCE LK-DECIMAL-LEN
        BY REFERENCE LK-ERROR
        BY VALUE WS-ERROR-CAP
        RETURNING WS-C-STATUS
    END-CALL
    MOVE WS-C-STATUS TO LK-STATUS
    GOBACK.
END PROGRAM {program}.
"#,
        bridge = f.bridge_stem
    ));
}

/// Render free-form GnuCOBOL source.
///
/// The initial callable subset is intentionally small: a `view`/`pure` function
/// taking one `address` and returning one `uint256`. This covers ERC-20
/// `balanceOf(address)` while keeping ABI work inside the shared runtime.
pub fn render_cobol_file(ir: &ContractIr, wrappers: bool) -> String {
    let namespace = namespace_name(&ir.name);
    let supported = poc_functions(ir);

    let mut out = format!(
        "*> Generated by abi-typegen. Do not edit.\n*> Contract: {}\n*> COBOL namespace: {namespace}\n*> Runtime ABI: ATG_ABI_VERSION=1\n",
        ir.name
    );

    for (f, overload) in ir.functions.iter().zip(overload_indices(&ir.functions)) {
        let suffix = overload
            .map(|i| format!(" overload {i}"))
            .unwrap_or_default();
        let selector: String = f.selector().iter().map(|b| format!("{b:02X}")).collect();
        out.push_str(&format!(
            "*> FUNCTION{}: {} selector 0x{}\n",
            suffix,
            f.signature(),
            selector
        ));
    }
    for e in &ir.events {
        if e.anonymous {
            out.push_str(&format!("*> EVENT: {} (anonymous)\n", e.signature()));
        } else {
            let topic: String = e.topic0().iter().map(|b| format!("{b:02X}")).collect();
            out.push_str(&format!("*> EVENT: {} topic0 0x{}\n", e.signature(), topic));
        }
    }
    for e in &ir.errors {
        let selector: String = e.selector().iter().map(|b| format!("{b:02X}")).collect();
        out.push_str(&format!(
            "*> ERROR: {} selector 0x{}\n",
            e.signature(),
            selector
        ));
    }

    if !wrappers {
        return out;
    }

    out.push_str(
        "*>\n*> First-pass callable subset: address -> uint256 read functions.\n*> uint256 values are returned as ASCII decimal in a PIC X(78) buffer.\n*> Compile as free-form COBOL and link the generated C bridge plus abi-typegen-runtime.\n",
    );

    if supported.is_empty() {
        out.push_str("*> No functions in this ABI match the current COBOL POC subset.\n");
        return out;
    }

    for f in &supported {
        out.push_str(&format!(
            "*>\n*> {} / selector 0x{}\n",
            f.signature, f.selector_hex
        ));
        emit_encode_program(&mut out, f);
        emit_decode_program(&mut out, f);
        emit_call_program(&mut out, f);
    }
    out
}

/// Render the generated C bridge consumed by the COBOL source.
pub fn render_cobol_bridge_file(ir: &ContractIr, wrappers: bool) -> String {
    layout::render_bridge(ir, &poc_functions(ir), wrappers)
}

/// Render both files required by the COBOL backend.
pub fn render_cobol_artifacts(ir: &ContractIr, wrappers: bool) -> CobolArtifacts {
    CobolArtifacts {
        cobol: render_cobol_file(ir, wrappers),
        bridge: render_cobol_bridge_file(ir, wrappers),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ContractIr {
        abi_typegen_core::parser::parse_artifact(
            "Token",
            r#"{"abi":[
                {"type":"function","name":"balanceOf","inputs":[{"name":"owner","type":"address"}],"outputs":[{"name":"amount","type":"uint256"}],"stateMutability":"view"},
                {"type":"function","name":"symbol","inputs":[],"outputs":[{"name":"","type":"string"}],"stateMutability":"view"}
            ]}"#,
        )
        .expect("fixture")
    }

    #[test]
    fn metadata_mode_contains_signatures_but_no_calls() {
        let ir = fixture();
        let text = render_cobol_file(&ir, false);
        assert!(text.contains("balanceOf(address)"));
        assert!(text.contains("selector 0x70A08231"));
        assert!(!text.contains("_encode\" USING"));
    }

    #[test]
    fn wrappers_emit_balance_of_poc() {
        let ir = fixture();
        assert!(poc_functions(&ir)[0].bridge_stem.len() <= 24);
        let files = render_cobol_artifacts(&ir, true);
        assert!(files.cobol.contains("PIC X(78)"));
        assert!(files.cobol.contains("BALANCE-OF"));
        assert!(files.bridge.contains("atg_encode("));
        assert!(files.bridge.contains("atg_decode("));
        assert!(files.bridge.contains("eth_call"));
        assert!(files.bridge.contains("ATG_COBOL_WITH_CURL"));
    }

    #[test]
    fn unsupported_dynamic_function_stays_metadata_only() {
        let ir = fixture();
        let text = render_cobol_file(&ir, true);
        assert!(text.contains("symbol()"));
        assert!(!text.contains("SYMBOL-CALL"));
    }

    #[test]
    fn decode_rejects_lengths_larger_than_cobol_buffer_and_clears_output_length() {
        let files = render_cobol_artifacts(&fixture(), true);
        assert!(files.cobol.contains("IF LK-RESULT-LEN > WS-RESULT-CAP"));
        assert!(files.cobol.contains("MOVE ZERO TO LK-DECIMAL-LEN"));
        assert!(files.bridge.contains("if (data_len > 4096u)"));
        assert!(files.bridge.contains("if (decimal_len) *decimal_len = 0;"));
    }

    #[test]
    fn similarly_spelled_function_names_get_distinct_programs() {
        let ir = abi_typegen_core::parser::parse_artifact(
            "Token",
            r#"{"abi":[{"type":"function","name":"fooBar","inputs":[{"name":"x","type":"address"}],"outputs":[{"name":"","type":"uint256"}],"stateMutability":"view"},{"type":"function","name":"foo_bar","inputs":[{"name":"x","type":"address"}],"outputs":[{"name":"","type":"uint256"}],"stateMutability":"view"},{"type":"function","name":"_0","inputs":[{"name":"x","type":"address"}],"outputs":[{"name":"","type":"uint256"}],"stateMutability":"view"}]}"#,
        ).expect("ABI");
        let names = declared_program_names(&ir, true);
        assert_eq!(names.len(), 9);
        let folded = names
            .iter()
            .map(|name| name.to_ascii_uppercase())
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(folded.len(), 9);
        assert!(
            names
                .iter()
                .all(|name| name.as_bytes()[0].is_ascii_alphabetic())
        );
        assert!(
            poc_functions(&ir)
                .iter()
                .all(|function| function.bridge_stem.len() <= 24)
        );
    }

    #[test]
    fn bridge_without_callable_functions_contains_no_unused_static_support() {
        let ir = abi_typegen_core::parser::parse_artifact("Empty", r#"{"abi":[]}"#).expect("ABI");
        for wrappers in [false, true] {
            let bridge = render_cobol_bridge_file(&ir, wrappers);
            assert!(bridge.contains("const char atg_cobol_Empty_abi[]"));
            assert!(!bridge.contains("static int atg_cobol_parse_address"));
        }
    }

    #[test]
    fn optional_rpc_transport_uses_strict_json_parser() {
        let bridge = render_cobol_bridge_file(&fixture(), true);
        assert!(bridge.contains("json_tokener_parse_ex"));
        assert!(bridge.contains("json_tokener_get_parse_end"));
        assert!(!bridge.contains("strstr(json"));
        assert!(bridge.contains("ATG_COBOL_MAX_HTTP_RESPONSE"));
    }
}
