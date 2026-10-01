use abi_typegen_codegen::{dart, generate_contract_files};
use abi_typegen_config::{Config, Target};
use abi_typegen_core::parser::parse_artifact;
use serde_json::{Value, json};

#[test]
fn nested_tuple_array_errors_use_the_canonical_adapter_signature() {
    let source = render(
        json!([
            {"type":"error","name":"NestedDenied","inputs":[{"name":"items","type":"tuple[]","internalType":"struct Sample.Item[]","components":[{"name":"count","type":"int8"},{"name":"ok","type":"bool"}]}]}
        ]),
        true,
    );
    assert!(source.contains("NestedDenied((int8,bool)[])"));
    assert!(source.contains("final List<SampleItem> items;"));
    let adapter = source
        .lines()
        .find(|line| line.contains("get _NestedDeniedError"))
        .expect("adapter");
    let literal = adapter
        .split_once("ContractAbi.fromJson(")
        .expect("ABI expression")
        .1;
    let abi_text = serde_json::Deserializer::from_str(literal)
        .into_iter::<String>()
        .next()
        .expect("literal")
        .expect("string");
    let abi: Value = serde_json::from_str(&abi_text).expect("ABI");
    assert_eq!(abi[0]["type"], "function");
    assert_eq!(abi[0]["outputs"][0]["type"], "tuple[]");
    assert_eq!(abi[0]["outputs"][0]["components"][0]["type"], "int8");
}

#[test]
fn event_and_error_names_avoid_existing_overload_suffixes() {
    let source = render(
        json!([
            {"type":"event","name":"Changed","inputs":[{"name":"x","type":"uint256","indexed":false}],"anonymous":false},
            {"type":"event","name":"Changed","inputs":[{"name":"x","type":"bool","indexed":false}],"anonymous":false},
            {"type":"event","name":"ChangedByUint256","inputs":[{"name":"x","type":"uint256","indexed":false}],"anonymous":false},
            {"type":"error","name":"Denied","inputs":[{"name":"x","type":"uint256"}]},
            {"type":"error","name":"Denied","inputs":[{"name":"x","type":"bool"}]},
            {"type":"error","name":"DeniedByUint256","inputs":[{"name":"x","type":"uint256"}]}
        ]),
        true,
    );
    assert!(source.contains("class SampleChangedByUint256Event2Event"));
    assert!(source.contains("class SampleErrorDeniedByUint256Error2"));
    assert!(source.contains("\"ChangedByUint256(uint256)\""));
    assert!(source.contains("\"DeniedByUint256(uint256)\""));
}

#[test]
fn normalized_result_names_match_decoder_arguments() {
    let source = render(
        json!([function(
            "read",
            json!([]),
            json!([
                {"name":"runtimeType","type":"uint256"}, {"name":"runtimeType_","type":"bool"}
            ]),
            "view"
        )]),
        true,
    );
    assert!(source.contains("final bool runtimeType_2;"));
    assert!(source.contains("runtimeType_2: r[1] as bool"));
}

#[test]
fn contract_names_with_dollars_are_safe_sdk_string_literals() {
    let ir = parse_artifact("Dollar$Token", &json!({"abi":[]}).to_string()).expect("ABI");
    let source = dart::render_dart_file(&ir, true);
    assert!(source.contains("ContractAbi.fromJson(DollarTokenAbiJson, \"Dollar\\$Token\")"));
}

