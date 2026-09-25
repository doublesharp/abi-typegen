//! Elixir bindings backed by Ethers.Contract.
//!
//! Ethers parses the ABI at compile time and generates contract functions,
//! constructors, event filters, custom-error helpers, docs, and typespecs.
//! Generated functions return `Ethers.TxData`; RPC execution remains explicit
//! through `Ethers.call/2` and `Ethers.send_transaction/2`.

use abi_typegen_core::types::ContractIr;
use heck::ToSnakeCase;
use std::collections::{BTreeSet, HashMap, HashSet};

/// Top-level Elixir modules we should not replace with generated contract modules.
const RESERVED_MODULES: &[&str] = &[
    "Access",
    "Agent",
    "Application",
    "Atom",
    "Code",
    "Date",
    "DateTime",
    "DynamicSupervisor",
    "Elixir",
    "Enum",
    "Ethers",
    "Exception",
    "File",
    "Float",
    "Function",
    "GenServer",
    "IO",
    "Integer",
    "Kernel",
    "List",
    "Logger",
    "Macro",
    "Map",
    "MapSet",
    "Mix",
    "Module",
    "NaiveDateTime",
    "Node",
    "Path",
    "Port",
    "Process",
    "Protocol",
    "Range",
    "Reference",
    "Registry",
    "Regex",
    "Stream",
    "String",
    "Supervisor",
    "System",
    "Task",
    "Time",
    "Tuple",
    "URI",
];

/// Returns the generated Elixir module name for a contract.
///
/// Solidity contract names that are already valid Elixir aliases are preserved,
/// including acronyms such as `ERC20`. Invalid/unsafe names are normalized and
/// core Elixir/Ethers module collisions receive a `Contract` suffix.
pub fn namespace_name(name: &str) -> String {
    let mut out = String::new();
    let mut capitalize = true;

    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            if out.is_empty() {
                if c.is_ascii_uppercase() {
                    out.push(c);
                } else if c.is_ascii_lowercase() {
                    out.push(c.to_ascii_uppercase());
                } else {
                    out.push_str("Contract");
                    out.push(c);
                }
            } else if capitalize && c.is_ascii_lowercase() {
                out.push(c.to_ascii_uppercase());
            } else {
                out.push(c);
            }
            capitalize = false;
        } else {
            capitalize = true;
        }
    }

    if out.is_empty() {
        out.push_str("Contract");
    }
    if RESERVED_MODULES.contains(&out.as_str()) {
        out.push_str("Contract");
    }
    out
}

/// Returns the conventional file name for a generated contract module.
pub fn file_name(contract_name: &str) -> String {
    format!("{}.ex", namespace_name(contract_name).to_snake_case())
}

/// Returns all Elixir modules emitted directly or by `Ethers.Contract` for this binding.
///
/// Use these names for cross-contract collision checks before writing output files.
pub fn declared_module_names(ir: &ContractIr, wrappers: bool) -> Vec<String> {
    let module = namespace_name(&ir.name);
    if wrappers {
        let mut names = vec![
            module.clone(),
            format!("{module}.EventFilters"),
            format!("{module}.Errors"),
        ];
        names.extend(
            ir.errors
                .iter()
                .map(|error| format!("{module}.Errors.{}", error.name)),
        );
        for (index, _) in function_aliases(ir).iter().enumerate() {
            let helper = format!("{module}.FunctionAliases.Binding{}", index + 1);
            names.extend([
                helper.clone(),
                format!("{helper}.EventFilters"),
                format!("{helper}.Errors"),
            ]);
        }
        names
    } else {
        vec![module]
    }
}

