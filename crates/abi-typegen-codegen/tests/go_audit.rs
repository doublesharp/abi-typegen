use abi_typegen_codegen::go::render_go_file;
use abi_typegen_core::parser::parse_artifact;

#[test]
fn indexed_reference_fields_hold_topic_hashes() {
    let ir = parse_artifact("Token", r#"{"abi":[{"type":"event","name":"Indexed","anonymous":true,"inputs":[{"name":"label","type":"string","indexed":true},{"name":"payload","type":"bytes","indexed":true},{"name":"values","type":"uint256[]","indexed":true},{"name":"pair","type":"uint256[2]","indexed":true},{"name":"tuple","type":"tuple","indexed":true,"components":[{"name":"value","type":"bool"}]},{"name":"plain","type":"string","indexed":false},{"name":"fixed","type":"bytes32","indexed":true},{"name":"flag","type":"bool","indexed":true}]}]}"#).expect("valid ABI");
    let out = render_go_file(&ir, "contracts");
    for field in ["Label", "Payload", "Values", "Pair", "Tuple"] {
        let line = out
            .lines()
            .find(|line| line.trim_start().starts_with(&format!("{field} ")))
            .expect("field exists");
        assert!(line.ends_with("common.Hash"), "{line}");
    }
    assert!(out.contains("Plain   string"), "{out}");
    assert!(out.contains("Fixed   [32]byte"), "{out}");
    assert!(out.contains("Flag    bool"), "{out}");
    assert!(
        out.contains("github.com/ethereum/go-ethereum/common"),
        "{out}"
    );
    assert!(
        !out.contains("math/big"),
        "unused indexed types must not import big: {out}"
    );
}

#[test]
fn digit_leading_exported_fields_keep_their_abi_tags() {
    let ir = parse_artifact("Token", r#"{"abi":[{"type":"function","name":"positional","stateMutability":"nonpayable","inputs":[{"name":"","type":"uint256"},{"name":"_0","type":"bool"},{"name":"x0","type":"bool"}],"outputs":[]},{"type":"event","name":"Positional","anonymous":false,"inputs":[{"name":"_0","type":"bool","indexed":true}]}]}"#).expect("valid ABI");
    let out = render_go_file(&ir, "contracts");
    assert!(out.contains("X0      bool `abi:\"_0\"`"), "{out}");
    assert!(out.contains("X02     bool `abi:\"x0\"`"), "{out}");
    assert!(out.contains("X0 bool `abi:\"_0\"`"), "{out}");
}

#[test]
fn digit_leading_contract_names_have_exported_identifiers() {
    let ir = parse_artifact("_0", r#"{"abi":[{"type":"function","name":"_1","stateMutability":"nonpayable","inputs":[{"name":"value","type":"tuple","internalType":"struct _0._2","components":[{"name":"_3","type":"bool"}]}],"outputs":[]}]}"#).expect("valid ABI");
    let out = render_go_file(&ir, "contracts");
    assert!(out.contains("const X0ABI ="), "{out}");
    assert!(out.contains("type X0X2 struct"), "{out}");
    assert!(out.contains("X3 bool `abi:\"_3\"`"), "{out}");
}
