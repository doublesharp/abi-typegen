use abi_typegen_codegen::{generate_contract_files, java};
use abi_typegen_config::{Config, Target};
use abi_typegen_core::parser::parse_artifact;
use serde_json::{Value, json};

fn function(name: &str, inputs: Value, outputs: Value, mutability: &str) -> Value {
    json!({"type":"function", "name":name, "inputs":inputs, "outputs":outputs, "stateMutability":mutability})
}

fn render(entries: Value, wrappers: bool) -> String {
    let ir = parse_artifact("Sample", &json!({"abi":entries}).to_string()).expect("valid ABI");
    java::render_java_file(&ir, "com.example.contracts", wrappers)
}

#[test]
fn dispatch_uses_the_public_class_name_and_configured_package() {
    let ir = parse_artifact("String", r#"{"abi":[]}"#).expect("valid ABI");
    for wrappers in [false, true] {
        let config = Config {
            targets: vec![Target::Java],
            wrappers,
            package: "com.example.contracts".into(),
            ..Config::from_toml_str("").expect("config")
        };
        let files = generate_contract_files(&ir, &config);
        assert_eq!(files.len(), 1);
        assert!(files["StringContract.java"].contains("package com.example.contracts;"));
        assert!(files["StringContract.java"].contains("public final class StringContract"));
    }
}

#[test]
fn namespaces_do_not_shadow_java_or_wrapper_types() {
    for name in [
        "String",
        "Boolean",
        "Object",
        "Class",
        "Math",
        "Integer",
        "Client",
        "TransactionOptions",
        "Layout",
        "Atom",
        "ArrayLayout",
        "TupleLayout",
    ] {
        assert_eq!(java::namespace_name(name), format!("{name}Contract"));
    }
    for (name, expected) in [
        ("", "Contract"),
        ("_", "Contract"),
        ("foo_bar", "FooBar"),
        ("_123", "X123"),
        ("tokenURI", "TokenURI"),
    ] {
        assert_eq!(java::namespace_name(name), expected);
    }
}

#[test]
fn metadata_mode_keeps_signatures_records_and_tuple_types_without_rpc() {
    let source = render(
        json!([
            function("transfer", json!([{"name":"to","type":"address"},{"name":"amount","type":"uint256"}]), json!([{"name":"ok","type":"bool"}]), "nonpayable"),
            {"type":"event","name":"Changed","inputs":[{"name":"amount","type":"uint256","indexed":false}],"anonymous":false},
            {"type":"error","name":"Denied","inputs":[{"name":"amount","type":"uint256"}]}
        ]),
        false,
    );
    for expected in [
        "public static final String ABI",
        "TRANSFER_SIGNATURE = \"transfer(address,uint256)\"",
        "TRANSFER_SELECTOR = \"0xa9059cbb\"",
        "record ChangedEvent(java.math.BigInteger amount)",
        "record DeniedError(java.math.BigInteger amount)",
        "DENIED_ERROR_SELECTOR = \"0x7e46dab6\"",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
    for absent in [
        "class Client",
        "encodeTransfer(",
        "decodeChangedEvent(",
        "decodeDeniedError(",
        "private interface Layout",
    ] {
        assert!(!source.contains(absent), "unexpected {absent}");
    }
}

#[test]
fn scalar_parameters_and_outputs_use_web3j_types_without_integer_loss() {
    let params = json!([
        {"name":"flag","type":"bool"}, {"name":"owner","type":"address"},
        {"name":"text","type":"string"}, {"name":"amount","type":"uint256"},
        {"name":"delta","type":"int8"}, {"name":"blob","type":"bytes"},
        {"name":"key","type":"bytes3"}
    ]);
    let source = render(
        json!([function("echo", params.clone(), params, "pure")]),
        true,
    );
    for expected in [
        "Boolean flag",
        "String owner",
        "String text",
        "java.math.BigInteger amount",
        "java.math.BigInteger delta",
        "org.web3j.abi.datatypes.DynamicBytes blob",
        "org.web3j.abi.datatypes.generated.Bytes3 key",
        "new org.web3j.abi.datatypes.Bool(flag)",
        "new org.web3j.abi.datatypes.Address(owner)",
        "new org.web3j.abi.datatypes.Utf8String(text)",
        "new org.web3j.abi.datatypes.generated.Uint256(amount)",
        "new org.web3j.abi.datatypes.generated.Int8(delta)",
        "record EchoResult(",
        "return new EchoResult(",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
    for sdk in [
        "Bool",
        "Address",
        "Utf8String",
        "generated.Uint256",
        "generated.Int8",
        "DynamicBytes",
        "generated.Bytes3",
    ] {
        assert!(
            source.contains(&format!(
                "new org.web3j.abi.TypeReference<org.web3j.abi.datatypes.{sdk}>() {{}}"
            )),
            "missing {sdk} output reference"
        );
    }
}

#[test]
fn nested_arrays_use_distinct_mapping_variables_and_layouts() {
    let params = json!([{ "name":"grid", "type":"uint256[][2]" }]);
    let source = render(
        json!([function("echo", params.clone(), params, "view")]),
        true,
    );
    for expected in [
        "java.util.List<java.util.List<java.math.BigInteger>> grid",
        "new org.web3j.abi.datatypes.generated.StaticArray2(org.web3j.abi.datatypes.DynamicArray.class",
        "grid.stream().map(item0 -> new org.web3j.abi.datatypes.DynamicArray",
        "item0.stream().map(item1 -> new org.web3j.abi.datatypes.generated.Uint256(item1))",
        "new ArrayLayout(new ArrayLayout(new Atom(false,",
        "}), null), 2)",
        "callEcho(Client client,",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
}

#[test]
fn static_and_dynamic_tuples_use_distinct_sdk_structs() {
    let static_tuple = json!({"name":"point","type":"tuple","internalType":"struct Sample.Point","components":[{"name":"x","type":"int256"},{"name":"ok","type":"bool"}]});
    let dynamic_tuple = json!({"name":"entry","type":"tuple","internalType":"struct Sample.Entry","components":[{"name":"label","type":"string"},{"name":"points","type":"tuple[]","internalType":"struct Sample.Point[]","components":[{"name":"x","type":"int256"},{"name":"ok","type":"bool"}]}]});
    let source = render(
        json!([function(
            "echo",
            json!([static_tuple, dynamic_tuple.clone()]),
            json!([dynamic_tuple]),
            "pure"
        )]),
        true,
    );
    for expected in [
        "class Point extends org.web3j.abi.datatypes.StaticStruct",
        "class Entry extends org.web3j.abi.datatypes.DynamicStruct",
        "public final java.util.List<Point> points;",
        "public static Entry decodeEntryValue(String data)",
        "private static Layout layoutPoint()",
        "private static Layout layoutEntry()",
        "new ArrayLayout(layoutPoint(), null)",
        "new org.web3j.abi.datatypes.DynamicArray(Point .class, points.stream().map(item0 -> item0).toList())",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
}

#[test]
fn fixed_arrays_of_dynamic_elements_make_tuples_dynamic() {
    let tuple = json!({"name":"entry","type":"tuple","internalType":"struct Sample.Entry","components":[{"name":"labels","type":"string[2]"}]});
    let source = render(
        json!([function(
            "echo",
            json!([tuple.clone()]),
            json!([tuple]),
            "pure"
        )]),
        true,
    );
    assert!(source.contains("class Entry extends org.web3j.abi.datatypes.DynamicStruct"));
    assert!(source.contains("new ArrayLayout(new Atom(true,"));
    assert!(
        source
            .contains("org.web3j.abi.datatypes.Utf8String.class); return value.getValue(); }), 2)")
    );
}

#[test]
fn unsupported_static_array_sizes_fail_before_constructing_sdk_functions() {
    for (ty, components) in [
        ("uint256[33]", None),
        ("uint256[33][]", None),
        (
            "tuple",
            Some(json!([{"name":"values","type":"uint256[33]"}])),
        ),
    ] {
        let mut param = json!({"name":"items","type":ty});
        if let Some(components) = components {
            param["components"] = components;
        }
        let source = render(
            json!([function("store", json!([param]), json!([]), "nonpayable")]),
            true,
        );
        let method = source
            .lines()
            .find(|line| line.contains("functionStore("))
            .expect("function adapter");
        assert!(method.contains("throw new UnsupportedOperationException(\"web3j supports static arrays with 1 to 32 elements\")"));
        assert!(!method.contains("return new org.web3j.abi.datatypes.Function("));
    }
}

#[test]
fn sdk_static_array_limit_keeps_supported_sizes_callable() {
    for size in [1, 2, 32] {
        let source = render(
            json!([function(
                "store",
                json!([{"name":"items","type":format!("uint8[{size}]")}]),
                json!([]),
                "nonpayable"
            )]),
            true,
        );
        assert!(source.contains(&format!("new org.web3j.abi.datatypes.generated.StaticArray{size}(org.web3j.abi.datatypes.generated.Uint8.class")));
        let method = source
            .lines()
            .find(|line| line.contains("functionStore("))
            .expect("function adapter");
        assert!(method.contains("return new org.web3j.abi.datatypes.Function("));
    }
}

#[test]
fn mutability_selects_read_write_and_nonpayable_value_checks() {
    let source = render(
        json!([
            function(
                "read",
                json!([]),
                json!([{"name":"n","type":"uint256"}]),
                "view"
            ),
            function("noop", json!([]), json!([]), "pure"),
            function("write", json!([]), json!([]), "nonpayable"),
            function("deposit", json!([]), json!([]), "payable")
        ]),
        true,
    );
    assert!(source.contains("java.math.BigInteger callRead(Client client)"));
    assert!(source.contains("void callNoop(Client client)"));
    assert!(source.contains("if (!data.isEmpty() && !data.equals(\"0x\"))"));
    let write = source
        .lines()
        .find(|line| line.contains("sendWrite("))
        .expect("write");
    assert!(write.contains("nonpayable function cannot receive value"));
    let payable = source
        .lines()
        .find(|line| line.contains("sendDeposit("))
        .expect("payable");
    assert!(!payable.contains("nonpayable"));
    assert!(payable.contains("client.send(encodeDeposit(), options)"));
    assert!(source.contains("transaction value must be nonnegative"));
}

#[test]
fn overloaded_functions_events_and_errors_keep_distinct_constants_and_helpers() {
    let source = render(
        json!([
            function("lookup", json!([{"name":"n","type":"uint256"}]), json!([]), "view"),
            function("lookup", json!([{"name":"owner","type":"address"}]), json!([]), "view"),
            {"type":"event","name":"Changed","inputs":[],"anonymous":false},
            {"type":"event","name":"Changed","inputs":[{"name":"n","type":"uint256","indexed":false}],"anonymous":false},
            {"type":"error","name":"Denied","inputs":[{"name":"n","type":"uint256"}]},
            {"type":"error","name":"Denied","inputs":[{"name":"owner","type":"address"}]}
        ]),
        true,
    );
    for expected in [
        "LOOKUP_0_SIGNATURE = \"lookup(uint256)\"",
        "LOOKUP_1_SIGNATURE = \"lookup(address)\"",
        "encodeLookup0(",
        "encodeLookup1(",
        "decodeChanged0Event(",
        "decodeChanged1Event(",
        "decodeDenied0Error(",
        "decodeDenied1Error(",
        "DENIED_0_ERROR_SELECTOR = \"0x7e46dab6\"",
        "DENIED_1_ERROR_SELECTOR = \"0xe7d05e27\"",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
}

#[test]
fn keyword_and_duplicate_fields_are_legal_unique_record_parameters() {
    let source = render(
        json!([{ "type":"error", "name":"Denied", "inputs":[
            {"name":"class","type":"bool"}, {"name":"value","type":"uint256"},
            {"name":"","type":"uint256"}, {"name":"","type":"uint256"},
            {"name":"x","type":"bool"}, {"name":"x","type":"bool"}
        ]}]),
        false,
    );
    assert!(source.contains("record DeniedError(Boolean class_, java.math.BigInteger value_, java.math.BigInteger uint256, java.math.BigInteger uint256_2, Boolean x, Boolean x2)"));
}

#[test]
fn anonymous_event_filters_start_with_indexed_fields_and_omit_topic_zero() {
    let source = render(
        json!([{ "type":"event", "name":"Note", "anonymous":true, "inputs":[
            {"name":"key","type":"string","indexed":true}, {"name":"n","type":"uint256","indexed":false}
        ]}]),
        true,
    );
    assert!(!source.contains("NOTE_EVENT_TOPIC"));
    assert!(!source.contains("log.getTopics().get(0).equalsIgnoreCase"));
    assert!(source.contains("log.getTopics().size() != 1"));
    assert!(source.contains(
        "new DecodedNoteEvent(log.getTopics().get(0), (java.math.BigInteger) values.get(0))"
    ));
    assert!(
        source.contains("if (key == null) filter.addNullTopic(); else filter.addSingleTopic(key)")
    );
}

#[test]
fn indexed_reference_hashes_and_typed_fields_keep_declaration_order() {
    let source = render(
        json!([{ "type":"event", "name":"Mixed", "anonymous":false, "inputs":[
            {"name":"amount","type":"uint256","indexed":false}, {"name":"owner","type":"address","indexed":true},
            {"name":"key","type":"bytes4","indexed":true}, {"name":"tags","type":"uint256[]","indexed":true}
        ]}]),
        true,
    );
    assert!(source.contains("record DecodedMixedEvent(java.math.BigInteger amount, String owner, org.web3j.abi.datatypes.generated.Bytes4 key, String tags)"));
    assert!(source.contains("log.getTopics().size() != 4"));
    assert!(source.contains("decodeIndexedValue(log.getTopics().get(1), new org.web3j.abi.TypeReference<org.web3j.abi.datatypes.Address>(true)"));
    assert!(source.contains("decodeIndexedValue(log.getTopics().get(2), new org.web3j.abi.TypeReference<org.web3j.abi.datatypes.generated.Bytes4>(true)"));
    assert!(source.contains("log.getTopics().get(3)"));
    assert!(source.contains("TypeEncoder.encode(new org.web3j.abi.datatypes.Address(owner))"));
    assert!(source.contains("TypeEncoder.encode(key)"));
    assert!(source.contains("filter.addSingleTopic(tags)"));
}

#[test]
fn empty_errors_and_events_produce_zero_field_records_and_decoders() {
    let source = render(
        json!([
            {"type":"error","name":"Denied","inputs":[]},
            {"type":"event","name":"Changed","inputs":[],"anonymous":false}
        ]),
        true,
    );
    assert!(source.contains("record DeniedError()"));
    assert!(source.contains("return new DeniedError();"));
    assert!(source.contains("record ChangedEvent()"));
    assert!(source.contains("return new ChangedEvent();"));
    assert!(source.contains("log.getTopics().size() != 1"));
    assert!(source.contains("if (!data.regionMatches(true, 0,"));
}

#[test]
fn long_embedded_abi_is_split_into_lossless_java_string_chunks() {
    let label = format!("line\n\t\"quoted\"\\{}", "a".repeat(35_000));
    let entries = json!([function(
        "echo",
        json!([{"name":label,"type":"string"}]),
        json!([]),
        "view"
    )]);
    let source = render(entries.clone(), false);
    let start = source.find("ABI = String.join(\"\", ").expect("ABI start")
        + "ABI = String.join(\"\", ".len();
    let end = source[start..].find(");").expect("ABI end") + start;
    let chunks: Vec<String> =
        serde_json::from_str(&format!("[{}]", &source[start..end])).expect("escaped literals");
    assert!(chunks.len() >= 3);
    assert!(chunks.iter().all(|chunk| chunk.chars().count() <= 16_000));
    let decoded: Value = serde_json::from_str(&chunks.concat()).expect("embedded ABI");
    assert_eq!(decoded, entries);
}
