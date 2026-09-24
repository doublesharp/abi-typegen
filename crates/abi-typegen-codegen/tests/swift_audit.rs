use abi_typegen_codegen::swift::render_swift_file;
use abi_typegen_core::parser::parse_artifact;

#[test]
fn contract_namespace_does_not_shadow_sdk_types() {
    for name in [
        "Data",
        "String",
        "Bool",
        "Sendable",
        "Hashable",
        "BigInt",
        "BigUInt",
        "Foundation",
        "Swift",
        "Web3Core",
    ] {
        let ir = parse_artifact(name, r#"{"abi":[{"type":"function","name":"set","inputs":[{"name":"value","type":"bytes"},{"name":"flag","type":"bool"},{"name":"text","type":"string"}],"outputs":[],"stateMutability":"nonpayable"}]}"#).expect("valid ABI");
        let out = render_swift_file(&ir);
        assert!(out.contains(&format!("public enum {name}2 {{")), "{out}");
    }
}

#[test]
fn underscore_parameter_has_usable_stored_property() {
    let ir = parse_artifact("Token", r#"{"abi":[{"type":"function","name":"set","inputs":[{"name":"_","type":"bool"},{"name":"_2","type":"bool"}],"outputs":[],"stateMutability":"nonpayable"}]}"#).expect("valid ABI");
    let out = render_swift_file(&ir);
    assert!(!out.contains("public let _:"), "{out}");
    assert!(out.contains("self._2 = _2"), "{out}");
    assert!(out.contains("self._22 = _22"), "{out}");
}

#[test]
#[ignore = "requires the Swift compiler"]
fn generated_swift_collision_cases_compile() {
    let dir = std::env::temp_dir().join(format!("abi-typegen-swift-audit-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create scratch directory");
    let mut paths = Vec::new();
    for name in [
        "Data",
        "String",
        "Bool",
        "Sendable",
        "Hashable",
        "BigInt",
        "BigUInt",
        "Foundation",
        "Swift",
        "Web3Core",
    ] {
        let ir = parse_artifact(name, r#"{"abi":[{"type":"function","name":"set","inputs":[{"name":"value","type":"bytes"},{"name":"_","type":"bool"},{"name":"_2","type":"bool"},{"name":"text","type":"string"},{"name":"self","type":"bool"},{"name":"self2","type":"bool"},{"name":"self_","type":"bool"}],"outputs":[],"stateMutability":"nonpayable"}]}"#).expect("valid ABI");
        let path = dir.join(format!("{name}.swift"));
        std::fs::write(&path, render_swift_file(&ir)).expect("write generated source");
        paths.push(path);
    }
    let output = std::process::Command::new("swiftc")
        .args(["-typecheck", "-swift-version", "6", "-module-cache-path"])
        .arg(dir.join("cache"))
        .args(&paths)
        .output()
        .expect("run swiftc");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::remove_dir_all(dir).expect("remove scratch directory");
}

#[test]
fn self_parameter_does_not_shadow_initializer_receiver() {
    let ir = parse_artifact("Token", r#"{"abi":[{"type":"event","name":"Changed","anonymous":false,"inputs":[{"name":"self","type":"uint256","indexed":true},{"name":"self2","type":"bool","indexed":false},{"name":"self_","type":"bool","indexed":false}]}]}"#).expect("valid ABI");
    let out = render_swift_file(&ir);
    assert!(out.contains("self.self2 = self2"), "{out}");
    assert!(out.contains("self.self22 = self22"), "{out}");
    assert!(out.contains("self.self_ = self_"), "{out}");
    assert!(!out.contains("`self`"), "{out}");
}
