use abi_typegen_codegen::{fsharp, java, swift};
use abi_typegen_core::parser::parse_artifact;
use serde_json::json;

#[test]
fn java_record_components_do_not_override_object_methods() {
    let names = [
        "clone",
        "finalize",
        "getClass",
        "hashCode",
        "notify",
        "notifyAll",
        "toString",
        "wait",
        "hashCode_",
    ];
    let inputs = names
        .iter()
        .map(|name| json!({"name":name,"type":"bool"}))
        .collect::<Vec<_>>();
    let ir = parse_artifact(
        "Records",
        &json!({"abi":[{"type":"error","name":"Denied","inputs":inputs}]}).to_string(),
    )
    .expect("valid ABI");
    for wrappers in [false, true] {
        let source = java::render_java_file(&ir, "contracts", wrappers);
        assert!(source.contains("record DeniedError(Boolean clone_, Boolean finalize_, Boolean getClass_, Boolean hashCode_, Boolean notify_, Boolean notifyAll_, Boolean toString_, Boolean wait_, Boolean hashCode_2)"), "{source}");
    }
}

#[test]
fn java_embedded_abi_preserves_unicode_control_characters_in_metadata() {
    let entries = json!([{"type":"function","name":"read","inputs":[],"outputs":[],"stateMutability":"view","x-note":"delete\u{7f}next\u{85}line"}]);
    let ir = parse_artifact("Metadata", &json!({"abi":entries}).to_string()).expect("valid ABI");
    let source = java::render_java_file(&ir, "contracts", false);
    let prefix = "ABI = String.join(\"\", ";
    let start = source.find(prefix).expect("embedded ABI") + prefix.len();
    let end = source[start..].find(");").expect("ABI terminator") + start;
    assert!(source[start..end].contains("\\u007f"));
    assert!(source[start..end].contains("\\u0085"));
    let chunks: Vec<String> =
        serde_json::from_str(&format!("[{}]", &source[start..end])).expect("escaped Java strings");
    let decoded: serde_json::Value = serde_json::from_str(&chunks.concat()).expect("embedded JSON");
    assert_eq!(decoded, entries);
}

#[test]
fn swift_wrapper_names_and_sdk_types_do_not_collide_with_contracts_or_tuples() {
    for name in [
        "Web3",
        "EthereumContract",
        "ABI",
        "ABIDecoder",
        "CodableTransaction",
        "EventLog",
        "EventFilterParameters",
        "ReadOperation",
        "WriteOperation",
        "Error",
        "Client",
        "WrapperError",
        "DecodedCustomError",
    ] {
        let inputs = json!([{"name":"entry","type":"tuple","internalType":format!("struct {name}"),"components":[{"name":"ok","type":"bool"}]}]);
        let ir = parse_artifact(name, &json!({"abi":[{"type":"function","name":"echo","inputs":inputs,"outputs":[],"stateMutability":"view"}]}).to_string()).expect("valid ABI");
        let source = swift::render_swift_file_with_wrappers(&ir, true);
        assert!(
            source.contains(&format!("public enum {name}2 {{")),
            "{source}"
        );
        assert!(
            source.contains(&format!("public struct {name}3: Sendable, Hashable")),
            "{source}"
        );
    }
}

#[test]
fn fsharp_normalizes_dollar_and_underscore_names_and_preserves_indexed_scalars() {
    for (name, module) in [
        ("$", "Contract"),
        ("_", "Contract"),
        ("_123", "Contract123"),
        ("$token", "Token"),
    ] {
        let ir = parse_artifact(name, &json!({"abi":[{"type":"event","name":"Mixed","anonymous":false,"inputs":[{"name":"$","type":"bytes4","indexed":true},{"name":"_123","type":"uint8","indexed":true},{"name":"_","type":"string","indexed":true},{"name":"amount","type":"uint256","indexed":false}]}]}).to_string()).expect("valid ABI");
        let source = fsharp::render_fsharp_file(&ir, true);
        assert!(source.contains(&format!("module Contracts.{module}")));
        assert!(source.contains("Contract: byte array"));
        assert!(source.contains("Contract123: BigInteger"));
        assert!(source.contains("Contract2: string"));
        assert!(source.contains("type private MixedTopicDecoder"));
        assert!(source.contains("Contract123 = value.Contract123"));
    }
}
