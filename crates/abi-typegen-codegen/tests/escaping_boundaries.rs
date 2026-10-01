use abi_typegen_codegen::{elixir, kotlin};
use abi_typegen_core::{parser::parse_artifact, types::ContractIr};
use serde_json::{Value, json};

fn parse(name: &str, abi: Value, metadata: Value) -> ContractIr {
    parse_artifact(name, &json!({"abi": abi, "metadata": metadata}).to_string())
        .expect("valid artifact")
}

#[test]
fn kotlin_natspec_does_not_open_nested_block_comments() {
    let ir = parse(
        "CommentCases",
        json!([{"type":"function","name":"read","inputs":[{"name":"value","type":"bool"}],"outputs":[],"stateMutability":"view"}]),
        json!({"output":{
            "userdoc":{"notice":"Contract /* opener", "methods":{"read(bool)":{"notice":"Function /* opener"}}},
            "devdoc":{"details":"Close */ safely", "methods":{"read(bool)":{"details":"\n\nDetails */ safely"}}}
        }}),
    );
    let source = kotlin::render_kotlin_file(&ir, "contracts");
    assert!(source.contains("Contract /&#42; opener"), "{source}");
    assert!(source.contains("Function /&#42; opener"), "{source}");
    assert!(source.contains("Close *&#47; safely"), "{source}");
    assert!(source.contains("Details *&#47; safely"), "{source}");
}

#[test]
fn kotlin_all_underscore_contract_has_a_usable_namespace() {
    for name in ["_", "___"] {
        let ir = parse(name, json!([]), Value::Null);
        let source = kotlin::render_kotlin_file(&ir, "contracts");
        assert!(source.contains("object Contract {"), "{source}");
        assert!(source.contains("val JSON: String"), "{source}");
    }
}

#[test]
fn kotlin_abi_metadata_escapes_c1_controls_and_dollar_templates() {
    let ir = parse(
        "LiteralCases",
        json!([{"type":"function","name":"read","inputs":[],"outputs":[],"stateMutability":"view","description":"price $5\u{85}\u{9f}😀"}]),
        Value::Null,
    );
    let source = kotlin::render_kotlin_file(&ir, "contracts");
    assert!(source.contains("price \\$5\\u0085\\u009f😀"), "{source}");
    assert!(!source.contains('\u{85}'));
    assert!(!source.contains('\u{9f}'));
}

#[test]
fn elixir_dollar_function_aliases_are_quoted_atoms() {
    let ir = parse(
        "DollarAliases",
        json!([
            {"type":"function","name":"foo$Bar","inputs":[{"name":"value","type":"bool"}],"outputs":[],"stateMutability":"view"},
            {"type":"function","name":"foo$_bar","inputs":[{"name":"value","type":"uint256"}],"outputs":[],"stateMutability":"view"}
        ]),
        Value::Null,
    );
    assert!(elixir::unsupported_sdk_collisions(&ir).is_empty());
    let source = elixir::render_elixir_file(&ir, true);
    assert!(
        source.contains("def unquote(:\"foo$_bar\")(arg0)"),
        "{source}"
    );
    assert!(
        source.contains("def unquote(:\"foo$_bar_2\")(arg0)"),
        "{source}"
    );
    assert!(
        source.contains(
            "apply(Elixir.DollarAliases.FunctionAliases.Binding1, :\"foo$_bar\", [arg0])"
        ),
        "{source}"
    );
}

#[test]
fn elixir_reserved_function_aliases_use_atom_names() {
    let ir = parse(
        "ReservedAliases",
        json!([
            {"type":"function","name":"when","inputs":[{"name":"value","type":"bool"}],"outputs":[],"stateMutability":"view"},
            {"type":"function","name":"When","inputs":[{"name":"value","type":"uint256"}],"outputs":[],"stateMutability":"view"}
        ]),
        Value::Null,
    );
    let source = elixir::render_elixir_file(&ir, true);
    assert!(source.contains("def unquote(:when)(arg0)"), "{source}");
    assert!(source.contains("def unquote(:when_2)(arg0)"), "{source}");
}

#[test]
fn elixir_unquote_methods_are_rejected_before_sdk_expansion() {
    let ir = parse(
        "ReservedAliases",
        json!([
            {"type":"function","name":"unquote","inputs":[{"name":"value","type":"bool"}],"outputs":[],"stateMutability":"view"},
            {"type":"function","name":"Unquote","inputs":[{"name":"value","type":"uint256"}],"outputs":[],"stateMutability":"view"},
            {"type":"function","name":"unquoteSplicing","inputs":[{"name":"value","type":"bool"}],"outputs":[],"stateMutability":"view"},
            {"type":"function","name":"UnquoteSplicing","inputs":[{"name":"value","type":"uint256"}],"outputs":[],"stateMutability":"view"}
        ]),
        Value::Null,
    );
    let problems = elixir::unsupported_sdk_collisions(&ir);
    assert_eq!(problems.len(), 4, "{problems:?}");
    assert!(
        problems
            .iter()
            .all(|problem| problem.contains("SDK typespec collision"))
    );
    let source = elixir::render_elixir_file(&ir, false);
    assert!(source.contains("def abi_json(), do: @abi_json"));
    assert!(source.contains("unquoteSplicing"));
    assert!(!source.contains("Ethers.Contract"));
}

#[test]
fn elixir_unquote_event_filters_are_rejected_before_sdk_expansion() {
    let ir = parse(
        "MetaprogrammingEvents",
        json!([
            {"type":"event","name":"Unquote","inputs":[{"name":"value","type":"bool","indexed":true}],"anonymous":false},
            {"type":"event","name":"UnquoteSplicing","inputs":[{"name":"value","type":"uint256","indexed":true}],"anonymous":false}
        ]),
        Value::Null,
    );
    let problems = elixir::unsupported_sdk_collisions(&ir);
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(
        problems
            .iter()
            .all(|problem| problem.contains("SDK typespec collision"))
    );
}

#[test]
fn elixir_contract_docs_escape_controls_and_interpolation() {
    let name = "Control\u{0}\u{1}\u{08}\u{0c}\u{7f}\u{85}\n\r\t\\\"#{raise \"bad\"}";
    let ir = parse(name, json!([]), Value::Null);
    let source = elixir::render_elixir_file(&ir, false);
    assert!(
        source.contains("\\u{0}\\u{1}\\b\\f\\u{7f}\\u{85}\\n\\r\\t\\\\\\\"\\#{raise "),
        "{source}"
    );
    assert!(source.contains("def abi_json(), do: @abi_json"));
}
