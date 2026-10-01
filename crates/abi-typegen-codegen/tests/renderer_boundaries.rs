use abi_typegen_codegen::{dart, elixir, rust, solidity, zod};
use abi_typegen_core::{
    parser::parse_artifact,
    types::{ContractIr, NatSpec},
};
use serde_json::{Value, json};

fn parse(entries: Value) -> ContractIr {
    parse_artifact("Sample", &json!({"abi":entries}).to_string()).expect("ABI")
}

fn function(name: &str, inputs: Value, outputs: Value) -> Value {
    json!({"type":"function", "name":name, "inputs":inputs, "outputs":outputs, "stateMutability":"view"})
}

#[test]
fn solidity_anonymous_events_and_empty_interfaces_have_complete_declarations() {
    let ir = parse(
        json!([{"type":"event","name":"Hidden","anonymous":true,"inputs":[{"name":"","type":"uint256","indexed":true}]}]),
    );
    let source = solidity::render_solidity_file(&ir);
    assert!(source.contains("event Hidden(uint256 indexed) anonymous;"));
    for (name, expected) in [
        ("I", "II"),
        ("iToken", "IiToken"),
        ("IToken", "IToken"),
        ("Sample", "ISample"),
    ] {
        let ir = parse_artifact(name, r#"{"abi":[]}"#).expect("empty ABI");
        let source = solidity::render_solidity_file(&ir);
        assert!(
            source.contains(&format!("interface {expected} {{")),
            "{name}"
        );
        assert!(!source.contains("function "));
    }
}

#[test]
fn solidity_same_short_struct_names_keep_different_shapes_and_reuse_equal_shapes() {
    let first = json!({"name":"a","type":"tuple","internalType":"struct A.Item","components":[{"name":"","type":"uint256"}]});
    let second = json!({"name":"b","type":"tuple","internalType":"struct B.Item","components":[{"name":"flag","type":"bool"}]});
    let ir = parse(json!([function(
        "read",
        json!([first.clone(), second]),
        json!([first])
    )]));
    let source = solidity::render_solidity_file(&ir);
    assert_eq!(source.matches("struct Item {").count(), 1);
    assert!(source.contains("struct Item_2 {"));
    assert!(source.contains("uint256 arg0;"));
    assert!(source.contains("Item memory"));
    assert!(source.contains("Item_2 calldata"));
}

#[test]
fn solidity_documentation_omits_unnamed_and_missing_parameter_tags() {
    let mut ir = parse(json!([function(
        "read",
        json!([{"name":"","type":"uint256"},{"name":"x","type":"uint256"},{"name":"missing","type":"bool"}]),
        json!([{"name":"y","type":"bool"},{"name":"missing","type":"uint256"}])
    )]));
    ir.functions[0].natspec = Some(NatSpec {
        dev: Some("Detailed docs".into()),
        params: std::collections::HashMap::from([("x".into(), "Input docs".into())]),
        returns: std::collections::HashMap::from([("y".into(), "Output docs".into())]),
        ..NatSpec::default()
    });
    let source = solidity::render_solidity_file(&ir);
    assert!(source.contains("/// @dev Detailed docs"));
    assert!(source.contains("/// @param x Input docs"));
    assert!(source.contains("/// @return y Output docs"));
    assert!(!source.contains("@param missing"));
    ir.functions[0].natspec = Some(NatSpec::default());
    assert!(!solidity::render_solidity_file(&ir).contains("/// @"));
}

#[test]
fn rust_docs_preserve_urls_code_spans_and_unnamed_return_descriptions() {
    let mut ir = parse(json!([function(
        "read2",
        json!([{"name":"","type":"tuple","components":[{"name":"","type":"bool"}]}]),
        json!([{"name":"","type":"uint256"},{"name":"named","type":"bool"}])
    )]));
    ir.functions[0].natspec = Some(NatSpec {
        notice: Some(
            "See https://example.com/path, http://example.org. `code` ```nested```` dangling `"
                .into(),
        ),
        dev: Some("\n  ".into()),
        returns: std::collections::HashMap::from([
            ("_0".into(), "Unnamed output".into()),
            ("named".into(), "Named output".into()),
        ]),
        ..NatSpec::default()
    });
    let source = rust::render_rust_file(&ir, false);
    assert!(source.contains("<https://example.com/path>,"));
    assert!(source.contains("<http://example.org>."));
    assert!(source.contains("- Unnamed output"));
    assert!(source.contains("- `named`: Named output"));
    assert!(source.contains("bool _0"));
    ir.functions[0].natspec = Some(NatSpec::default());
    assert!(!rust::render_rust_file(&ir, false).contains("# Returns"));
}

#[test]
fn zod_skips_empty_constructor_events_and_errors_without_dropping_primary_schemas() {
    let ir = parse(json!([
        {"type":"constructor","inputs":[],"stateMutability":"nonpayable"},
        {"type":"event","name":"Empty","anonymous":true,"inputs":[]},
        {"type":"error","name":"Denied","inputs":[]},
        function("read",json!([{"name":"small","type":"int8"},{"name":"large","type":"int256"}]),json!([{"name":"out","type":"bool"}]))
    ]));
    let source = zod::render_zod_file(&ir);
    assert!(!source.contains("ConstructorParams"));
    assert!(!source.contains("EmptyEventSchema"));
    assert!(!source.contains("DeniedErrorSchema"));
    assert!(source.contains("SampleReadParamsSchema"));
    assert!(source.contains("z.number().int()"));
    assert!(source.contains("z.bigint()"));
}

#[test]
fn dart_normalized_event_error_and_field_names_remain_unique() {
    let ir = parse(json!([
        {"type":"event","name":"fooBar","anonymous":false,"inputs":[]},
        {"type":"event","name":"foo_bar","anonymous":false,"inputs":[]},
        {"type":"event","name":"foo_bar","anonymous":true,"inputs":[{"name":"fooBar","type":"bool","indexed":false},{"name":"foo_bar","type":"uint256","indexed":false}]},
        {"type":"error","name":"Denied","inputs":[]},
        {"type":"error","name":"Denied","inputs":[]},
        function("set",json!([{"name":"$","type":"bool"},{"name":"_9","type":"uint256"}]),json!([]))
    ]));
    let source = dart::render_dart_file(&ir, true);
    assert!(source.contains("FooBarByEvent0"));
    assert!(source.contains("FooBarByEvent1"));
    assert!(source.contains("DeniedByError0"));
    assert!(source.contains("DeniedByError1"));
    assert!(source.contains("fooBar2"));
    assert!(source.contains("value0"));
    assert!(source.contains("value1"));
}

#[test]
fn elixir_aliases_preserve_nonfunction_abi_entries_and_control_characters() {
    let ir = parse(json!([
        function("fooBar",json!([]),json!([])),
        function("foo_bar",json!([{"name":"x","type":"bool"}]),json!([])),
        function("fooBar",json!([{"name":"x","type":"uint256"}]),json!([])),
        {"type":"event","name":"Changed","anonymous":false,"inputs":[]},
        {"type":"error","name":"Denied","inputs":[]}
    ]));
    let source = elixir::render_elixir_file(&ir, true);
    assert!(source.contains("@macro_abi_json"));
    assert!(source.contains("Binding1"));
    assert!(source.contains("Changed"));
    assert!(source.contains("Denied"));
    for (name, expected) in [("$", "Contract"), ("a_B", "AB"), ("a_b", "AB")] {
        assert_eq!(elixir::namespace_name(name), expected);
    }
    let ir = parse(
        json!([{"type":"event","name":"Note","anonymous":false,"inputs":[],"extra":"\u{0001}#x#{x}"}]),
    );
    let source = elixir::render_elixir_file(&ir, false);
    assert!(source.contains("\\\\u0001"));
    assert!(source.contains("\\#{x}"));
}

#[test]
fn elixir_reserved_helpers_only_collide_at_the_reserved_arity() {
    let ir = parse(json!([
        function("__default_address__", json!([]), json!([])),
        function(
            "__contract_binary__",
            json!([{"name":"x","type":"uint256"}]),
            json!([])
        )
    ]));
    let errors = elixir::unsupported_sdk_collisions(&ir);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("__default_address__()"));
}

