use abi_typegen_codegen::{rust, solidity, tuples::TupleRegistry};
use abi_typegen_core::parser::parse_artifact;
use serde_json::json;

#[test]
fn payable_fallback_retains_mutability_in_rust_and_solidity_interfaces() {
    for payable in [false, true] {
        let state = if payable { "payable" } else { "nonpayable" };
        let ir = parse_artifact(
            "Fallback",
            &json!({"abi":[{"type":"fallback","stateMutability":state}]}).to_string(),
        )
        .expect("fallback ABI");
        let declaration = if payable {
            "fallback() external payable;"
        } else {
            "fallback() external;"
        };
        assert!(rust::render_rust_file(&ir, false).contains(declaration));
        assert!(solidity::render_solidity_file(&ir).contains(declaration));
    }
}

#[test]
fn nested_tuples_that_claim_the_outer_name_keep_both_definitions() {
    let ir = parse_artifact("Sample", &json!({"abi":[{"type":"function","name":"read","inputs":[
        {"name":"item","type":"tuple","internalType":"struct A.BC","components":[
            {"name":"child","type":"tuple","internalType":"struct AB.C","components":[{"name":"n","type":"uint256"}]}
        ]}
    ],"outputs":[],"stateMutability":"view"}]}).to_string()).expect("ABI");
    let registry = TupleRegistry::new(&ir);
    assert_eq!(registry.defs().len(), 2);
    assert_eq!(registry.defs()[0].name, "ABC");
    assert_eq!(registry.defs()[1].name, "ABC2");
    assert_ne!(registry.defs()[0].components, registry.defs()[1].components);
    let source = rust::render_rust_file(&ir, false);
    assert!(source.contains("struct ABC {"));
    assert!(source.contains("struct ABC2 {"));
}
