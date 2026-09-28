const SAMPLE: &str = r#"{"abi":[{"type":"function","name":"echo","stateMutability":"view","inputs":[{"name":"payload","type":"tuple","internalType":"struct Sample.Payload","components":[{"name":"amount","type":"uint256"},{"name":"rows","type":"int256[][]"},{"name":"label","type":"string"},{"name":"flags","type":"bool[2]"}]}],"outputs":[{"name":"payload","type":"tuple","internalType":"struct Sample.Payload","components":[{"name":"amount","type":"uint256"},{"name":"rows","type":"int256[][]"},{"name":"label","type":"string"},{"name":"flags","type":"bool[2]"}]}]},{"type":"function","name":"setPayload","stateMutability":"payable","inputs":[{"name":"payload","type":"tuple","internalType":"struct Sample.Payload","components":[{"name":"amount","type":"uint256"},{"name":"rows","type":"int256[][]"},{"name":"label","type":"string"},{"name":"flags","type":"bool[2]"}]}],"outputs":[{"name":"payload","type":"tuple","internalType":"struct Sample.Payload","components":[{"name":"amount","type":"uint256"},{"name":"rows","type":"int256[][]"},{"name":"label","type":"string"},{"name":"flags","type":"bool[2]"}]}]},{"type":"error","name":"Denied","inputs":[{"name":"code","type":"uint256"}]},{"type":"event","name":"Message","anonymous":true,"inputs":[{"name":"text","type":"string","indexed":true},{"name":"code","type":"uint256","indexed":false}]},{"type":"constructor","stateMutability":"payable","inputs":[{"name":"payload","type":"tuple","internalType":"struct Sample.Payload","components":[{"name":"amount","type":"uint256"},{"name":"rows","type":"int256[][]"},{"name":"label","type":"string"},{"name":"flags","type":"bool[2]"}]}]}]}"#;

use abi_typegen_codegen::renderers::ocaml::render_ocaml_artifacts;
use abi_typegen_core::parser::parse_artifact;
#[test]
fn ocaml_preserves_metadata_without_runtime() {
    let ir = parse_artifact("Sample", SAMPLE).expect("ABI");
    let a = render_ocaml_artifacts(&ir, false);
    assert!(a.source.contains("let abi ="));
    assert!(a.source.contains("type atg_payload ="));
    assert!(a.bridge.is_empty());
    assert!(!a.source.contains("external "));
}
#[test]
fn ocaml_uses_checked_runtime_and_exact_integers() {
    let ir = parse_artifact("Sample", SAMPLE).expect("ABI");
    let a = render_ocaml_artifacts(&ir, true);
    assert!(a.source.contains("Z.t"));
    assert!(a.source.contains("word false 256"));
    assert!(a.source.contains("let decode_atg_message_event"));
    assert!(a.source.contains("let decode_atg_denied_error"));
    assert!(a.bridge.contains("atg_decode_event("));
    assert!(a.bridge.contains("ocaml_atg_sample_encode"));
}
