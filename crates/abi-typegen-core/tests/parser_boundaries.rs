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
fn overload_order_uses_canonical_selectors_and_preserves_metadata() {
    let nested = json!({"type":"function","name":"route","stateMutability":"payable",
        "inputs":[
            {"name":"items","type":"tuple[][2]","internalType":"struct Routes.Item[][2]","components":[
                {"name":"amount","type":"uint"},
                {"name":"children","type":"tuple[]","components":[
                    {"name":"owner","type":"address"},{"name":"tags","type":"bytes32[2]"}
                ]}
            ]},
            {"name":"weights","type":"uint256[3][]"}
        ],"outputs":[{"name":"result","type":"int"}]});
    let simple = json!({"type":"function","name":"route","stateMutability":"view",
        "inputs":[{"name":"keys","type":"bytes32[2][]"},{"name":"weights","type":"uint256[3][]"}],
        "outputs":[{"name":"ok","type":"bool"}]});
    let middle = json!({"type":"function","name":"middle","inputs":[],"outputs":[]});
    let signature = "route((uint256,(address,bytes32[2])[])[][2],uint256[3][])";
    for pair in [
        [nested.clone(), simple.clone()],
        [simple.clone(), nested.clone()],
    ] {
        let source = json!({"abi":[
            {"type":"constructor","inputs":[{"name":"owner","type":"address"}]},
            pair[0], middle, {"type":"event","name":"Changed","inputs":[],"anonymous":true},
            pair[1], {"type":"error","name":"Denied","inputs":[]},
            {"type":"fallback","stateMutability":"payable"},{"type":"receive","stateMutability":"payable"}
        ],"metadata":{"output":{"userdoc":{"notice":"Routes", "methods":{
            (signature):{"notice":"Nested route"}
        }}}}});
        let ir = parse_artifact("Routes", &source.to_string()).unwrap();
        assert_eq!(ir.raw_abi, source["abi"]);
        assert_eq!(
            ir.functions[0].signature(),
            "route(bytes32[2][],uint256[3][])"
        );
        assert_eq!(ir.functions[0].selector().to_string(), "0x2c65d8fc");
        assert_eq!(ir.functions[1].name, "middle");
        let f = &ir.functions[2];
        assert_eq!(f.signature(), signature);
        assert_eq!(f.selector().to_string(), "0x7248a337");
        assert_eq!(f.inputs[0].name, "items");
        assert_eq!(f.inputs[1].name, "weights");
        assert_eq!(
            f.inputs[0].internal_type.as_deref(),
            Some("struct Routes.Item[][2]")
        );
        assert_eq!(f.outputs[0].ty.canonical(), "int256");
        assert_eq!(f.state_mutability, StateMutability::Payable);
        assert_eq!(
            f.natspec.as_ref().unwrap().notice.as_deref(),
            Some("Nested route")
        );
        assert_eq!(
            ir.natspec.as_ref().unwrap().notice.as_deref(),
            Some("Routes")
        );
        assert_eq!(ir.constructor.as_ref().unwrap().inputs[0].name, "owner");
        assert!(ir.events[0].anonymous);
        assert_eq!(ir.errors[0].name, "Denied");
        assert!(ir.has_receive && ir.has_fallback && ir.fallback_is_payable());
    }
}

#[test]
fn explicit_unknown_mutability_is_rejected_instead_of_becoming_nonpayable() {
    for kind in ["function", "constructor", "fallback", "receive"] {
        for mutability in ["invalid", "", "Payable", "readonly"] {
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