/// Reports ABI entries that Ethers 0.8 cannot generate without shadowing or
/// redefining an event-filter function or custom-error module.
///
/// Reject these before rendering wrappers; Ethers derives callable names with
/// `Macro.underscore/1` for event filters, and its error structs use the raw
/// error name. Function-name collisions are handled by SDK-backed aliases.
pub fn unsupported_sdk_collisions(ir: &ContractIr) -> Vec<String> {
    let mut problems = Vec::new();
    let constructor_arity = ir
        .constructor
        .as_ref()
        .map_or(0, |constructor| constructor.inputs.len());
    for function in &ir.functions {
        let method = ethers_method_name(&function.name);
        let arity = function.inputs.len();
        let reserved = matches!(
            method.as_str(),
            "__default_address__" | "__contract_binary__"
        ) && arity == 0
            || method == "constructor" && arity == constructor_arity;
        if reserved {
            problems.push(format!(
                "Elixir SDK helper name/arity collision: {} conflicts with {method}/{arity}",
                function.signature()
            ));
        }
    }
    let mut events: HashMap<(String, usize), (&str, String)> = HashMap::new();
    let mut indexed_signatures: HashMap<(String, Vec<String>), String> = HashMap::new();
    for event in &ir.events {
        let key = (
            ethers_method_name(&event.name),
            event.inputs.iter().filter(|input| input.indexed).count(),
        );
        if key.1 == 0
            && matches!(
                key.0.as_str(),
                "__default_address__" | "__all__" | "__events__"
            )
        {
            problems.push(format!(
                "Elixir SDK event-filter helper name/arity collision: {} conflicts with {}/0",
                event.signature(),
                key.0
            ));
        }
        let signature = event.signature();
        let indexed_types = event
            .inputs
            .iter()
            .filter(|input| input.indexed)
            .map(|input| input.ty.canonical())
            .collect::<Vec<_>>();
        if let Some(previous) =
            indexed_signatures.insert((event.name.clone(), indexed_types), signature.clone())
        {
            problems.push(format!(
                "Elixir event filter overload ambiguity: {previous} and {signature} have identical indexed types"
            ));
        }
        if let Some((previous_name, previous_signature)) = events.get(&key) {
            if *previous_name != event.name {
                problems.push(format!(
                    "Elixir event filter name/arity collision: {previous_signature} and {signature}"
                ));
            }
        } else {
            events.insert(key, (&event.name, signature));
        }
    }

    let mut errors = HashMap::new();
    for error in &ir.errors {
        if let Some(previous_signature) = errors.insert(error.name.as_str(), error.signature()) {
            problems.push(format!(
                "Elixir error module collision: {previous_signature} and {}",
                error.signature()
            ));
        }
    }
    problems
}

/// Mirrors `Macro.underscore/1` for Solidity's ASCII identifier alphabet.
/// Existing underscores are preserved, including leading, trailing, and
/// repeated underscores; acronym and digit boundaries insert one underscore.
fn ethers_method_name(name: &str) -> String {
    let chars = name.as_bytes();
    let mut normalized = String::with_capacity(name.len() + 4);
    for (index, &current) in chars.iter().enumerate() {
        if current.is_ascii_uppercase() && index > 0 {
            let previous = chars[index - 1];
            let next_is_lowercase = chars.get(index + 1).is_some_and(u8::is_ascii_lowercase);
            if previous.is_ascii_lowercase()
                || previous.is_ascii_digit()
                || previous == b'$'
                || (previous.is_ascii_uppercase() || previous == b'_') && next_is_lowercase
            {
                normalized.push('_');
            }
        }
        normalized.push(current.to_ascii_lowercase() as char);
    }
    normalized
}

/// (ABI name, public alias, distinct arities) for names Ethers would shadow.
fn function_aliases(ir: &ContractIr) -> Vec<(String, String, Vec<usize>)> {
    let mut by_key: HashMap<(String, usize), &str> = HashMap::new();
    let mut conflicting = HashSet::new();
    for function in &ir.functions {
        let key = (ethers_method_name(&function.name), function.inputs.len());
        if key == ("abi_json".to_string(), 0) {
            conflicting.insert(function.name.clone());
        }
        if let Some(previous) = by_key.insert(key, &function.name) {
            if previous != function.name {
                conflicting.insert(previous.to_string());
                conflicting.insert(function.name.clone());
            }
        }
    }
    let mut used = ir
        .functions
        .iter()
        .filter(|function| !conflicting.contains(&function.name))
        .map(|function| ethers_method_name(&function.name))
        .collect::<HashSet<_>>();
    used.insert("abi_json".to_string());
    let mut seen = HashSet::new();
    let mut aliases = Vec::new();
    for function in &ir.functions {
        if !conflicting.contains(&function.name) || !seen.insert(function.name.clone()) {
            continue;
        }
        let base = ethers_method_name(&function.name);
        let mut candidate = base.clone();
        let mut number = 2;
        while used.contains(&candidate) {
            candidate = format!("{base}_{number}");
            number += 1;
        }
        used.insert(candidate.clone());
        let arities = ir
            .functions
            .iter()
            .filter(|item| item.name == function.name)
            .map(|item| item.inputs.len())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        aliases.push((function.name.clone(), candidate, arities));
    }
    aliases
}

