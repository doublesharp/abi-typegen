use abi_typegen_codegen::shell;
use abi_typegen_core::parser::parse_artifact;
use serde_json::json;

#[test]
fn declared_names_match_metadata_and_wrapper_modes_for_every_item_kind() {
    let ir = parse_artifact(
        "Sample",
        &json!({"abi":[
            {"type":"constructor","inputs":[],"stateMutability":"payable"},
            {"type":"function","name":"read","inputs":[],"outputs":[],"stateMutability":"view"},
            {"type":"function","name":"write","inputs":[],"outputs":[],"stateMutability":"payable"},
            {"type":"error","name":"Denied","inputs":[]},
            {"type":"event","name":"Changed","inputs":[],"anonymous":false},
            {"type":"event","name":"Hidden","inputs":[],"anonymous":true}
        ]})
        .to_string(),
    )
    .expect("ABI");
    for wrappers in [false, true] {
        let names = shell::declared_names(&ir, wrappers);
        let source = shell::render_shell_file(&ir, wrappers);
        for name in &names {
            assert!(
                source.contains(&format!("{name}=")) || source.contains(&format!("{name}()")),
                "missing {name}"
            );
        }
        assert_eq!(
            names.len(),
            names.iter().collect::<std::collections::HashSet<_>>().len()
        );
        assert_eq!(names.iter().any(|n| n.ends_with("_deploy")), wrappers);
        assert!(!names.iter().any(|n| n == "ATG_SAMPLE_HIDDEN_EVENT_TOPIC"));
        assert_eq!(
            source.contains("anonymous events cannot be filtered by signature"),
            wrappers
        );
    }
}

#[test]
fn payable_constructor_and_functions_forward_value_while_nonpayable_rejects_it() {
    for mutability in ["payable", "nonpayable"] {
        let ir = parse_artifact("Sample", &json!({"abi":[
            {"type":"constructor","inputs":[{"name":"x","type":"uint256"}],"stateMutability":mutability},
            {"type":"function","name":"write","inputs":[],"outputs":[],"stateMutability":mutability}
        ]}).to_string()).expect("ABI");
        let source = shell::render_shell_file(&ir, true);
        assert!(source.contains("ATG_SAMPLE_CONSTRUCTOR_SIGNATURE='constructor(uint256)'"));
        assert!(source.contains("local atg_bytecode=$1"));
        assert!(source.contains("bytecode must be nonempty, even-length 0x-prefixed hex"));
        assert_eq!(
            source.contains("atg_opts+=(--value"),
            mutability == "payable"
        );
        assert_eq!(
            source.contains("nonpayable call cannot receive value"),
            mutability == "nonpayable"
        );
    }
}

#[test]
fn unusual_solidity_identifiers_keep_safe_unique_shell_names() {
    for (name, prefix) in [("$", "atg_item"), ("_9", "atg__9"), ("$A$B", "atg_a_b")] {
        let ir = parse_artifact(name, r#"{"abi":[{"type":"function","name":"$","inputs":[],"outputs":[],"stateMutability":"view"}]}"#).expect("ABI");
        let source = shell::render_shell_file(&ir, true);
        assert!(
            source.contains(&format!("{prefix}_item_encode()")),
            "{name}"
        );
    }
}

#[test]
fn unnamed_event_fields_and_recursive_types_preserve_cast_signatures() {
    let ir = parse_artifact("Sample", &json!({"abi":[
        {"type":"function","name":"nested","stateMutability":"pure","inputs":[{"name":"x","type":"tuple[][2]","components":[{"name":"flag","type":"bool"},{"name":"data","type":"bytes3"}]}],"outputs":[{"name":"out","type":"int8[]"}]},
        {"type":"event","name":"Changed","anonymous":false,"inputs":[{"name":"","type":"string","indexed":true},{"name":"values","type":"uint256[]","indexed":false}]}
    ]}).to_string()).expect("ABI");
    let source = shell::render_shell_file(&ir, true);
    assert!(source.contains("'nested((bool,bytes3)[][2])(int8[])'"));
    assert!(source.contains("'Changed(string indexed,uint256[] values)'"));
    assert!(source.contains("'Changed(uint256[])'"));
    assert!(source.contains("decode-event --sig"));
}

#[test]
fn abi_metadata_is_shell_quoted_without_changing_apostrophes() {
    let ir = parse_artifact("Sample", &json!({"abi":[{"type":"function","name":"read","stateMutability":"view","inputs":[],"outputs":[],"extra":"don't expand $HOME or `commands`"}]}).to_string()).expect("ABI");
    let source = shell::render_shell_file(&ir, false);
    assert!(source.contains("don'\\''t expand $HOME or `commands`"));
    assert!(!source.contains("atg_sample_read_call()"));
}
