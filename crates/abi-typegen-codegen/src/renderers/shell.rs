//! Bash bindings backed by Foundry `cast`.
//!
//! Generated files are intended to be sourced from Bash scripts. They do not
//! modify the caller's shell options and keep all RPC/signing behavior in Cast.

use crate::naming::Scope;
use abi_typegen_core::types::{ContractIr, SolType, StateMutability};
use heck::ToSnakeCase;

fn ident(name: &str) -> String {
    let snake = name.to_snake_case();
    let mut out = String::new();
    for c in snake.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    let out = out.trim_matches('_');
    let mut out = if out.is_empty() {
        "item".to_string()
    } else {
        out.to_string()
    };
    if out.as_bytes().first().is_some_and(|b| b.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

fn sh_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn sol_type(ty: &SolType) -> String {
    match ty {
        SolType::Bool => "bool".into(),
        SolType::Address => "address".into(),
        SolType::StringType => "string".into(),
        SolType::Uint(bits) => format!("uint{bits}"),
        SolType::Int(bits) => format!("int{bits}"),
        SolType::Bytes => "bytes".into(),
        SolType::BytesN(size) => format!("bytes{size}"),
        SolType::Array(inner) => format!("{}[]", sol_type(inner)),
        SolType::FixedArray(inner, size) => format!("{}[{size}]", sol_type(inner)),
        SolType::Tuple(parts) => format!(
            "({})",
            parts
                .iter()
                .map(|p| sol_type(&p.ty))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn output_signature(
    function_signature: &str,
    outputs: &[abi_typegen_core::types::AbiParam],
) -> String {
    format!(
        "{function_signature}({})",
        outputs
            .iter()
            .map(|p| sol_type(&p.ty))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn constructor_signature(ir: &ContractIr) -> String {
    let inputs = ir
        .constructor
        .as_ref()
        .map_or(&[][..], |constructor| constructor.inputs.as_slice());
    format!(
        "constructor({})",
        inputs
            .iter()
            .map(|p| sol_type(&p.ty))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn claim_item(scope: &mut Scope, base: &str) -> String {
    scope.claim_family(
        base,
        &[
            "_signature",
            "_call_signature",
            "_selector",
            "_encode",
            "_decode",
            "_call_raw",
            "_call",
            "_send",
            "_filter_signature",
            "_data_signature",
            "_topic",
            "_logs",
            "_decode_data",
            "_decode_error",
        ],
    )
}

fn arity_guard(expected: usize, usage: &str) -> String {
    format!(
        "    if (( $# != {expected} )); then\n        printf '%s\\n' {} >&2\n        return 2\n    fi\n",
        sh_quote(&format!("usage: {usage}"))
    )
}

fn common_rpc_options() -> &'static str {
    r#"    local -a atg_opts=()
    if [[ -n ${ATG_RPC_URL:-} ]]; then atg_opts+=(--rpc-url "$ATG_RPC_URL"); fi
"#
}

fn call_options() -> &'static str {
    r#"    if [[ -n ${ATG_BLOCK:-} ]]; then atg_opts+=(--block "$ATG_BLOCK"); fi
    if [[ -n ${ATG_FROM:-} ]]; then atg_opts+=(--from "$ATG_FROM"); fi
    if declare -p ATG_CAST_CALL_ARGS >/dev/null 2>&1; then atg_opts+=(${ATG_CAST_CALL_ARGS[@]+"${ATG_CAST_CALL_ARGS[@]}"}); fi
"#
}

fn send_options(payable: bool) -> String {
    let mut out = String::from(
        r#"    if [[ -n ${ATG_PRIVATE_KEY:-} ]]; then atg_opts+=(--private-key "$ATG_PRIVATE_KEY"); fi
    if [[ -n ${ATG_FROM:-} ]]; then atg_opts+=(--from "$ATG_FROM"); fi
    if [[ -n ${ATG_GAS_LIMIT:-} ]]; then atg_opts+=(--gas-limit "$ATG_GAS_LIMIT"); fi
"#,
    );
    if payable {
        out.push_str(
            "    if [[ -n ${ATG_VALUE:-} ]]; then atg_opts+=(--value \"$ATG_VALUE\"); fi\n",
        );
    } else {
        out.push_str(
            "    if [[ -n ${ATG_VALUE:-} ]]; then printf '%s\\n' 'nonpayable call cannot receive value' >&2; return 2; fi\n",
        );
        out.push_str(
            "    if declare -p ATG_CAST_SEND_ARGS >/dev/null 2>&1; then\n        local atg_arg\n        for atg_arg in ${ATG_CAST_SEND_ARGS[@]+\"${ATG_CAST_SEND_ARGS[@]}\"}; do\n            case $atg_arg in --value|--value=*) printf '%s\\n' 'nonpayable call cannot receive value' >&2; return 2 ;; esac\n        done\n    fi\n",
        );
    }
    out.push_str(
        r#"    if declare -p ATG_CAST_SEND_ARGS >/dev/null 2>&1; then atg_opts+=(${ATG_CAST_SEND_ARGS[@]+"${ATG_CAST_SEND_ARGS[@]}"}); fi
"#,
    );
    out
}

fn logs_options() -> &'static str {
    r#"    if [[ -n ${ATG_FROM_BLOCK:-} ]]; then atg_opts+=(--from-block "$ATG_FROM_BLOCK"); fi
    if [[ -n ${ATG_TO_BLOCK:-} ]]; then atg_opts+=(--to-block "$ATG_TO_BLOCK"); fi
    if declare -p ATG_CAST_LOG_ARGS >/dev/null 2>&1; then atg_opts+=(${ATG_CAST_LOG_ARGS[@]+"${ATG_CAST_LOG_ARGS[@]}"}); fi
"#
}

/// Return the Bash namespace prefix for a contract.
pub fn namespace_name(name: &str) -> String {
    format!("atg_{}", ident(name))
}

struct ShellNames {
    prefix: String,
    upper: String,
    functions: Vec<String>,
    errors: Vec<String>,
    events: Vec<String>,
}

fn allocate_names(ir: &ContractIr) -> ShellNames {
    let prefix = namespace_name(&ir.name);
    let upper = prefix.to_ascii_uppercase();
    let constructor_symbol = format!("{prefix}_constructor_signature");
    let mut scope = Scope::with_reserved([constructor_symbol.as_str()]);
    let functions = ir
        .functions
        .iter()
        .map(|function| claim_item(&mut scope, &format!("{prefix}_{}", ident(&function.name))))
        .collect();
    let errors = ir
        .errors
        .iter()
        .map(|error| {
            claim_item(
                &mut scope,
                &format!("{prefix}_{}_error", ident(&error.name)),
            )
        })
        .collect();
    let events = ir
        .events
        .iter()
        .map(|event| {
            claim_item(
                &mut scope,
                &format!("{prefix}_{}_event", ident(&event.name)),
            )
        })
        .collect();
    ShellNames {
        prefix,
        upper,
        functions,
        errors,
        events,
    }
}

/// Returns every Bash variable and function emitted for this contract.
///
/// The shared `ATG_CAST_BIN` setting is intentionally excluded because it is
/// reused by all sourced bindings.
pub fn declared_names(ir: &ContractIr, wrappers: bool) -> Vec<String> {
    let names = allocate_names(ir);
    let mut declared = vec![
        format!("{}_ABI", names.upper),
        format!("{}_CONSTRUCTOR_SIGNATURE", names.upper),
    ];
    if wrappers {
        declared.push(format!("{}_constructor_args", names.prefix));
        declared.push(format!("{}_deploy", names.prefix));
    }
    for (function, item) in ir.functions.iter().zip(&names.functions) {
        let var = item.to_ascii_uppercase();
        declared.extend([
            format!("{var}_SIGNATURE"),
            format!("{var}_CALL_SIGNATURE"),
            format!("{var}_SELECTOR"),
        ]);
        if wrappers {
            declared.extend([format!("{item}_encode"), format!("{item}_decode")]);
            match function.state_mutability {
                StateMutability::View | StateMutability::Pure => {
                    declared.extend([format!("{item}_call_raw"), format!("{item}_call")]);
                }
                StateMutability::NonPayable | StateMutability::Payable => {
                    declared.push(format!("{item}_send"));
                }
            }
        }
    }
    for item in &names.errors {
        let var = item.to_ascii_uppercase();
        declared.extend([format!("{var}_SIGNATURE"), format!("{var}_SELECTOR")]);
        if wrappers {
            declared.push(format!("{item}_decode"));
        }
    }
    for (event, item) in ir.events.iter().zip(&names.events) {
        let var = item.to_ascii_uppercase();
        declared.extend([
            format!("{var}_SIGNATURE"),
            format!("{var}_FILTER_SIGNATURE"),
            format!("{var}_DATA_SIGNATURE"),
        ]);
        if !event.anonymous {
            declared.push(format!("{var}_TOPIC"));
        }
        if wrappers {
            declared.extend([format!("{item}_logs"), format!("{item}_decode_data")]);
        }
    }
    declared
}

/// Render one sourceable Bash library using Foundry `cast` for ABI/RPC operations.
///
/// When `wrappers` is false only metadata constants are emitted.
pub fn render_shell_file(ir: &ContractIr, wrappers: bool) -> String {
    let names = allocate_names(ir);
    let prefix = &names.prefix;
    let upper = &names.upper;
    let abi = serde_json::to_string(&ir.raw_abi).expect("ABI serializes");
    let mut out = format!(
        "# Generated by abi-typegen. Do not edit.\n# Source this file from Bash. Requires Foundry cast for callable bindings.\n# This file intentionally does not change shell options.\n# shellcheck shell=bash\n\n{upper}_ABI={}\n: \"${{ATG_CAST_BIN:=cast}}\"\n\n",
        sh_quote(&abi)
    );

    let constructor_sig = constructor_signature(ir);
    out.push_str(&format!(
        "{upper}_CONSTRUCTOR_SIGNATURE={}\n",
        sh_quote(&constructor_sig)
    ));
    if wrappers {
        let argc = ir.constructor.as_ref().map_or(0, |c| c.inputs.len());
        out.push_str(&format!(
            r#"
{prefix}_constructor_args() {{
{guard}    "$ATG_CAST_BIN" abi-encode -- "${{{upper}_CONSTRUCTOR_SIGNATURE}}" "$@"
}}

{prefix}_deploy() {{
{deploy_guard}    local atg_bytecode=$1
    if [[ ! $atg_bytecode =~ ^0x([[:xdigit:]]{{2}})+$ ]]; then
        printf '%s\n' 'bytecode must be nonempty, even-length 0x-prefixed hex' >&2
        return 2
    fi
    shift
    local atg_encoded atg_initcode
    atg_encoded=$("$ATG_CAST_BIN" abi-encode -- "${{{upper}_CONSTRUCTOR_SIGNATURE}}" "$@") || return
    atg_initcode="${{atg_bytecode}}${{atg_encoded#0x}}"
{rpc}{send}    "$ATG_CAST_BIN" send ${{atg_opts[@]+"${{atg_opts[@]}}"}} --create "$atg_initcode"
}}
"#,
            guard = arity_guard(
                argc,
                &format!("{prefix}_constructor_args <{} constructor args>", argc)
            ),
            deploy_guard = arity_guard(
                argc + 1,
                &format!("{prefix}_deploy <bytecode> <{} constructor args>", argc)
            ),
            rpc = common_rpc_options(),
            send = send_options(
                ir.constructor
                    .as_ref()
                    .is_some_and(|c| c.state_mutability == StateMutability::Payable)
            ),
        ));
    }

    for (function, item) in ir.functions.iter().zip(&names.functions) {
        let var = item.to_ascii_uppercase();
        let signature = function.signature();
        let cast_signature = output_signature(&signature, &function.outputs);
        out.push_str(&format!(
            "\n{var}_SIGNATURE={}\n{var}_CALL_SIGNATURE={}\n{var}_SELECTOR={}\n",
            sh_quote(&signature),
            sh_quote(&cast_signature),
            sh_quote(&function.selector().to_string())
        ));
        if !wrappers {
            continue;
        }
        let argc = function.inputs.len();
        out.push_str(&format!(
            r#"
{item}_encode() {{
{encode_guard}    "$ATG_CAST_BIN" calldata -- "${{{var}_SIGNATURE}}" "$@"
}}
"#,
            encode_guard = arity_guard(argc, &format!("{item}_encode <{} ABI args>", argc)),
        ));

        if function.outputs.is_empty() {
            out.push_str(&format!(
                r#"
{item}_decode() {{
{decode_guard}    if [[ $1 == 0x || -z $1 ]]; then
        return 0
    fi
    printf 'unexpected non-empty result for %s: %s\n' "${{{var}_SIGNATURE}}" "$1" >&2
    return 1
}}
"#,
                decode_guard = arity_guard(1, &format!("{item}_decode <0x-result>")),
            ));
        } else {
            out.push_str(&format!(
                r#"
{item}_decode() {{
{decode_guard}    "$ATG_CAST_BIN" decode-abi -- "${{{var}_CALL_SIGNATURE}}" "$1"
}}
"#,
                decode_guard = arity_guard(1, &format!("{item}_decode <abi-result>")),
            ));
        }

        match function.state_mutability {
            StateMutability::View | StateMutability::Pure => {
                out.push_str(&format!(
                    r#"
{item}_call_raw() {{
{guard}    local atg_address=$1
    shift
{rpc}{call}    "$ATG_CAST_BIN" call ${{atg_opts[@]+"${{atg_opts[@]}}"}} -- "$atg_address" "${{{var}_SIGNATURE}}" "$@"
}}

{item}_call() {{
{guard2}    local atg_address=$1
    shift
{rpc2}{call2}    "$ATG_CAST_BIN" call ${{atg_opts[@]+"${{atg_opts[@]}}"}} -- "$atg_address" "${{{var}_CALL_SIGNATURE}}" "$@"
}}
"#,
                    guard = arity_guard(argc + 1, &format!("{item}_call_raw <contract> <{} ABI args>", argc)),
                    guard2 = arity_guard(argc + 1, &format!("{item}_call <contract> <{} ABI args>", argc)),
                    rpc = common_rpc_options(),
                    call = call_options(),
                    rpc2 = common_rpc_options(),
                    call2 = call_options(),
                ));
            }
            StateMutability::NonPayable | StateMutability::Payable => {
                out.push_str(&format!(
                    r#"
{item}_send() {{
{guard}    local atg_address=$1
    shift
{rpc}{send}    "$ATG_CAST_BIN" send ${{atg_opts[@]+"${{atg_opts[@]}}"}} -- "$atg_address" "${{{var}_SIGNATURE}}" "$@"
}}
"#,
                    guard = arity_guard(argc + 1, &format!("{item}_send <contract> <{} ABI args>", argc)),
                    rpc = common_rpc_options(),
                    send = send_options(function.state_mutability == StateMutability::Payable),
                ));
            }
        }
    }

    for (error, item) in ir.errors.iter().zip(&names.errors) {
        let var = item.to_ascii_uppercase();
        out.push_str(&format!(
            "\n{var}_SIGNATURE={}\n{var}_SELECTOR={}\n",
            sh_quote(&error.signature()),
            sh_quote(&error.selector().to_string())
        ));
        if wrappers {
            out.push_str(&format!(
                r#"
{item}_decode() {{
{guard}    "$ATG_CAST_BIN" decode-error --sig "${{{var}_SIGNATURE}}" -- "$1"
}}
"#,
                guard = arity_guard(1, &format!("{item}_decode <revert-data>")),
            ));
        }
    }

    for (event, item) in ir.events.iter().zip(&names.events) {
        let var = item.to_ascii_uppercase();
        let filter_signature = format!(
            "{}({})",
            event.name,
            event
                .inputs
                .iter()
                .map(|p| {
                    let mut part = sol_type(&p.ty);
                    if p.indexed {
                        part.push_str(" indexed");
                    }
                    if !p.name.is_empty() {
                        part.push(' ');
                        part.push_str(&p.name);
                    }
                    part
                })
                .collect::<Vec<_>>()
                .join(",")
        );
        let data_signature = format!(
            "{}({})",
            event.name,
            event
                .inputs
                .iter()
                .filter(|param| !param.indexed)
                .map(|param| sol_type(&param.ty))
                .collect::<Vec<_>>()
                .join(",")
        );
        out.push_str(&format!(
            "\n{var}_SIGNATURE={}\n{var}_FILTER_SIGNATURE={}\n{var}_DATA_SIGNATURE={}\n",
            sh_quote(&event.signature()),
            sh_quote(&filter_signature),
            sh_quote(&data_signature)
        ));
        if !event.anonymous {
            out.push_str(&format!(
                "{var}_TOPIC={}\n",
                sh_quote(&event.topic0().to_string())
            ));
        }
        if !wrappers {
            continue;
        }
        if event.anonymous {
            out.push_str(&format!(
                r#"
{item}_logs() {{
    printf '%s\n' 'anonymous events cannot be filtered by signature; use cast logs with explicit topics' >&2
    return 2
}}
"#
            ));
        } else {
            out.push_str(&format!(
                r#"
{item}_logs() {{
    if (( $# < 1 )); then
        printf '%s\n' {} >&2
        return 2
    fi
    local atg_address=$1
    shift
{rpc}{logs}    "$ATG_CAST_BIN" logs ${{atg_opts[@]+"${{atg_opts[@]}}"}} --address "$atg_address" -- "${{{var}_FILTER_SIGNATURE}}" "$@"
}}
"#,
                sh_quote(&format!("usage: {item}_logs <contract> [indexed-filter ...]")),
                rpc = common_rpc_options(),
                logs = logs_options(),
            ));
        }
        out.push_str(&format!(
            r#"
# Decodes the event data payload only. Indexed fields live in topics and are not
# reconstructed by cast decode-event.
{item}_decode_data() {{
{guard}    "$ATG_CAST_BIN" decode-event --sig "${{{var}_DATA_SIGNATURE}}" -- "$1"
}}
"#,
            guard = arity_guard(1, &format!("{item}_decode_data <event-data>")),
        ));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use abi_typegen_core::parser::parse_artifact;

    #[test]
    fn renders_cast_wrappers_and_metadata_mode() {
        let ir = parse_artifact(
            "Token",
            r#"{"abi":[
                {"type":"constructor","inputs":[{"name":"name","type":"string"}],"stateMutability":"nonpayable"},
                {"type":"function","name":"balanceOf","inputs":[{"name":"owner","type":"address"}],"outputs":[{"name":"amount","type":"uint256"}],"stateMutability":"view"},
                {"type":"function","name":"mint","inputs":[{"name":"to","type":"address"},{"name":"amount","type":"uint256"}],"outputs":[],"stateMutability":"nonpayable"},
                {"type":"event","name":"Transfer","inputs":[{"name":"from","type":"address","indexed":true},{"name":"to","type":"address","indexed":true},{"name":"amount","type":"uint256","indexed":false}],"anonymous":false},
                {"type":"error","name":"Denied","inputs":[{"name":"owner","type":"address"}]}
            ]}"#,
        )
        .expect("fixture");

        let full = render_shell_file(&ir, true);
        assert!(full.contains("atg_token_balance_of_encode()"), "{full}");
        assert!(full.contains("atg_token_balance_of_call_raw()"), "{full}");
        assert!(full.contains("atg_token_balance_of_call()"), "{full}");
        assert!(full.contains("atg_token_mint_send()"), "{full}");
        assert!(full.contains("atg_token_transfer_event_logs()"), "{full}");
        assert!(full.contains("decode-error --sig"), "{full}");
        assert!(full.contains("balanceOf(address)(uint256)"), "{full}");

        let plain = render_shell_file(&ir, false);
        assert!(plain.contains("ATG_TOKEN_ABI="));
        assert!(plain.contains("ATG_TOKEN_BALANCE_OF_SIGNATURE="));
        assert!(!plain.contains("cast calldata"));
    }

    #[test]
    fn declared_names_expose_cross_contract_symbol_collisions() {
        let foo = parse_artifact(
            "Foo",
            r#"{"abi":[{"type":"function","name":"barBaz","inputs":[],"outputs":[],"stateMutability":"view"}]}"#,
        )
        .expect("foo fixture");
        let foo_bar = parse_artifact(
            "FooBar",
            r#"{"abi":[{"type":"function","name":"baz","inputs":[],"outputs":[],"stateMutability":"view"}]}"#,
        )
        .expect("foo bar fixture");
        assert_eq!(namespace_name("Foo"), "atg_foo");
        assert_eq!(namespace_name("FooBar"), "atg_foo_bar");
        for wrappers in [false, true] {
            let first = declared_names(&foo, wrappers);
            let second = declared_names(&foo_bar, wrappers);
            assert!(first.contains(&"ATG_FOO_BAR_BAZ_SIGNATURE".into()));
            assert!(second.contains(&"ATG_FOO_BAR_BAZ_SIGNATURE".into()));
            if wrappers {
                assert!(first.contains(&"atg_foo_bar_baz_encode".into()));
                assert!(second.contains(&"atg_foo_bar_baz_encode".into()));
            }
        }
    }
}
