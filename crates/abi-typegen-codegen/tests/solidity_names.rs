use abi_typegen_codegen::solidity;
use abi_typegen_core::parser::parse_artifact;

#[test]
fn solidity_names_do_not_use_typescript_keyword_escaping() {
    let ir = parse_artifact("Cases", r#"{"abi":[{"type":"function","name":"run","stateMutability":"nonpayable","inputs":[{"name":"class","type":"uint256"},{"name":"_class","type":"bool"}],"outputs":[{"name":"item","type":"tuple","components":[{"name":"class","type":"uint256"},{"name":"_class","type":"bool"}]}]},{"type":"event","name":"Changed","inputs":[{"name":"class","type":"uint256","indexed":true},{"name":"_class","type":"bool","indexed":false}],"anonymous":false},{"type":"error","name":"Bad","inputs":[{"name":"class","type":"uint256"},{"name":"_class","type":"bool"}]}]}"#).unwrap();
    let output = solidity::render_solidity_file(&ir);
    assert!(output.contains("uint256 class;"), "{output}");
    assert!(
        output.contains("run(uint256 class, bool _class)"),
        "{output}"
    );
    assert!(
        output.contains("Changed(uint256 indexed class, bool _class)"),
        "{output}"
    );
    assert!(
        output.contains("Bad(uint256 class, bool _class)"),
        "{output}"
    );
}

#[test]
fn unnamed_inputs_do_not_shadow_named_inputs_or_outputs() {
    let ir = parse_artifact("Cases", r#"{"abi":[{"type":"function","name":"run","stateMutability":"view","inputs":[{"name":"","type":"uint256"},{"name":"arg0","type":"bool"}],"outputs":[{"name":"arg0","type":"bool"},{"name":"","type":"uint256"}]}]}"#).unwrap();
    let output = solidity::render_solidity_file(&ir);
    assert!(output.contains("run(uint256, bool arg0)"), "{output}");
    assert!(output.contains("returns (bool, uint256)"), "{output}");
}

#[test]
fn unnamed_struct_fields_avoid_real_positional_names() {
    let ir = parse_artifact("Cases", r#"{"abi":[{"type":"function","name":"read","stateMutability":"view","inputs":[],"outputs":[{"name":"","type":"tuple","components":[{"name":"","type":"uint256"},{"name":"arg0","type":"bool"}]}]}]}"#).unwrap();
    let output = solidity::render_solidity_file(&ir);
    assert!(output.contains("uint256 arg02;"), "{output}");
    assert!(output.contains("bool arg0;"), "{output}");
}
