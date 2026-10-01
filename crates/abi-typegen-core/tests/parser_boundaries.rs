use abi_typegen_core::{parser::parse_artifact, types::StateMutability};
use serde_json::{Value, json};

#[test]
fn special_entry_mutability_is_validated_at_the_parse_boundary() {
    for (kind, mutability) in [
        ("constructor", "view"),
        ("constructor", "pure"),
        ("fallback", "view"),
        ("fallback", "pure"),
        ("receive", "view"),
        ("receive", "pure"),
        ("receive", "nonpayable"),
    ] {
        let error = parse_artifact(
            "Token",
            &artifact(json!([{"type":kind,"inputs":[],"stateMutability":mutability}])),
        )
        .expect_err("invalid special entry mutability");
        assert!(error.to_string().contains(kind), "{error}");
    }
    parse_artifact("Token", &artifact(json!([{"type":"receive"}])))
        .expect("omitted receive mutability implies payable");
}

fn artifact(abi: Value) -> String {
    json!({"abi":abi}).to_string()
}

#[test]
fn explicit_unknown_mutability_is_rejected_instead_of_becoming_nonpayable() {
    for kind in ["function", "constructor", "fallback", "receive"] {
        for mutability in ["veiw", "", "Payable", "readonly"] {
            let abi = json!([{"type":kind,"name":"read","inputs":[],"outputs":[],"stateMutability":mutability}]);
            let error = parse_artifact("Token", &artifact(abi)).expect_err("unknown mutability");
            assert!(error.to_string().contains("mutability"), "{error}");
        }
    }
}

#[test]
fn omitted_mutability_retains_legacy_nonpayable_default() {
    let ir = parse_artifact(
        "Token",
        &artifact(json!([
            {"type":"function","name":"read","inputs":[],"outputs":[]},
            {"type":"constructor","inputs":[]}
        ])),
    )
    .expect("legacy ABI");
    assert_eq!(
        ir.functions[0].state_mutability,
        StateMutability::NonPayable
    );
    assert_eq!(
        ir.constructor.expect("constructor").state_mutability,
        StateMutability::NonPayable
    );
}

#[test]
fn natspec_lookup_canonicalizes_integer_aliases_inside_tuple_arrays() {
    let artifact = json!({"abi":[
        {"type":"function","name":"read","inputs":[{"name":"items","type":"tuple[]","components":[{"name":"n","type":"uint"},{"name":"signed","type":"int[2]"}]}],"outputs":[],"stateMutability":"view"},
        {"type":"event","name":"Changed","inputs":[{"name":"n","type":"uint","indexed":false}],"anonymous":false},
        {"type":"error","name":"Denied","inputs":[{"name":"n","type":"int"}]}
    ],"metadata":{"output":{"userdoc":{
        "methods":{"read((uint256,int256[2])[])":{"notice":"Reads aliases"}},
        "events":{"Changed(uint256)":{"notice":"Changed aliases"}},
        "errors":{"Denied(int256)":{"notice":"Denied aliases"}}
    }}}});
    let ir = parse_artifact("Token", &artifact.to_string()).expect("ABI");
    assert_eq!(
        ir.functions[0]
            .natspec
            .as_ref()
            .and_then(|ns| ns.notice.as_deref()),
        Some("Reads aliases")
    );
    assert_eq!(
        ir.events[0]
            .natspec
            .as_ref()
            .and_then(|ns| ns.notice.as_deref()),
        Some("Changed aliases")
    );
    assert_eq!(
        ir.errors[0]
            .natspec
            .as_ref()
            .and_then(|ns| ns.notice.as_deref()),
        Some("Denied aliases")
    );
}
