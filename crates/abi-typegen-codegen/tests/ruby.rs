use abi_typegen_codegen::ruby;
use abi_typegen_core::parser::parse_artifact;
use abi_typegen_core::types::{ContractIr, NatSpec};
use serde_json::{Value, json};

fn function(name: &str, inputs: Value, outputs: Value, mutability: &str) -> Value {
    json!({"type":"function","name":name,"inputs":inputs,"outputs":outputs,"stateMutability":mutability})
}

fn ir(name: &str, entries: Value) -> ContractIr {
    parse_artifact(name, &json!({"abi":entries}).to_string()).expect("valid ABI")
}

fn render(entries: Value) -> String {
    ruby::render_ruby_file_with_wrappers(&ir("Sample", entries), true)
}

#[test]
fn constructor_only_named_tuples_are_available_in_both_output_modes() {
    let ir = ir(
        "Sample",
        json!([{"type":"constructor","stateMutability":"payable","inputs":[
            {"name":"settings","type":"tuple","internalType":"struct Sample.Settings","components":[
                {"name":"owner","type":"address"},{"name":"limit","type":"uint256"}
            ]}
        ]}]),
    );
    for wrappers in [false, true] {
        let source = ruby::render_ruby_file_with_wrappers(&ir, wrappers);
        assert!(source.contains("SampleSettings = Struct.new(:owner, :limit, keyword_init: true)"));
        assert!(ruby::declared_constant_names(&ir, wrappers).contains(&"SampleSettings".into()));
    }
}

#[test]
fn tuple_conversion_uses_struct_storage_when_fields_shadow_conversion_methods() {
    let source = render(json!([function(
        "store",
        json!([{
            "name":"settings","type":"tuple","internalType":"struct Sample.Settings","components":[
                {"name":"to_a","type":"uint256"},{"name":"owner","type":"address"}
            ]
        }]),
        json!([]),
        "nonpayable"
    )]));
    assert_eq!(
        source
            .matches("Struct.instance_method(:to_a).bind_call(value)")
            .count(),
        2
    );
}

