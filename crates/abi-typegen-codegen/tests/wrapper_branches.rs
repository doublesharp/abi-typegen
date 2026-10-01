use abi_typegen_codegen::{fsharp, kotlin, swift};
use abi_typegen_core::{parser::parse_artifact, types::ContractIr};
use serde_json::{Value, json};

fn ir(entries: Value) -> ContractIr {
    parse_artifact("Sample", &json!({"abi":entries}).to_string()).expect("valid ABI")
}

fn function(name: &str, inputs: Value, outputs: Value, mutability: &str) -> Value {
    json!({"type":"function", "name":name, "inputs":inputs, "outputs":outputs, "stateMutability":mutability})
}

#[test]
fn swift_writes_distinguish_payable_values_and_empty_arguments() {
    let ir = ir(json!([
        function("deposit", json!([]), json!([]), "payable"),
        function(
            "set",
            json!([{"name":"x","type":"uint8"}]),
            json!([]),
            "nonpayable"
        ),
        function("ping", json!([]), json!([]), "view"),
        function(
            "read",
            json!([{"name":"x","type":"bool"}]),
            json!([{"name":"ok","type":"bool"}]),
            "pure"
        )
    ]));
    let source = swift::render_swift_file_with_wrappers(&ir, true);
    for expected in [
        "prepareDeposit(transaction: CodableTransaction? = nil, value: BigUInt = 0)",
        "guard tx.value == 0",
        "guard data.isEmpty",
        "readPing(transaction: CodableTransaction? = nil)",
        "readRead(_ args:",
        "guard args.x <",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
    assert_eq!(source.matches("nonpayable value").count(), 1);
}

#[test]
fn swift_hash_events_preserve_positions_and_anonymous_topic_counts() {
    for anonymous in [false, true] {
        let ir = ir(
            json!([{"type":"event","name":"Changed","anonymous":anonymous,"inputs":[
                {"name":"label","type":"string","indexed":true},
                {"name":"amount","type":"uint256","indexed":false},
                {"name":"owner","type":"address","indexed":true}
            ]}]),
        );
        let source = swift::render_swift_file_with_wrappers(&ir, true);
        assert!(source.contains("struct DecodedChangedEvent"));
        assert!(source.contains("public let label: Data"));
        assert!(source.contains("public let amount: BigUInt"));
        assert!(source.contains("filter0: Data? = nil"));
        assert!(source.contains("guard value.count == 32"));
        assert!(source.contains(&format!(
            "guard log.topics.count == {}",
            if anonymous { 2 } else { 3 }
        )));
        assert!(source.contains(&format!(
            "let topic0 = log.topics[{}]",
            if anonymous { 0 } else { 1 }
        )));
        assert_eq!(
            source.contains("guard log.topics.first == event.topic"),
            !anonymous
        );
    }
}

#[test]
fn swift_all_indexed_events_reject_nonempty_data_and_empty_errors_check_payloads() {
    let ir = ir(json!([
        {"type":"event","name":"Mark","anonymous":true,"inputs":[{"name":"ok","type":"bool","indexed":true}]},
        {"type":"event","name":"Empty","anonymous":false,"inputs":[]},
        {"type":"error","name":"Denied","inputs":[]},
        {"type":"error","name":"Failed","inputs":[{"name":"reason","type":"string"}]}
    ]));
    let source = swift::render_swift_file_with_wrappers(&ir, true);
    assert!(source.contains("unexpected event data"));
    assert!(source.contains("decodeCustomError("));
    assert!(source.contains("return .denied(DeniedError())"));
    assert!(source.contains("return .failed(FailedError("));
    assert!(source.contains("error.decodeEthError(Data(data.dropFirst(4))) != nil"));
    assert!(source.contains("event.inputs[0].type"));
}

#[test]
fn kotlin_functions_keep_void_single_and_multi_results_and_sdk_limits() {
    let ir = ir(json!([
        function("ping", json!([]), json!([]), "view"),
        function(
            "read",
            json!([]),
            json!([{"name":"x","type":"uint256"}]),
            "pure"
        ),
        function(
            "pair",
            json!([]),
            json!([{"name":"x","type":"bool"},{"name":"y","type":"string"}]),
            "view"
        ),
        function("deposit", json!([]), json!([]), "payable"),
        function(
            "set",
            json!([{"name":"x","type":"uint256[33]"}]),
            json!([]),
            "nonpayable"
        )
    ]));
    let source = kotlin::render_kotlin_file_with_wrappers(&ir, "app.contracts", true);
    for expected in [
        "decodePingResult(data: String): Unit",
        "decodeReadResult(data: String): BigInteger",
        "return PairResult(",
        "web3j supports static arrays with 1 to 32 elements",
        "callPing(client: Client)",
        "sendDeposit(client: Client, options: TransactionOptions)",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
    assert_eq!(
        source
            .matches("nonpayable function cannot receive value")
            .count(),
        1
    );
}

#[test]
fn kotlin_reference_hash_events_distinguish_anonymous_and_signature_topics() {
    for anonymous in [true, false] {
        let ir = ir(
            json!([{"type":"event","name":"Changed","anonymous":anonymous,"inputs":[
                {"name":"label","type":"string","indexed":true},
                {"name":"amount","type":"uint256","indexed":false},
                {"name":"owner","type":"address","indexed":true}
            ]}]),
        );
        let source = kotlin::render_kotlin_file_with_wrappers(&ir, "app.contracts", true);
        assert!(source.contains("data class DecodedChangedEvent"));
        assert!(source.contains("val label: String"));
        assert!(source.contains("indexed hash must be 32 bytes"));
        assert!(source.contains("filter.addNullTopic()"));
        assert!(source.contains("decodeIndexedValue(indexed[1]"));
        assert!(source.contains(&format!(
            "log.topics.drop({})",
            if anonymous { 0 } else { 1 }
        )));
        assert_eq!(source.contains("log.topics.firstOrNull()"), !anonymous);
    }
}

#[test]
fn kotlin_empty_events_and_errors_return_singletons_without_invalid_constructors() {
    let ir = ir(json!([
        {"type":"event","name":"Empty","anonymous":false,"inputs":[]},
        {"type":"error","name":"Denied","inputs":[]},
        {"type":"error","name":"Failed","inputs":[{"name":"reason","type":"string"}]}
    ]));
    let source = kotlin::render_kotlin_file_with_wrappers(&ir, "app.contracts", true);
    assert!(source.contains("return EmptyEvent\n"));
    assert!(!source.contains("return EmptyEvent("));
    assert!(source.contains("return DeniedError\n"));
    assert!(source.contains("return FailedError(values[0] as String)"));
}

#[test]
fn fsharp_constructor_and_write_guards_follow_payability() {
    for mutability in ["payable", "nonpayable"] {
        let ir = ir(json!([
            {"type":"constructor","inputs":[{"name":"owner","type":"address"}],"stateMutability":mutability},
            function("set", json!([{"name":"x","type":"uint256"}]), json!([]), mutability),
            function("read", json!([]), json!([{"name":"x","type":"uint256"}]), "view")
        ]));
        let source = fsharp::render_fsharp_file(&ir, true);
        assert!(source.contains("let encodeDeployment"));
        assert!(source.contains("let deploy"));
        assert!(source.contains("GetData(bytecode, abi, [| box args.Owner |])"));
        assert!(source.contains("let sendSet"));
        assert!(source.contains("let callRead"));
        assert_eq!(
            source.contains("Constructor is not payable"),
            mutability == "nonpayable"
        );
        assert_eq!(
            source.contains("Function is not payable"),
            mutability == "nonpayable"
        );
    }
}

#[test]
fn fsharp_indexed_references_use_topic_transport_and_anonymous_filters() {
    for anonymous in [true, false] {
        let ir = ir(
            json!([{"type":"event","name":"Changed","anonymous":anonymous,"inputs":[
                {"name":"label","type":"string","indexed":true},
                {"name":"amount","type":"uint256","indexed":false}
            ]}]),
        );
        let source = fsharp::render_fsharp_file(&ir, true);
        assert!(source.contains("let private convertChanged"));
        assert!(source.contains("let initial: ChangedTopicDecoder"));
        assert!(source.contains("Label = value.Label; Amount = value.Amount"));
        assert!(source.contains(&format!("EventTopicDecoder({anonymous})")));
        assert_eq!(source.contains("filter.Topics <- [||]"), anonymous);
        assert_eq!(source.contains("log.Topics.Length = 1"), anonymous);
    }
}

#[test]
fn fsharp_empty_error_payload_guards_and_metadata_modes_keep_primary_types() {
    let ir = ir(json!([
        {"type":"error","name":"Denied","inputs":[]},
        {"type":"error","name":"Failed","inputs":[{"name":"reason","type":"string"}]},
        function("read", json!([]), json!([{"name":"x","type":"uint256"}]), "view")
    ]));
    let source = fsharp::render_fsharp_file(&ir, true);
    assert!(source.contains("let decodeDeniedError"));
    assert_eq!(source.matches("Unexpected custom error payload").count(), 1);
    let metadata = fsharp::render_fsharp_file(&ir, false);
    assert!(metadata.contains("type FailedError"));
    assert!(metadata.contains("let abi"));
    assert!(!metadata.contains("let callRead"));
}