#[test]
fn generated_value_fields_avoid_object_members() {
    let source = render(
        json!([function(
            "read",
            json!([]),
            json!([
                {"name":"runtimeType","type":"uint256"},
                {"name":"hashCode","type":"bool"},
                {"name":"toString","type":"string"}
            ]),
            "view"
        )]),
        true,
    );
    for expected in [
        "final BigInt runtimeType_;",
        "final bool hashCode_;",
        "final String toString_;",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
}

#[test]
fn dollar_names_are_literal_in_signatures_event_lookup_and_validation() {
    let source = render(
        json!([
            function("foo$bar", json!([{"name":"x$y","type":"uint8"}]), json!([]), "view"),
            {"type":"event","name":"Changed$Now","inputs":[],"anonymous":false},
            {"type":"error","name":"Denied$Now","inputs":[]}
        ]),
        true,
    );
    for expected in [
        "Signature = \"foo\\$bar(uint8)\"",
        "e.stringSignature == \"Changed\\$Now()\"",
        "ErrorSignature = \"Denied\\$Now()\"",
        "value out of range for 'x\\$y'",
        "const SampleChangedNowEvent();",
    ] {
        assert!(source.contains(expected), "missing {expected}: {source}");
    }
}

fn render(entries: Value, wrappers: bool) -> String {
    let ir = parse_artifact("Sample", &json!({"abi": entries}).to_string()).expect("valid ABI");
    dart::render_dart_file(&ir, wrappers)
}

fn function(name: &str, inputs: Value, outputs: Value, mutability: &str) -> Value {
    json!({"type": "function", "name": name, "inputs": inputs, "outputs": outputs, "stateMutability": mutability})
}

#[test]
fn dart_dispatch_emits_one_normalized_source_file_in_both_modes() {
    let ir = parse_artifact("Token", include_str!("../../../tests/fixtures/erc20.json"))
        .expect("fixture");
    for wrappers in [false, true] {
        let config = Config {
            targets: vec![Target::Dart],
            wrappers,
            ..Config::from_toml_str("").expect("default config")
        };
        let files = generate_contract_files(&ir, &config);
        assert_eq!(files.len(), 1);
        let source = &files["token.dart"];
        assert!(source.contains("const String TokenAbiJson = \"["));
        assert!(source.contains("transferSignature = \"transfer(address,uint256)\""));
        assert!(source.contains("transferSelector = '0xa9059cbb'"));
        assert!(source.contains("Uint8List transferCall("));
        assert_eq!(source.contains("client.sendTransaction("), wrappers);
    }
}

#[test]
fn namespaces_avoid_sdk_type_names_and_handle_empty_or_numeric_names() {
    for (input, namespace, filename) in [
        ("BigInt", "BigIntContract", "bigIntContract.dart"),
        (
            "Web3Client",
            "Web3ClientContract",
            "web3ClientContract.dart",
        ),
        ("String", "StringContract", "stringContract.dart"),
        ("", "Contract", "contract.dart"),
        ("123_token", "_123Token", "123token.dart"),
        ("foo_bar", "FooBar", "fooBar.dart"),
    ] {
        assert_eq!(dart::namespace_name(input), namespace);
        assert_eq!(dart::file_name(input), filename);
    }
}

#[test]
fn empty_abi_keeps_a_usable_contract_and_embedded_abi() {
    let source = render(json!([]), false);
    assert!(source.contains("const String SampleAbiJson = \"[]\";"));
    assert!(source.contains("class Sample {"));
    assert!(
        source.contains("Sample(this.address) : contract = DeployedContract(SampleAbi, address);")
    );
    assert!(!source.contains("Future<"));
}

#[test]
fn metadata_mode_retains_offline_codecs_and_types_without_rpc_wrappers() {
    let source = render(
        json!([
            function("read", json!([]), json!([{"name":"n","type":"uint256"}]), "view"),
            function("write", json!([{"name":"n","type":"uint256"}]), json!([]), "nonpayable"),
            {"type":"event","name":"Changed","inputs":[{"name":"n","type":"uint256","indexed":false}],"anonymous":false},
            {"type":"error","name":"Denied","inputs":[{"name":"n","type":"uint256"}]}
        ]),
        false,
    );
    for expected in [
        "Uint8List readCall()",
        "BigInt decodeReadResult(String data)",
        "Uint8List writeCall({required BigInt n})",
        "class SampleChangedEvent",
        "class SampleErrorDenied",
        "decodeDeniedError(String data)",
        "SampleDeniedErrorSignature = \"Denied(uint256)\"",
        "SampleDeniedErrorSelector = '0x7e46dab6'",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
    for absent in [
        "client.call(",
        "client.sendTransaction(",
        "client.getLogs(",
        "decodeChangedEvent(",
    ] {
        assert!(!source.contains(absent), "unexpected {absent}");
    }
}

#[test]
fn read_and_write_wrappers_follow_mutability_and_payable_options() {
    let source = render(
        json!([
            function(
                "read",
                json!([]),
                json!([{"name":"x","type":"bool"}]),
                "view"
            ),
            function("noop", json!([]), json!([]), "pure"),
            function("write", json!([]), json!([]), "nonpayable"),
            function("deposit", json!([]), json!([]), "payable")
        ]),
        true,
    );
    assert!(source.contains("Future<bool> read(Web3Client client) async"));
    assert!(source.contains("Future<void> noop(Web3Client client) async"));
    assert_eq!(source.matches("await client.call(").count(), 2);
    assert_eq!(source.matches("client.sendTransaction(").count(), 2);
    assert!(
        source.contains(
            "write(Web3Client client, Credentials credentials, {int? chainId, int? maxGas"
        )
    );
    assert!(source.contains("deposit(Web3Client client, Credentials credentials, {int? chainId, EtherAmount? value, int? maxGas"));
    assert_eq!(source.matches("value: value,").count(), 1);
}

#[test]
fn scalar_and_nested_array_types_keep_exact_dart_types() {
    let inputs = json!([
        {"name":"unsigned","type":"uint256"}, {"name":"signed","type":"int8"},
        {"name":"flag","type":"bool"}, {"name":"owner","type":"address"},
        {"name":"blob","type":"bytes"}, {"name":"key","type":"bytes32"},
        {"name":"label","type":"string"}, {"name":"grid","type":"uint16[][2]"}
    ]);
    let source = render(
        json!([function("echo", inputs.clone(), inputs, "pure")]),
        true,
    );
    for field in [
        "BigInt unsigned",
        "BigInt signed",
        "bool flag",
        "EthereumAddress owner",
        "Uint8List blob",
        "Uint8List key",
        "String label",
        "List<List<BigInt>> grid",
    ] {
        assert!(source.contains(field), "missing {field}");
    }
    assert!(source.contains("class SampleEchoFunctionResult"));
    assert!(source.contains("owner: r[3] as EthereumAddress"));
    assert!(source.contains("grid: (r[7] as List<dynamic>).map((e) => (e as List<dynamic>).map((e) => e as BigInt).toList()).toList()"));
}

#[test]
fn tuple_arrays_have_named_fields_and_recursive_offline_conversion() {
    let tuple = json!({"name":"items","type":"tuple[]","internalType":"struct Sample.Item[]","components":[{"name":"count","type":"uint8"},{"name":"key","type":"bytes2"},{"name":"flags","type":"bool[2]"}]});
    let source = render(
        json!([function(
            "echo",
            json!([tuple.clone()]),
            json!([tuple]),
            "pure"
        )]),
        false,
    );
    for expected in [
        "class SampleItem",
        "final BigInt count;",
        "final Uint8List key;",
        "final List<bool> flags;",
        "required List<SampleItem> items",
        "items.map((e) => e.toAbi()).toList()",
        "SampleItem.fromAbi(e as List<dynamic>)",
        "abiItem0.count.bitLength > 8",
        "abiItem0.key.length != 2",
        "abiItem0.flags.length != 2",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
}

#[test]
fn range_and_size_validation_runs_before_encoding_and_rpc_calls() {
    let source = render(
        json!([function(
            "set",
            json!([
                {"name":"n","type":"uint8"}, {"name":"signed","type":"int16"},
                {"name":"key","type":"bytes3"}, {"name":"values","type":"uint8[2]"}
            ]),
            json!([]),
            "nonpayable"
        )]),
        true,
    );
    for guard in [
        "n.isNegative || n.bitLength > 8",
        "signed < -(BigInt.one << 15) || signed > (BigInt.one << 15) - BigInt.one",
        "key.length != 3",
        "values.length != 2",
        "abiItem3.isNegative || abiItem3.bitLength > 8",
    ] {
        assert!(source.contains(guard), "missing {guard}");
    }
    let validation_call = "_validateSetArgs(n: n, signed: signed, key: key, values: values);";
    let write = source.split("Future<String> set(").nth(1).expect("write");
    assert!(
        write.find(validation_call).expect("validation")
            < write.find("client.sendTransaction(").expect("RPC")
    );
    let offline = source.split("Uint8List setCall(").nth(1).expect("offline");
    assert!(
        offline.find(validation_call).expect("validation")
            < offline.find("_set.encodeCall(").expect("encode")
    );
}

#[test]
fn function_overloads_and_normalized_collisions_keep_canonical_identities() {
    let source = render(
        json!([
            function(
                "fooBar",
                json!([{"name":"x","type":"uint256"}]),
                json!([]),
                "view"
            ),
            function(
                "foo_bar",
                json!([{"name":"x","type":"uint256"}]),
                json!([]),
                "view"
            ),
            function("fooBar", json!([]), json!([]), "view"),
            function("address", json!([]), json!([]), "view")
        ]),
        true,
    );
    for expected in [
        "fooBarByUint256Signature = \"fooBar(uint256)\"",
        "fooBarByUint256Selector = '0xf548f646'",
        "fooBarByUint256ByFunction1Signature = \"foo_bar(uint256)\"",
        "fooBarByUint256ByFunction1Selector = '0x5e25f0c6'",
        "fooBarByFunction2Signature = \"fooBar()\"",
        "addressFunctionSignature = \"address()\"",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
    assert!(source.contains("f.encodeName() == fooBarByUint256ByFunction1Signature"));
}

#[test]
fn argument_names_avoid_keywords_wrapper_parameters_and_duplicates() {
    let source = render(
        json!([function(
            "set",
            json!([
                {"name":"class","type":"bool"}, {"name":"client","type":"bool"},
                {"name":"value","type":"bool"}, {"name":"","type":"bool"},
                {"name":"foo_bar","type":"bool"}, {"name":"fooBar","type":"bool"}
            ]),
            json!([]),
            "payable"
        )]),
        true,
    );
    for name in ["class_", "client_", "value_", "value3", "fooBar", "fooBar2"] {
        assert!(
            source.contains(&format!("required bool {name}")),
            "missing {name}"
        );
    }
    assert!(source.contains("parameters: [class_, client_, value_, value3, fooBar, fooBar2]"));
}

#[test]
fn anonymous_events_omit_signature_topic_and_check_topic_count() {
    let source = render(
        json!([
            {"type":"event","name":"Note","anonymous":true,"inputs":[{"name":"key","type":"string","indexed":true},{"name":"count","type":"uint8","indexed":false}]},
            {"type":"event","name":"Empty","anonymous":true,"inputs":[]}
        ]),
        true,
    );
    assert!(
        source.contains(
            "if (topics.length != 1) throw FormatException('event topic count mismatch')"
        )
    );
    assert!(source.contains("key: hexToBytes(topics[0]!)"));
    assert!(source.contains("count: values[0] as BigInt"));
    assert!(
        source
            .contains("if (topics.isNotEmpty) throw FormatException('event topic count mismatch')")
    );
    assert_eq!(
        source.matches("final topics = <List<String?>>[];").count(),
        2
    );
    assert!(!source.contains("throw FormatException('event topic0 mismatch')"));
}

#[test]
fn indexed_reference_hashes_do_not_shift_following_decoded_fields() {
    let source = render(
        json!([{"type":"event","name":"Mixed","anonymous":false,"inputs":[
            {"name":"key","type":"tuple","indexed":true,"components":[{"name":"n","type":"uint256"},{"name":"flag","type":"bool"}]},
            {"name":"owner","type":"address","indexed":true},
            {"name":"label","type":"string","indexed":true},
            {"name":"count","type":"uint256","indexed":false}
        ]}]),
        true,
    );
    for expected in [
        "topics.length != 4",
        "topics[0] != mixedTopic0",
        "key: hexToBytes(topics[1]!)",
        "owner: values[0] as EthereumAddress",
        "label: hexToBytes(topics[3]!)",
        "count: values[1] as BigInt",
        "final Uint8List key;",
        "final Uint8List label;",
        "_eventTopic(mixedEvent.components[1].parameter.type, owner!)",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
}

#[test]
fn one_word_indexed_arrays_advance_the_sdk_decoding_position() {
    let source = render(
        json!([{"type":"event","name":"Mixed","anonymous":false,"inputs":[
            {"name":"key","type":"uint256[1]","indexed":true},
            {"name":"count","type":"uint256","indexed":false}
        ]}]),
        true,
    );
    assert!(source.contains("key: hexToBytes(topics[1]!)"));
    assert!(source.contains("count: values[1] as BigInt"));
}

#[test]
fn overloaded_errors_use_the_matching_abi_and_selector() {
    let source = render(
        json!([
            {"type":"error","name":"Denied","inputs":[{"name":"code","type":"uint256"}]},
            {"type":"error","name":"Denied","inputs":[{"name":"who","type":"address"}]},
            {"type":"error","name":"Empty","inputs":[]}
        ]),
        false,
    );
    for expected in [
        "SampleDeniedByUint256ErrorSelector = '0x7e46dab6'",
        "SampleDeniedByAddressErrorSelector = '0xe7d05e27'",
        "decodeDeniedByUint256Error(String data)",
        "decodeDeniedByAddressError(String data)",
        "class SampleErrorEmpty",
        "const SampleErrorEmpty();",
        "code: values[0] as BigInt",
        "who: values[0] as EthereumAddress",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
    // Decode adapters must use outputs from the corresponding error, not the other overload.
    let adapter = source
        .lines()
        .find(|line| line.contains("get _DeniedByAddressError"))
        .expect("adapter");
    let literal = adapter
        .split_once("ContractAbi.fromJson(")
        .expect("adapter")
        .1;
    let json = serde_json::Deserializer::from_str(literal)
        .into_iter::<String>()
        .next()
        .expect("literal")
        .expect("JSON string");
    let entries: Value = serde_json::from_str(&json).expect("adapter ABI");
    assert_eq!(entries[0]["type"], "function");
    assert_eq!(
        entries[0]["outputs"],
        json!([{"name":"who","type":"address"}])
    );
}

#[test]
fn overloaded_events_use_canonical_signatures_in_accessors() {
    let source = render(
        json!([
            {"type":"event","name":"Changed","inputs":[{"name":"n","type":"uint256","indexed":false}],"anonymous":false},
            {"type":"event","name":"Changed","inputs":[{"name":"owner","type":"address","indexed":false}],"anonymous":false}
        ]),
        true,
    );
    for expected in [
        "changedByUint256Event",
        "changedByAddressEvent",
        "e.stringSignature == \"Changed(uint256)\"",
        "e.stringSignature == \"Changed(address)\"",
        "class SampleChangedByUint256Event",
        "class SampleChangedByAddressEvent",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
}

#[test]
fn abi_literals_preserve_triple_quotes_and_dollar_signs_without_interpolation() {
    let entries = json!([{"type":"function","name":"read","inputs":[],"outputs":[],"stateMutability":"view","extra":"quote ''' dollar $value backslash \\ newline\n"}]);
    let source = render(entries.clone(), false);
    let expression = source
        .lines()
        .find(|line| line.starts_with("const String SampleAbiJson = "))
        .expect("ABI literal");
    let literal = expression
        .strip_prefix("const String SampleAbiJson = ")
        .expect("assignment")
        .strip_suffix(';')
        .expect("semicolon");
    let json_text: String =
        serde_json::from_str(&literal.replace("\\$", "$")).expect("escaped literal");
    assert_eq!(
        serde_json::from_str::<Value>(&json_text).expect("ABI JSON"),
        entries
    );
    assert!(literal.contains("\\$value"));
}
