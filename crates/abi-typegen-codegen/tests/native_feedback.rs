use abi_typegen_codegen::{go, kotlin, rust, swift};
use abi_typegen_core::parser::parse_artifact;

fn contract() -> abi_typegen_core::types::ContractIr {
    parse_artifact("Token", r#"{"abi":[
      {"type":"function","name":"deploy","inputs":[{"name":"p","type":"tuple","internalType":"struct Token.DeployParams","components":[{"name":"a","type":"bytes32"},{"name":"b","type":"bytes32"},{"name":"price","type":"uint256"}]}],"outputs":[],"stateMutability":"nonpayable"},
      {"type":"function","name":"DEFAULT_YEAR_PRICE","inputs":[],"outputs":[],"stateMutability":"view"},
      {"type":"function","name":"defaultYearPrice","inputs":[],"outputs":[],"stateMutability":"view"},
      {"type":"function","name":"balanceOf","inputs":[{"name":"","type":"uint256"},{"name":"","type":"uint256"}],"outputs":[],"stateMutability":"view"},
      {"type":"event","name":"Transfer","inputs":[],"anonymous":false},
      {"type":"error","name":"Failed","inputs":[]}
    ],"metadata":{"output":{"userdoc":{"methods":{"balanceOf(uint256,uint256)":{"notice":"Returns   ``owner``'s balance.\n   More    text with `code`."}}}}}}"#).expect("fixture parses")
}

#[test]
fn kotlin_tuple_has_decoder_constructor() {
    let out = kotlin::render_kotlin_file(&contract(), "test");
    assert!(
        out.contains(
            "constructor(a: Bytes32, b: Bytes32, price: Uint256) : this(a, b, price.value)"
        ),
        "{out}"
    );
}

#[test]
fn kotlin_collisions_precede_role_suffix() {
    let out = kotlin::render_kotlin_file(&contract(), "test");
    assert!(out.contains("class Deploy2Params("), "{out}");
    assert!(out.contains("DEFAULT_YEAR_PRICE2_SIGNATURE"), "{out}");
    assert!(out.contains("DEFAULT_YEAR_PRICE2_SELECTOR"), "{out}");
    assert!(out.contains("val uint256_2: BigInteger"), "{out}");
}

#[test]
fn swift_names_and_prose() {
    let out = swift::render_swift_file(&contract());
    for expected in [
        "struct Deploy2Params",
        "let transferEventTopic",
        "let failedErrorSelector",
        "let uint256_2:",
        "Returns ``owner``'s balance.",
        "More text with `code`.",
    ] {
        assert!(out.contains(expected), "missing {expected}: {out}");
    }
}

#[test]
fn go_prose_is_normalized() {
    let out = go::render_go_file(&contract(), "test");
    assert!(out.contains("Returns “owner“'s balance."), "{out}");
    assert!(out.contains("More text with `code`."), "{out}");
}

#[test]
fn rust_lint_allowance_and_inline_code() {
    let out = rust::render_rust_file(&contract(), true);
    assert!(
        out.contains("#![allow(clippy::too_many_arguments)]"),
        "{out}"
    );
    assert!(out.contains("with `code`."), "{out}");
}
