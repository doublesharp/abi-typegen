use abi_typegen_codegen::{java, kotlin, php, swift};
use abi_typegen_core::parser::parse_artifact;
use abi_typegen_core::types::{ContractIr, NatSpec};
use serde_json::{Value, json};

fn ir(entries: Value) -> ContractIr {
    parse_artifact("Sample", &json!({"abi": entries}).to_string()).expect("valid ABI")
}

fn function(name: &str, inputs: Value, outputs: Value) -> Value {
    json!({"type":"function","name":name,"stateMutability":"view","inputs":inputs,"outputs":outputs})
}

#[test]
fn java_dynamic_arrays_of_supported_static_arrays_use_concrete_sdk_element_classes() {
    let params = json!([{"name":"rows","type":"uint256[2][]"}]);
    let source = java::render_java_file(
        &ir(json!([function("echo", params.clone(), params)])),
        "contracts",
        true,
    );
    for expected in [
        "java.util.List<java.util.List<java.math.BigInteger>> rows",
        "org.web3j.abi.datatypes.generated.StaticArray2 .class",
        "new org.web3j.abi.datatypes.generated.StaticArray2(org.web3j.abi.datatypes.generated.Uint256.class",
        "org.web3j.abi.datatypes.DynamicArray<org.web3j.abi.datatypes.generated.StaticArray2<org.web3j.abi.datatypes.generated.Uint256>>",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
}

#[test]
fn java_tuple_fields_keep_unsupported_static_array_metadata_without_sdk_array33_classes() {
    let tuple = json!({"name":"items","type":"tuple","internalType":"struct Sample.Items","components":[{"name":"values","type":"uint256[33]"}]});
    for wrappers in [false, true] {
        let source = java::render_java_file(
            &ir(json!([
                function(
                    "echo",
                    json!([tuple.clone()]),
                    json!([{"name":"values","type":"uint256[33]"}])
                ),
                function(
                    "read",
                    json!([]),
                    json!([{"name":"values","type":"uint256[33]"}])
                )
            ])),
            "contracts",
            wrappers,
        );
        if wrappers {
            assert!(source.contains(
                "org.web3j.abi.datatypes.StaticArray<org.web3j.abi.datatypes.generated.Uint256>"
            ));
        }
        assert!(source.contains("unsupportedStaticArray()"));
        assert!(!source.contains("generated.StaticArray33"));
        assert!(source.contains("public final java.util.List<java.math.BigInteger> values;"));
    }
}

#[test]
fn underscore_fields_are_escaped_in_java_and_php_error_records() {
    let ir = ir(json!([{"type":"error","name":"Denied","inputs":[{"name":"_","type":"bool"}]}]));
    let java = java::render_java_file(&ir, "contracts", false);
    assert!(java.contains("record DeniedError(Boolean __)"));
    let php = php::render_php_file(&ir, "Contracts", false);
    assert!(php.contains("public bool $__"));
}

#[test]
fn kotlin_arrays_of_sdk_values_preserve_elements_in_both_tuple_constructors() {
    let only_bytes = json!({"name":"blob","type":"tuple","internalType":"struct Sample.Blob","components":[{"name":"payload","type":"bytes"},{"name":"key","type":"bytes4"}]});
    let collection = json!({"name":"collection","type":"tuple","internalType":"struct Sample.Collection","components":[
        {"name":"payloads","type":"bytes[]"},{"name":"keys","type":"bytes4[2]"},
        {"name":"blobs","type":"tuple[]","internalType":"struct Sample.Blob[]","components":only_bytes["components"]}
    ]});
    let source = kotlin::render_kotlin_file_with_wrappers(
        &ir(json!([function(
            "echo",
            json!([only_bytes, collection.clone()]),
            json!([collection])
        )])),
        "contracts",
        true,
    );
    for expected in [
        "data class Blob(",
        "val payload: DynamicBytes",
        "val key: Bytes4",
        "DynamicArray(DynamicBytes::class.java, payloads)",
        "StaticArray2(Bytes4::class.java, keys)",
        "DynamicArray(Blob::class.java, blobs)",
        "payloads.value",
        "keys.value",
        "blobs.value",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
    assert!(!source.contains("payloads.value.map"));
    assert!(!source.contains("blobs.value.map"));
}

#[test]
fn kotlin_signed_integer_arrays_and_zero_field_events_keep_callable_wrappers() {
    let ir = ir(json!([
        function("echo", json!([{"name":"values","type":"int8[2]"}]), json!([{"name":"values","type":"int8[2]"}])),
        {"type":"event","name":"Empty","anonymous":false,"inputs":[]},
        {"type":"event","name":"Anonymous","anonymous":true,"inputs":[]}
    ]));
    let source = kotlin::render_kotlin_file_with_wrappers(&ir, "contracts", true);
    for expected in [
        "import org.web3j.abi.datatypes.generated.Int8",
        "StaticArray2(Int8::class.java, values.map { Int8(it) })",
        "data object EmptyEvent",
        "data object AnonymousEvent",
        "return EmptyEvent",
        "return AnonymousEvent",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
    assert!(!source.contains("throw UnsupportedOperationException(\"web3j supports static arrays"));
    for name in ["Uint", "Int", "Bytes", "StaticArray"] {
        let name = kotlin::namespace_name(name);
        assert!(!name.ends_with("Contract"));
    }
}

#[test]
fn php_mixed_indexed_reference_event_records_preserve_scalar_and_nonindexed_types() {
    let source = php::render_php_file(
        &ir(
            json!([{"type":"event","name":"Mixed","anonymous":false,"inputs":[
                {"name":"label","type":"string","indexed":true},
                {"name":"owner","type":"address","indexed":true},
                {"name":"amount","type":"uint256","indexed":false}
            ]}]),
        ),
        "Contracts",
        true,
    );
    for expected in [
        "class SampleDecodedMixedEvent",
        "public string $label",
        "public string $owner",
        "public \\Brick\\Math\\BigInteger $amount",
        "function decodeMixedEvent",
        "function filterMixedEvent",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
}

#[test]
fn swift_validation_checks_fixed_lengths_and_each_dynamic_array_element() {
    let scalar_tuple = json!({"name":"settings","type":"tuple","internalType":"struct Sample.Settings","components":[{"name":"ok","type":"bool"},{"name":"text","type":"string"},{"name":"owner","type":"address"}]});
    let ir = ir(json!([
        function("echo", json!([scalar_tuple, {"name":"flags","type":"bool[2]"},{"name":"keys","type":"bytes4[]"}]), json!([])),
        {"type":"event","name":"Anonymous","anonymous":true,"inputs":[]}
    ]));
    let source = swift::render_swift_file_with_wrappers(&ir, true);
    for expected in [
        "private static func _validateSettings",
        "        _ = value\n",
        "guard args.flags.count == 2",
        "for item in args.keys",
        "guard item.count == 4",
        "public static func filterAnonymous()",
        "        return []\n",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
    assert!(!source.contains("for item in flags"));
}

#[test]
fn kotlin_and_swift_natspec_preserve_paragraph_breaks_without_ending_doc_comments() {
    let mut ir = ir(json!([function("read", json!([]), json!([]))]));
    let docs = NatSpec {
        notice: Some("Read values\n\nKeep */ inside docs".into()),
        ..NatSpec::default()
    };
    ir.natspec = Some(docs.clone());
    ir.functions[0].natspec = Some(docs);
    let kotlin = kotlin::render_kotlin_file(&ir, "contracts");
    assert!(kotlin.contains(" *\n"));
    assert!(kotlin.contains("Keep *&#47; inside docs"));
    let swift = swift::render_swift_file(&ir);
    assert!(swift.contains("/// Read values\n///\n/// Keep */ inside docs"));
}
