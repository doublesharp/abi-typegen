use abi_typegen_codegen::kotlin::render_kotlin_file;
use abi_typegen_core::parser::parse_artifact;

#[test]
fn contract_namespace_does_not_shadow_sdk_types() {
    for name in [
        "String",
        "Boolean",
        "List",
        "Address",
        "BigInteger",
        "JvmField",
        "Uint256",
        "StaticArray2",
    ] {
        let ir = parse_artifact(name, r#"{"abi":[{"type":"function","name":"set","inputs":[{"name":"value","type":"uint256"},{"name":"flag","type":"bool"},{"name":"text","type":"string"}],"outputs":[],"stateMutability":"nonpayable"}]}"#).expect("valid ABI");
        let out = render_kotlin_file(&ir, "contracts");
        let suffix = if matches!(name, "Uint256" | "StaticArray2") {
            "Contract"
        } else {
            "2"
        };
        assert!(out.contains(&format!("object {name}{suffix} {{")), "{out}");
    }
}

#[test]
fn tuple_name_does_not_shadow_jvm_field_annotation() {
    let ir = parse_artifact("Token", r#"{"abi":[{"type":"function","name":"set","inputs":[{"name":"value","type":"tuple","internalType":"struct Token.JvmField","components":[{"name":"x","type":"uint256"}]}],"outputs":[],"stateMutability":"nonpayable"}]}"#).expect("valid ABI");
    let out = render_kotlin_file(&ir, "contracts");
    assert!(out.contains("data class JvmField2("), "{out}");
}

#[test]
fn underscore_only_names_are_escaped() {
    let ir = parse_artifact("Token", r#"{"abi":[{"type":"function","name":"set","inputs":[{"name":"p","type":"tuple","components":[{"name":"_","type":"bool"},{"name":"__","type":"bool"}]}],"outputs":[],"stateMutability":"nonpayable"}]}"#).expect("valid ABI");
    let out = render_kotlin_file(&ir, "contracts");
    assert!(out.contains("val `_`: Boolean,"), "{out}");
    assert!(out.contains("val `__`: Boolean,"), "{out}");
    assert!(out.contains("Bool(`_`), Bool(`__`)"), "{out}");
}