/// Renders one standalone Elixir module using Ethers 0.8.x.
///
/// With `wrappers` enabled, `Ethers.Contract` generates transport-independent
/// `Ethers.TxData` builders. With it disabled, the module only exposes ABI JSON
/// and requires no Elixir dependencies.
pub fn render_elixir_file(ir: &ContractIr, wrappers: bool) -> String {
    let module = namespace_name(&ir.name);
    let abi_json = serde_json::to_string(&ir.raw_abi).expect("JSON values serialize");
    let abi = elixir_string(&abi_json);
    let docs = elixir_string(&if wrappers {
        format!(
            "Bindings for the `{}` contract. Generated by abi-typegen using Ethers.Contract.",
            ir.name
        )
    } else {
        format!(
            "ABI metadata for the `{}` contract. Generated by abi-typegen.",
            ir.name
        )
    });

    let mut body = String::from(
        "  @doc \"Returns the full contract ABI as JSON.\"\n  @spec abi_json() :: String.t()\n  def abi_json(), do: @abi_json\n",
    );
    if wrappers {
        let aliases = function_aliases(ir);
        let alias_names = aliases
            .iter()
            .map(|(name, _, _)| name.as_str())
            .collect::<HashSet<_>>();
        let entries = ir.raw_abi.as_array().expect("parsed ABI is an array");
        if aliases.is_empty() {
            body.push_str("  use Ethers.Contract, abi: @abi_json\n");
        } else {
            let macro_entries = entries
                .iter()
                .filter(|entry| {
                    entry.get("type").and_then(serde_json::Value::as_str) != Some("function")
                        || !entry
                            .get("name")
                            .and_then(serde_json::Value::as_str)
                            .is_some_and(|name| alias_names.contains(name))
                })
                .cloned()
                .collect::<Vec<_>>();
            let macro_abi = serde_json::to_string(&macro_entries).expect("ABI values serialize");
            body.push_str(&format!(
                "  @macro_abi_json {}\n  use Ethers.Contract, abi: @macro_abi_json\n",
                elixir_string(&macro_abi)
            ));
            for (index, (name, alias, arities)) in aliases.iter().enumerate() {
                let helper_name = format!("Binding{}", index + 1);
                let group = entries
                    .iter()
                    .filter(|entry| {
                        entry.get("type").and_then(serde_json::Value::as_str) == Some("function")
                            && entry.get("name").and_then(serde_json::Value::as_str)
                                == Some(name.as_str())
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let group_abi = serde_json::to_string(&group).expect("ABI values serialize");
                body.push_str(&format!(
                    "\n  defmodule FunctionAliases.{helper_name} do\n    @moduledoc false\n    use Ethers.Contract, abi: {}\n  end\n",
                    elixir_string(&group_abi)
                ));
                let sdk_method = ethers_method_name(name);
                for arity in arities {
                    let args = (0..*arity)
                        .map(|index| format!("arg{index}"))
                        .collect::<Vec<_>>();
                    let joined = args.join(", ");
                    body.push_str(&format!(
                        "\n  @doc \"Builds calldata for `{name}` through Ethers.Contract.\"\n  def {alias}({joined}), do: Elixir.{module}.FunctionAliases.{helper_name}.{sdk_method}({joined})\n"
                    ));
                }
            }
        }
    }
    format!(
        "# Generated by abi-typegen. Do not edit.\n\ndefmodule {module} do\n  @moduledoc {docs}\n\n  @abi_json {abi}\n{body}end\n"
    )
}

/// Produces an Elixir double-quoted string literal without allowing source
/// interpolation or control characters to escape the generated literal.
fn elixir_string(value: &str) -> String {
    let mut out = String::from("\"");
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            '#' if chars.peek() == Some(&'{') => out.push_str("\\#"),
            c if c.is_control() => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use abi_typegen_core::parser::parse_artifact;

    fn render(name: &str, abi: &str, rpc: bool) -> String {
        let ir = parse_artifact(name, &format!(r#"{{"abi":{abi}}}"#)).expect("valid artifact");
        render_elixir_file(&ir, rpc)
    }

    #[test]
    fn file_and_module_names_preserve_normal_names_and_acronyms() {
        assert_eq!(namespace_name("Token"), "Token");
        assert_eq!(namespace_name("ERC20"), "ERC20");
        assert_eq!(namespace_name("tuple-cases"), "TupleCases");
        assert_eq!(namespace_name("123Token"), "Contract123Token");
        assert_eq!(file_name("TupleCases"), "tuple_cases.ex");
        assert_eq!(file_name("ERC20"), "erc20.ex");
    }

    #[test]
    fn reserved_top_level_modules_get_a_suffix() {
        assert_eq!(namespace_name("String"), "StringContract");
        assert_eq!(namespace_name("Ethers"), "EthersContract");
        assert_eq!(namespace_name("Kernel"), "KernelContract");
    }

    #[test]
    fn emits_ethers_contract_with_embedded_json_abi() {
        let out = render(
            "Token",
            r#"[{"type":"function","name":"balanceOf","inputs":[{"name":"owner","type":"address"}],"outputs":[{"name":"","type":"uint256"}],"stateMutability":"view"}]"#,
            true,
        );
        assert!(out.contains("defmodule Token do"), "{out}");
        assert!(out.contains("use Ethers.Contract, abi: @abi_json"), "{out}");
        assert!(out.contains("\\\"name\\\":\\\"balanceOf\\\""), "{out}");
        assert!(
            !out.contains("Ethers.call("),
            "RPC execution should stay outside the generated module: {out}"
        );
    }

    #[test]
    fn disabling_wrappers_keeps_abi_without_ethers_dependency() {
        let abi =
            r#"[{"type":"function","name":"f","inputs":[],"outputs":[],"stateMutability":"view"}]"#;
        let metadata = render("Token", abi, false);
        assert!(
            metadata.contains("def abi_json(), do: @abi_json"),
            "{metadata}"
        );
        assert!(metadata.contains("\\\"name\\\":\\\"f\\\""), "{metadata}");
        assert!(!metadata.contains("Ethers.Contract"), "{metadata}");
        assert!(!metadata.contains("use Ethers"), "{metadata}");
    }

    #[test]
    fn declares_macro_generated_modules_only_with_wrappers() {
        let ir = parse_artifact("Token", r#"{"abi":[]}"#).expect("valid artifact");
        assert_eq!(
            declared_module_names(&ir, true),
            ["Token", "Token.EventFilters", "Token.Errors"]
        );
        assert_eq!(declared_module_names(&ir, false), ["Token"]);
    }

    #[test]
    fn declares_custom_error_struct_modules() {
        let ir = parse_artifact(
            "Vault",
            r#"{"abi":[{"type":"error","name":"Denied","inputs":[{"name":"code","type":"uint256"}]}]}"#,
        )
        .expect("valid artifact");
        assert_eq!(
            declared_module_names(&ir, true),
            [
                "Vault",
                "Vault.EventFilters",
                "Vault.Errors",
                "Vault.Errors.Denied"
            ]
        );
    }

    #[test]
    fn isolates_sdk_macro_function_name_shadowing() {
        let ir = parse_artifact(
            "NamingCases",
            r#"{"abi":[
              {"type":"function","name":"fooBar","inputs":[{"name":"value","type":"bool"}],"outputs":[],"stateMutability":"view"},
              {"type":"function","name":"foo_bar","inputs":[{"name":"value","type":"uint256"}],"outputs":[],"stateMutability":"view"}
            ]}"#,
        )
        .expect("valid artifact");
        assert!(unsupported_sdk_collisions(&ir).is_empty());
        let out = render_elixir_file(&ir, true);
        assert!(out.contains("def foo_bar(arg0)"), "{out}");
        assert!(out.contains("def foo_bar_2(arg0)"), "{out}");
        assert!(out.contains("defmodule FunctionAliases.Binding1"), "{out}");
        assert!(out.contains("defmodule FunctionAliases.Binding2"), "{out}");
        assert!(
            declared_module_names(&ir, true)
                .contains(&"NamingCases.FunctionAliases.Binding1".into())
        );
    }

    #[test]
    fn abi_json_function_does_not_replace_metadata_accessor() {
        let ir = parse_artifact(
            "Token",
            r#"{"abi":[{"type":"function","name":"abiJson","inputs":[],"outputs":[],"stateMutability":"view"}]}"#,
        )
        .expect("valid artifact");
        let out = render_elixir_file(&ir, true);
        assert!(out.contains("def abi_json(), do: @abi_json"), "{out}");
        assert!(out.contains("def abi_json_2()"), "{out}");
    }

    #[test]
    fn rejects_redefined_error_structs() {
        let ir = parse_artifact(
            "Vault",
            r#"{"abi":[
              {"type":"error","name":"Denied","inputs":[{"name":"code","type":"uint256"}]},
              {"type":"error","name":"Denied","inputs":[{"name":"owner","type":"address"}]}
            ]}"#,
        )
        .expect("valid artifact");
        assert_eq!(
            unsupported_sdk_collisions(&ir),
            ["Elixir error module collision: Denied(uint256) and Denied(address)"]
        );
    }

    #[test]
    fn reports_shadowed_event_filters_for_cli_preflight() {
        let ir = parse_artifact(
            "Events",
            r#"{"abi":[
              {"type":"event","name":"FooBar","inputs":[{"name":"x","type":"uint256","indexed":true}],"anonymous":false},
              {"type":"event","name":"foo_bar","inputs":[{"name":"x","type":"address","indexed":true}],"anonymous":false}
            ]}"#,
        )
        .expect("valid artifact");
        assert_eq!(
            unsupported_sdk_collisions(&ir),
            ["Elixir event filter name/arity collision: FooBar(uint256) and foo_bar(address)"]
        );
    }

    #[test]
    fn reports_sdk_reserved_helper_methods() {
        let ir = parse_artifact(
            "Helpers",
            r#"{"abi":[
              {"type":"function","name":"__default_address__","inputs":[],"outputs":[],"stateMutability":"view"},
              {"type":"function","name":"__contractBinary__","inputs":[],"outputs":[],"stateMutability":"view"},
              {"type":"function","name":"constructor","inputs":[],"outputs":[],"stateMutability":"view"}
            ]}"#,
        )
        .expect("valid artifact");
        let problems = unsupported_sdk_collisions(&ir);
        assert_eq!(problems.len(), 3, "{problems:?}");
        assert!(
            problems
                .iter()
                .all(|problem| problem.contains("SDK helper name/arity collision"))
        );
    }

    #[test]
    fn edge_underscores_follow_sdk_macro_names() {
        assert_eq!(
            ethers_method_name("__defaultAddress__"),
            "__default_address__"
        );
        assert_eq!(ethers_method_name("__foo__"), "__foo__");
        assert_eq!(ethers_method_name("fooBar"), "foo_bar");
        assert_eq!(ethers_method_name("_foo"), "_foo");
        assert_eq!(ethers_method_name("foo_"), "foo_");
        assert_eq!(ethers_method_name("foo__bar"), "foo__bar");
        assert_eq!(ethers_method_name("ERC20Transfer"), "erc20_transfer");
        assert_eq!(ethers_method_name("foo1Bar"), "foo1_bar");
        assert_eq!(ethers_method_name("FOOBar"), "foo_bar");
        assert_eq!(ethers_method_name("fooBAR"), "foo_bar");
        assert_eq!(ethers_method_name("FOO_Bar"), "foo__bar");
        assert_eq!(ethers_method_name("foo__Bar"), "foo___bar");
        assert_eq!(ethers_method_name("_Foo"), "__foo");
        assert_eq!(ethers_method_name("_FOO"), "_foo");
        assert_eq!(ethers_method_name("foo_ABc"), "foo_a_bc");
        assert_eq!(ethers_method_name("foo$Bar"), "foo$_bar");
        assert_eq!(ethers_method_name("$Foo"), "$_foo");
        assert_eq!(ethers_method_name("foo$B"), "foo$_b");
        let ir = parse_artifact(
            "Names",
            r#"{"abi":[
              {"type":"event","name":"foo","inputs":[{"name":"x","type":"uint256","indexed":true}],"anonymous":false},
              {"type":"event","name":"__foo__","inputs":[{"name":"x","type":"uint256","indexed":true}],"anonymous":false}
            ]}"#,
        )
        .expect("valid artifact");
        assert!(unsupported_sdk_collisions(&ir).is_empty());
    }

    #[test]
    fn reports_sdk_reserved_event_filter_helpers() {
        let ir = parse_artifact(
            "Events",
            r#"{"abi":[
              {"type":"event","name":"__default_address__","inputs":[],"anonymous":false},
              {"type":"event","name":"__all__","inputs":[],"anonymous":false},
              {"type":"event","name":"__events__","inputs":[],"anonymous":false}
            ]}"#,
        )
        .expect("valid artifact");
        let problems = unsupported_sdk_collisions(&ir);
        assert_eq!(problems.len(), 3, "{problems:?}");
        assert!(
            problems
                .iter()
                .all(|problem| problem.contains("event-filter helper name/arity collision"))
        );
    }

    #[test]
    fn rejects_event_overloads_with_identical_indexed_types() {
        let ir = parse_artifact(
            "Events",
            r#"{"abi":[
              {"type":"event","name":"Changed","inputs":[{"name":"owner","type":"address","indexed":true},{"name":"amount","type":"uint256","indexed":false}],"anonymous":false},
              {"type":"event","name":"Changed","inputs":[{"name":"owner","type":"address","indexed":true},{"name":"label","type":"string","indexed":false}],"anonymous":false}
            ]}"#,
        )
        .expect("valid artifact");
        assert_eq!(
            unsupported_sdk_collisions(&ir),
            [
                "Elixir event filter overload ambiguity: Changed(address,uint256) and Changed(address,string) have identical indexed types"
            ]
        );
    }

    #[test]
    fn overloads_and_complex_types_are_preserved_in_the_abi() {
        let out = render(
            "Cases",
            r#"[
              {"type":"function","name":"f","inputs":[{"name":"x","type":"uint256"}],"outputs":[],"stateMutability":"view"},
              {"type":"function","name":"f","inputs":[{"name":"x","type":"int256"}],"outputs":[],"stateMutability":"view"},
              {"type":"function","name":"g","inputs":[{"name":"x","type":"tuple[]","components":[{"name":"owner","type":"address"},{"name":"values","type":"uint256[]"}]}],"outputs":[{"name":"","type":"bytes32[64]"}],"stateMutability":"pure"}
            ]"#,
            true,
        );
        assert_eq!(out.matches("\\\"name\\\":\\\"f\\\"").count(), 2, "{out}");
        assert!(out.contains("tuple[]"), "{out}");
        assert!(out.contains("bytes32[64]"), "{out}");
    }

    #[test]
    fn embedded_abi_cannot_start_elixir_interpolation() {
        let out = render(
            "Token",
            r##"[{"type":"function","name":"f","inputs":[{"name":"#{System.cmd(\"sh\",[])}","type":"uint256"}],"outputs":[],"stateMutability":"view"}]"##,
            true,
        );
        assert!(out.contains("\\#{System.cmd"), "{out}");
        assert!(!out.contains("\"#{System.cmd"), "{out}");
    }

    #[test]
    fn empty_abi_is_valid_input() {
        let out = render("Empty", "[]", true);
        assert!(out.contains("@abi_json \"[]\""), "{out}");
        assert!(out.contains("use Ethers.Contract"), "{out}");
    }

    #[test]
    fn core_module_names_cannot_be_overwritten() {
        for name in [
            "Access",
            "Code",
            "Exception",
            "Macro",
            "Module",
            "Protocol",
            "Registry",
        ] {
            assert_eq!(namespace_name(name), format!("{name}Contract"));
        }
    }
}