#[test]
fn typescript_no_wrapper_dispatch_keeps_only_the_primary_abi_module() {
    use abi_typegen_codegen::generate_contract_files;
    use abi_typegen_config::{Config, Target};
    let ir = parse(json!([function(
        "read",
        json!([]),
        json!([{"name":"ok","type":"bool"}])
    )]));
    for target in [
        Target::Wagmi,
        Target::Ethers,
        Target::Ethers5,
        Target::Web3js,
    ] {
        let config = Config {
            targets: vec![target],
            wrappers: false,
            ..Config::from_toml_str("").expect("config")
        };
        let files = generate_contract_files(&ir, &config);
        assert_eq!(files.len(), 1);
        assert!(files["Sample.abi.ts"].contains("export const SampleAbi"));
        assert!(files["Sample.abi.ts"].contains("\"read\""));
    }
}

#[test]
fn rust_unnamed_event_fields_and_numeric_helper_names_keep_distinct_positions() {
    let ir = parse(json!([
        {"type":"event","name":"Changed","anonymous":false,"inputs":[
            {"name":"","type":"bool","indexed":true},
            {"name":"value2","type":"uint256","indexed":false},
            {"name":"value_2","type":"bool","indexed":false}
        ]},
        {"type":"error","name":"Denied","inputs":[{"name":"item10","type":"uint256"}]}
    ]));
    let source = rust::render_rust_file(&ir, false);
    assert!(source.contains("event Changed(bool indexed,"));
    assert!(source.contains("uint256 value2"));
    assert!(source.contains("bool value_22"));
    assert!(source.contains("error Denied(uint256 item10)"));
}

#[test]
fn yaml_unnamed_tuple_components_and_natspec_without_notice_preserve_abi_shapes() {
    let mut ir = parse(json!([
        function("read",json!([]),json!([{"name":"item","type":"tuple","components":[{"name":"","type":"bool"}]}])),
        {"type":"event","name":"Empty","anonymous":false,"inputs":[]},
        {"type":"error","name":"Denied","inputs":[]}
    ]));
    ir.events[0].natspec = Some(NatSpec::default());
    ir.errors[0].natspec = Some(NatSpec::default());
    let source = abi_typegen_codegen::yaml::render_yaml_file(&ir);
    assert!(source.contains("type: tuple"));
    assert!(source.contains("type: \"bool\""));
    assert!(!source.contains("name: \"\""));
    assert!(!source.contains("notice:"));
}
