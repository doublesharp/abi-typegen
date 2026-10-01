use abi_typegen_codegen::{generate_contract_files, php};
use abi_typegen_config::{Config, Target};
use abi_typegen_core::parser::parse_artifact;
use serde_json::{Value, json};

fn function(name: &str, inputs: Value, outputs: Value, mutability: &str) -> Value {
    json!({"type":"function", "name":name, "inputs":inputs, "outputs":outputs, "stateMutability":mutability})
}

fn render(entries: Value, wrappers: bool) -> String {
    let ir = parse_artifact("Sample", &json!({"abi":entries}).to_string()).expect("ABI");
    php::render_php_file(&ir, "App\\Contracts", wrappers)
}

#[test]
fn dispatch_escapes_reserved_classes_and_keeps_metadata_without_wrappers() {
    let ir = parse_artifact("Array", r#"{"abi":[]}"#).expect("ABI");
    let config = Config {
        targets: vec![Target::Php],
        wrappers: false,
        package: "App\\Contracts".into(),
        ..Config::from_toml_str("").expect("config")
    };
    let files = generate_contract_files(&ir, &config);
    assert_eq!(files.len(), 1);
    let source = &files["ArrayContract.php"];
    assert!(source.contains("namespace App\\Contracts;"));
    assert!(source.contains("final class ArrayContract"));
    assert!(source.contains("public const ABI"));
    assert!(!source.contains("function encode"));
    assert!(!source.contains("class ArrayContractClient"));
}

#[test]
fn namespaces_handle_empty_and_case_insensitive_reserved_names() {
    for name in [
        "Array", "array", "CLASS", "String", "self", "parent", "static",
    ] {
        let actual = php::namespace_name(name);
        assert!(actual.ends_with("Contract"), "{name}: {actual}");
    }
    for (name, expected) in [
        ("", "Contract"),
        ("_", "Contract"),
        ("foo_bar", "FooBar"),
        ("_123", "X123"),
    ] {
        assert_eq!(php::namespace_name(name), expected);
    }
}

#[test]
fn global_namespace_does_not_emit_an_empty_namespace_statement() {
    let ir = parse_artifact("Sample", r#"{"abi":[]}"#).expect("ABI");
    for namespace in ["", " ", "\\App\\Contracts\\"] {
        let source = php::render_php_file(&ir, namespace, false);
        if namespace.trim().is_empty() {
            assert!(!source.contains("namespace "));
        } else {
            assert!(source.contains("namespace App\\Contracts;"));
        }
    }
}

#[test]
fn scalar_types_preserve_integer_widths_and_exact_bytes_layouts() {
    let params = json!([
        {"name":"flag","type":"bool"}, {"name":"owner","type":"address"},
        {"name":"text","type":"string"}, {"name":"amount","type":"uint256"},
        {"name":"delta","type":"int8"}, {"name":"blob","type":"bytes"}, {"name":"key","type":"bytes3"}
    ]);
    let source = render(
        json!([function("echo", params.clone(), params, "pure")]),
        true,
    );
    for expected in [
        "bool $flag",
        "string $owner",
        "string $text",
        "\\Brick\\Math\\BigInteger $amount",
        "\\Brick\\Math\\BigInteger $delta",
        "string $blob",
        "string $key",
        "'type' => 'uint256'",
        "'type' => 'int8'",
        "'type' => 'bytes3'",
        "return new SampleEchoResult(",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
}

#[test]
fn nested_tuple_arrays_have_named_values_and_distinct_iteration_variables() {
    let tuple = json!({"name":"items","type":"tuple[][2]", "internalType":"struct Sample.Item[][2]", "components":[
        {"name":"amount","type":"uint256"}, {"name":"flags","type":"bool[2]"},
        {"name":"nested","type":"tuple", "components":[{"name":"blob","type":"bytes"}]}
    ]});
    let source = render(
        json!([function(
            "echo",
            json!([tuple.clone()]),
            json!([tuple]),
            "pure"
        )]),
        true,
    );
    for expected in [
        "final readonly class SampleItem",
        "array $items",
        "'size' => 2",
        "'size' => null",
        "array_map(static fn($item0)",
        "array_map(static fn($item1)",
        "->amount",
        "->flags",
        "->nested",
        "new SampleItem(",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
}

#[test]
fn zero_single_and_multiple_outputs_use_the_correct_return_shape() {
    let source = render(
        json!([
            function("ping", json!([]), json!([]), "view"),
            function(
                "flag",
                json!([]),
                json!([{"name":"ok","type":"bool"}]),
                "pure"
            ),
            function(
                "pair",
                json!([]),
                json!([{"name":"a","type":"uint256"},{"name":"b","type":"string"}]),
                "view"
            )
        ]),
        true,
    );
    assert!(source.contains("decodePingResult(string $data): void"));
    assert!(source.contains("callPing(SampleClient $client): void"));
    assert!(source.contains("decodeFlagResult(string $data): bool"));
    assert!(source.contains("return $values[0];"));
    assert!(source.contains("decodePairResult(string $data): SamplePairResult"));
    assert!(source.contains("return new SamplePairResult($values[0], $values[1]);"));
}

#[test]
fn payable_and_nonpayable_writes_reserve_client_and_option_parameter_names() {
    let source = render(
        json!([
            function("deposit", json!([]), json!([]), "payable"),
            function(
                "update",
                json!([{"name":"client","type":"address"},{"name":"options","type":"uint256"}]),
                json!([]),
                "nonpayable"
            )
        ]),
        true,
    );
    assert!(
        source.contains(
            "sendDeposit(SampleClient $client, SampleTransactionOptions $options): string"
        )
    );
    assert!(source.contains("sendUpdate(SampleClient $client, string $client2, \\Brick\\Math\\BigInteger $options2, SampleTransactionOptions $options)"));
    assert_eq!(
        source
            .matches("nonpayable function cannot receive value")
            .count(),
        1
    );
    assert!(source.contains("self::encodeUpdate($client2, $options2)"));
}

#[test]
fn anonymous_events_do_not_require_or_filter_a_signature_topic() {
    let source = render(
        json!([{"type":"event","name":"Hidden","anonymous":true,"inputs":[
            {"name":"amount","type":"uint256","indexed":true},{"name":"ok","type":"bool","indexed":false}
        ]}]),
        true,
    );
    assert!(source.contains("if (count($topics) !== 1) return null;"));
    assert!(
        source
            .contains("self::decodeTop($topics[0], [['kind' => 'atom', 'type' => 'uint256']])[0]")
    );
    assert!(!source.contains("strcasecmp($topics[0]"));
    assert!(source.contains("?\\Brick\\Math\\BigInteger $amount = null"));
}

#[test]
fn indexed_reference_types_use_hash_records_and_checked_filter_values() {
    let inputs = json!([
        {"name":"text","type":"string","indexed":true},{"name":"blob","type":"bytes","indexed":true},
        {"name":"items","type":"uint256[2]","indexed":true},
        {"name":"tuple","type":"tuple","indexed":true,"components":[{"name":"value","type":"bool"}]}
    ]);
    let source = render(
        json!([{"type":"event","name":"References","anonymous":true,"inputs":inputs}]),
        true,
    );
    assert!(source.contains("final readonly class SampleDecodedReferencesEvent"));
    for index in 0..4 {
        assert!(source.contains(&format!("self::topicHash($topics[{index}])")));
    }
    for name in ["text", "blob", "items", "tuple"] {
        assert!(source.contains(&format!(
            "${name} === null ? null : self::topicHash(${name})"
        )));
    }
}

#[test]
fn metadata_records_include_empty_events_errors_and_multi_results() {
    let ir = parse_artifact("Sample", &json!({"abi":[
        function("pair", json!([]), json!([{"name":"x","type":"uint256"},{"name":"y","type":"bool"}]), "view"),
        {"type":"event","name":"Empty","anonymous":false,"inputs":[]},
        {"type":"error","name":"Denied","inputs":[]}
    ]}).to_string()).expect("ABI");
    let metadata = php::declared_class_names(&ir, false);
    assert_eq!(
        metadata,
        [
            "Sample",
            "SampleDeniedError",
            "SampleEmptyEvent",
            "SamplePairResult"
        ]
    );
    let wrapped = php::declared_class_names(&ir, true);
    assert_eq!(wrapped.len(), metadata.len() + 2);
    assert!(wrapped.contains(&"SampleClient".into()));
    assert!(wrapped.contains(&"SampleTransactionOptions".into()));
    let source = php::render_php_file(&ir, "App", false);
    assert!(source.contains("public function __construct() {}"));
    assert!(source.contains("DENIED_ERROR_SELECTOR"));
    assert!(!source.contains("decodeDeniedError("));
}

#[test]
fn case_insensitive_helpers_and_record_fields_cannot_collide() {
    let source = render(
        json!([
            function(
                "foo_bar",
                json!([{"name":"self","type":"bool"},{"name":"SELF","type":"bool"},{"name":"","type":"uint256"}]),
                json!([]),
                "view"
            ),
            function(
                "FooBar",
                json!([{"name":"x","type":"uint256"}]),
                json!([]),
                "view"
            )
        ]),
        true,
    );
    assert!(source.contains("encodeFooBar("));
    assert!(source.contains("encodeFooBar2("));
    assert!(source.contains("bool $self"));
    assert!(source.contains("bool $SELF"));
    assert!(source.contains("\\Brick\\Math\\BigInteger $uint256"));
}

#[test]
fn error_helpers_keep_zero_argument_and_typed_payloads_distinct() {
    let source = render(
        json!([
            {"type":"error","name":"Denied","inputs":[]},
            {"type":"error","name":"Denied","inputs":[{"name":"reason","type":"string"}]}
        ]),
        true,
    );
    assert!(source.contains("decodeDenied0Error(string $data): ?SampleDenied0Error"));
    assert!(source.contains("decodeDenied1Error(string $data): ?SampleDenied1Error"));
    assert!(source.contains("return new SampleDenied0Error();"));
    assert!(source.contains("return new SampleDenied1Error($values[0]);"));
}