#[test]
fn indexed_scalars_are_checked_for_canonical_encoding_before_returning_fields() {
    let source = render(
        json!([{"type":"event","name":"Scalar","anonymous":false,"inputs":[
            {"name":"ok","type":"bool","indexed":true},{"name":"key","type":"bytes4","indexed":true}
        ]}]),
    );
    for expected in [
        "_assert_canonical([\"bool\"], indexed_values1, topics[1])",
        "_assert_canonical([\"bytes4\"], indexed_values2, topics[2])",
        "decoded[\"ok\"] = indexed_values1[0]",
        "decoded[\"key\"] = indexed_values2[0]",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
}

#[test]
fn metadata_and_legacy_stubs_keep_abi_without_callable_wrappers() {
    let mut ir = ir(
        "Sample",
        json!([
            function("read", json!([]), json!([]), "view"),
            function(
                "write",
                json!([{"name":"n","type":"uint256"}]),
                json!([]),
                "nonpayable"
            )
        ]),
    );
    ir.natspec = Some(NatSpec {
        notice: Some(" Contract\nnotice ".into()),
        ..NatSpec::default()
    });
    ir.functions[0].natspec = Some(NatSpec::default());
    ir.functions[1].natspec = Some(NatSpec {
        notice: Some(" Write\nnotice ".into()),
        ..NatSpec::default()
    });
    let metadata = ruby::render_ruby_file_with_wrappers(&ir, false);
    assert!(metadata.contains("SAMPLE_ABI = JSON.parse"));
    assert!(!metadata.contains("require \"eth\""));
    assert!(!metadata.contains("SampleContract"));
    let legacy = ruby::render_ruby_file(&ir);
    for expected in [
        "# Contract notice",
        "# Write notice",
        "def read\n",
        "def write(n)",
        "raise NotImplementedError",
    ] {
        assert!(legacy.contains(expected), "missing {expected}");
    }
    assert!(!legacy.contains("def encode_"));
}

#[test]
fn legacy_overload_suffixes_cover_empty_and_typed_arguments_in_each_partition() {
    let source = ruby::render_ruby_file(&ir(
        "Sample",
        json!([
            function("read", json!([]), json!([]), "view"),
            function(
                "read",
                json!([{"name":"n","type":"uint256"}]),
                json!([]),
                "pure"
            ),
            function("write", json!([]), json!([]), "nonpayable"),
            function(
                "write",
                json!([{"name":"n","type":"uint256"}]),
                json!([]),
                "payable"
            )
        ]),
    ));
    for expected in [
        "def read\n",
        "def read_uint256(n)",
        "def write\n",
        "def write_uint256(n)",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
}

#[test]
fn payable_and_nonpayable_functions_and_constructors_validate_transaction_value() {
    for mutability in ["payable", "nonpayable"] {
        let source = render(json!([
            function("write", json!([]), json!([]), mutability),
            {"type":"constructor","stateMutability":mutability,"inputs":[]}
        ]));
        assert!(source.contains("def write(transaction = {})"));
        assert!(source.contains("def self.build_deployment(bytecode, transaction = {})"));
        assert!(source.contains("Eth::Abi.encode([], [])"));
        if mutability == "payable" {
            assert_eq!(
                source
                    .matches("Transaction value must be a nonnegative Integer")
                    .count(),
                2
            );
        } else {
            assert!(source.contains("Function is not payable"));
            assert!(source.contains("Constructor is not payable"));
        }
    }
}

#[test]
fn read_return_shapes_include_void_single_and_multiple_values() {
    let source = render(json!([
        function("noop", json!([]), json!([]), "pure"),
        function(
            "one",
            json!([]),
            json!([{"name":"n","type":"uint256"}]),
            "view"
        ),
        function(
            "many",
            json!([{"name":"n","type":"uint256"}]),
            json!([{"name":"n","type":"uint256"},{"name":"ok","type":"bool"}]),
            "view"
        )
    ]));
    for expected in [
        "def noop\n",
        "def many(n)",
        "Unexpected output for void function",
        "    values[0]\n",
        "    values\n",
        "_assert_canonical(function.outputs.map(&:parsed_type), values, data)",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
}

#[test]
fn functions_cannot_replace_generated_encoders_decoders_or_accessors() {
    let names = [
        "foo",
        "encodeFoo",
        "decodeFooResult",
        "client",
        "address",
        "contract",
        "buildDeployment",
        "initialize",
        "class",
        "_0",
        "_",
        "foo",
    ];
    let source = render(Value::Array(
        names
            .into_iter()
            .enumerate()
            .map(|(index, name)| {
                function(
                    name,
                    if index == 11 {
                        json!([{"name":"n","type":"uint256"}])
                    } else {
                        json!([])
                    },
                    json!([]),
                    "view",
                )
            })
            .collect(),
    ));
    for expected in [
        "def foo\n",
        "def encode_foo_2\n",
        "def decode_foo_result_2\n",
        "def client_2\n",
        "def address_2\n",
        "def contract_2\n",
        "def build_deployment_2\n",
        "def initialize_\n",
        "def class_\n",
        "def method_0\n",
        "def method_\n",
        "def foo_uint256(n)",
    ] {
        assert!(source.contains(expected), "missing {expected}\n{source}");
    }
}

#[test]
fn later_functions_and_events_do_not_replace_preexisting_abi_methods() {
    let source = render(json!([
        function("encodeLeading", json!([]), json!([]), "view"),
        function("leading", json!([]), json!([]), "view"),
        function("decodeTrailingResult", json!([]), json!([]), "view"),
        function("trailing", json!([]), json!([]), "view"),
        function("decodeOnlyLog", json!([]), json!([]), "view"),
        {"type":"event","name":"Only","anonymous":false,"inputs":[]}
    ]));
    for expected in [
        "def leading_2\n",
        "def trailing_2\n",
        "def decode_only_log\n",
        "def decode_only_2_log(",
        "def filter_only_2(",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
}

#[test]
fn keyword_digit_and_duplicate_arguments_have_unique_legal_ruby_names() {
    let source = render(json!([function(
        "store",
        json!([
            {"name":"","type":"bool"},{"name":"_","type":"bool"},{"name":"0x","type":"bool"},
            {"name":"next","type":"bool"},{"name":"x","type":"bool"},{"name":"x","type":"bool"},
            {"name":"transaction","type":"bool"}
        ]),
        json!([]),
        "nonpayable"
    )]));
    assert!(
        source
            .contains("def store(arg0, arg1, arg2, next_, x, x2, transaction2, transaction = {})")
    );
}

#[test]
fn anonymous_and_overloaded_events_and_errors_have_distinct_helpers() {
    let source = render(json!([
        function("filterNote", json!([]), json!([]), "view"),
        function("decodeNoteLog", json!([]), json!([]), "view"),
        function("decodeDeniedError", json!([]), json!([]), "view"),
        {"type":"event","name":"Note","anonymous":true,"inputs":[{"name":"n","type":"uint256","indexed":false}]},
        {"type":"event","name":"Note","anonymous":false,"inputs":[]},
        {"type":"error","name":"Denied","inputs":[]},
        {"type":"error","name":"Denied","inputs":[{"name":"n","type":"uint256"}]}
    ]));
    for expected in [
        "NOTE_1_EVENT_SIGNATURE",
        "NOTE_2_EVENT_TOPIC",
        "def filter_note_2(",
        "def filter_note_3(",
        "def decode_denied_2_error(",
        "def decode_denied_3_error(",
        "DENIED_1_ERROR_SELECTOR",
        "DENIED_2_ERROR_SELECTOR",
        "topics = []",
        "topics.length == 0",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
    assert!(!source.contains("NOTE_1_EVENT_TOPIC"));
}

#[test]
fn indexed_filters_preserve_reference_hashes_and_encode_scalar_topics() {
    let source = render(
        json!([{"type":"event","name":"References","anonymous":false,"inputs":[
            {"name":"label","type":"string","indexed":true},{"name":"payload","type":"bytes","indexed":true},
            {"name":"items","type":"uint256[]","indexed":true}
        ]},{"type":"event","name":"Composite","anonymous":false,"inputs":[
            {"name":"fixed","type":"uint256[2]","indexed":true},
            {"name":"point","type":"tuple","indexed":true,"components":[{"name":"x","type":"uint256"}]},
            {"name":"owner","type":"address","indexed":true},{"name":"n","type":"uint256","indexed":false}
        ]}]),
    );
    for expected in [
        "_topic_hash_string(label)",
        "_topic_hash_bytes(payload)",
        "_topic_hash_preencoded(items)",
        "_topic_hash_preencoded(fixed)",
        "_topic_hash_preencoded(point)",
        "Eth::Abi.encode([\"address\"], [_abi_value(owner)])",
        "decoded[\"point\"] = topics[2]",
        "decoded[\"owner\"] = indexed_values3[0]",
        "decoded[\"n\"] = values[0]",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
}

#[test]
fn tuple_declarations_cover_nested_arrays_unnamed_and_reused_structs() {
    let inner = json!({"name":"points","type":"tuple[][2]","internalType":"struct Sample.Point[][2]","components":[{"name":"next","type":"bool"},{"name":"next","type":"bool"}]});
    let entry = json!({"name":"entry","type":"tuple","internalType":"struct Sample.Entry","components":[inner]});
    let ir = ir(
        "Sample",
        json!([
            function("echo", json!([entry.clone(),{"name":"","type":"tuple","components":[{"name":"x","type":"uint256"}]}]), json!([entry]), "view"),
            {"type":"event","name":"Changed","anonymous":false,"inputs":[{"name":"eventTuple","type":"tuple","indexed":false,"components":[{"name":"x","type":"uint256"}]}]},
            {"type":"error","name":"Denied","inputs":[{"name":"errorTuple","type":"tuple","components":[{"name":"ok","type":"bool"}]}]}
        ]),
    );
    let source = ruby::render_ruby_file_with_wrappers(&ir, true);
    for expected in [
        "SampleEntry = Struct.new(:points",
        "SamplePoint = Struct.new(:next_, :next_2",
        "SampleEventTuple = Struct.new",
        "SampleErrorTuple = Struct.new",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
    assert_eq!(source.matches("SampleEntry = Struct.new").count(), 1);
    assert_eq!(ruby::declared_constant_names(&ir, false).len(), 5);
}

#[test]
fn unusual_contract_names_keep_legal_top_level_constants() {
    for (name, abi_name, class_name) in [
        ("", "CONTRACT__ABI", "ContractBinding"),
        ("_", "CONTRACT__ABI", "ContractBinding"),
        ("123", "CONTRACT_123_ABI", "X123Contract"),
    ] {
        let source = ruby::render_ruby_file_with_wrappers(&ir(name, json!([])), true);
        assert!(source.contains(&format!("{abi_name} = JSON.parse")));
        assert!(source.contains(&format!("class {class_name}")), "{source}");
    }
}
