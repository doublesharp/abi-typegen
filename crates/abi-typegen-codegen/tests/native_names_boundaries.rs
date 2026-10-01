use abi_typegen_codegen::{cobol, csharp};
use abi_typegen_core::parser::parse_artifact;
use serde_json::{Value, json};
use std::collections::HashSet;

fn artifact(name: &str, abi: Value) -> abi_typegen_core::types::ContractIr {
    parse_artifact(name, &json!({"abi": abi}).to_string()).expect("valid ABI")
}

#[test]
fn csharp_abi_properties_do_not_hide_sdk_members_or_their_containing_class() {
    let ir = artifact(
        "Names",
        json!([
            {"type":"function","name":"send","stateMutability":"view","inputs":[
                {"name":"gas","type":"uint256"}, {"name":"gas2","type":"uint256"},
                {"name":"amountToSend","type":"uint256"}, {"name":"nonce","type":"uint256"},
                {"name":"namesSendParams","type":"uint256"}, {"name":"toString","type":"string"}
            ],"outputs":[{"name":"namesSendResult","type":"uint256"}]},
            {"type":"constructor","stateMutability":"nonpayable","inputs":[{"name":"namesConstructorParams","type":"bool"}]},
            {"type":"event","name":"Changed","anonymous":false,"inputs":[{"name":"namesChangedEvent","type":"bool","indexed":false}]},
            {"type":"error","name":"Denied","inputs":[{"name":"namesDeniedError","type":"bool"}]},
            {"type":"function","name":"tuple","stateMutability":"view","inputs":[{"name":"value","type":"tuple","internalType":"struct Names.Position","components":[{"name":"namesPosition","type":"bool"}]}],"outputs":[]}
        ]),
    );
    for wrappers in [false, true] {
        let source = csharp::render_csharp_file_with_wrappers(&ir, wrappers);
        for expected in [
            "public BigInteger Gas2 {",
            "public BigInteger Gas22 {",
            "public BigInteger AmountToSend2 {",
            "public BigInteger Nonce2 {",
            "public BigInteger NamesSendParams2 {",
            "public string ToString2 {",
            "public BigInteger NamesSendResult2 {",
            "public bool NamesConstructorParams2 {",
            "public bool NamesChangedEvent2 {",
            "public bool NamesDeniedError2 {",
            "public bool NamesPosition2 {",
        ] {
            assert!(source.contains(expected), "missing {expected}");
        }
        assert!(source.contains("[Parameter(\"uint256\", \"gas\", 1)]"));
        if wrappers {
            assert!(source.contains("new object[] { args.Gas2, args.Gas22, args.AmountToSend2, args.Nonce2, args.NamesSendParams2, args.ToString2 }"));
            assert!(source.contains("new object[] { args.NamesConstructorParams2 }"));
        }
    }
}

#[test]
fn csharp_dollar_only_names_receive_distinct_positional_properties() {
    let ir = artifact(
        "Names",
        json!([
            {"type":"function","name":"read","stateMutability":"view","inputs":[
                {"name":"$","type":"bool"}, {"name":"_","type":"bool"},
                {"name":"arg0","type":"bool"}, {"name":"$0","type":"bool"}
            ],"outputs":[]}
        ]),
    );
    let source = csharp::render_csharp_file_with_wrappers(&ir, true);
    for expected in [
        "public bool Arg0 {",
        "public bool Arg1 {",
        "public bool Arg02 {",
        "public bool Arg3 {",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
    assert!(source.contains("new object[] { args.Arg0, args.Arg1, args.Arg02, args.Arg3 }"));
}

#[test]
fn csharp_constructor_function_dto_does_not_redeclare_constructor_arguments() {
    let ir = artifact(
        "Names",
        json!([
            {"type":"constructor","stateMutability":"nonpayable","inputs":[]},
            {"type":"function","name":"Constructor","stateMutability":"view","inputs":[],"outputs":[]}
        ]),
    );
    let source = csharp::render_csharp_file_with_wrappers(&ir, true);
    assert!(source.contains("public class NamesConstructor2Params : FunctionMessage"));
    assert!(source.contains("public string EncodeConstructor2(NamesConstructor2Params args)"));
    assert!(source.contains("public static string EncodeDeployment(Web3 web3, string bytecode, NamesConstructorParams args)"));
}

#[test]
fn csharp_tuple_types_do_not_redeclare_generated_helpers_or_function_dtos() {
    let ir = artifact(
        "Names",
        json!([
            {"type":"constructor","stateMutability":"nonpayable","inputs":[]},
            {"type":"function","name":"echo","stateMutability":"view","inputs":[
                {"name":"binding","type":"tuple","internalType":"struct Names.Binding","components":[{"name":"ok","type":"bool"}]},
                {"name":"metadata","type":"tuple","internalType":"struct Names.AbiMetadata","components":[{"name":"ok","type":"bool"}]},
                {"name":"constructor","type":"tuple","internalType":"struct Names.ConstructorParams","components":[{"name":"ok","type":"bool"}]},
                {"name":"params","type":"tuple","internalType":"struct Names.EchoParams","components":[{"name":"ok","type":"bool"}]}
            ],"outputs":[{"name":"value","type":"tuple","internalType":"struct Names.EchoResult","components":[{"name":"ok","type":"bool"}]}]}
        ]),
    );
    for wrappers in [false, true] {
        let source = csharp::render_csharp_file_with_wrappers(&ir, wrappers);
        for stem in [
            "Binding",
            "AbiMetadata",
            "ConstructorParams",
            "EchoParams",
            "EchoResult",
        ] {
            assert!(
                source.contains(&format!("public class Names{stem}2\n")),
                "tuple {stem} must have its own type"
            );
        }
        for expected in [
            "public NamesBinding2 Binding {",
            "public NamesAbiMetadata2 Metadata {",
            "public NamesConstructorParams2 Constructor {",
            "public NamesEchoParams2 Params {",
            "public NamesEchoResult2 Value {",
        ] {
            assert!(source.contains(expected), "missing {expected}");
        }
        assert_eq!(
            source
                .matches("public class NamesEchoParams : FunctionMessage")
                .count(),
            1
        );
    }
}

#[test]
fn csharp_tuple_types_do_not_shadow_nethereum_base_classes_or_attributes() {
    let ir = artifact(
        "Function",
        json!([{"type":"function","name":"echo","stateMutability":"view","inputs":[
            {"name":"message","type":"tuple","internalType":"struct Function.Message","components":[{"name":"ok","type":"bool"}]},
            {"name":"output","type":"tuple","internalType":"struct Function.Output","components":[{"name":"ok","type":"bool"}]}
        ],"outputs":[]}]),
    );
    let source = csharp::render_csharp_file_with_wrappers(&ir, true);
    assert!(source.contains("public class FunctionMessage2\n"));
    assert!(source.contains("public class FunctionOutput2\n"));
    assert!(source.contains("public FunctionMessage2 Message {"));
    assert!(source.contains("public FunctionOutput2 Output {"));
    assert!(source.contains("public class FunctionEchoParams : FunctionMessage"));
}

#[test]
fn cobol_normalized_bridge_collisions_keep_each_signature_callable() {
    for contract in ["$", "_0", "VeryLongContractNameWithManyLetters"] {
        let ir = artifact(
            contract,
            json!([
                {"type":"function","name":"$read","stateMutability":"view","inputs":[{"name":"owner","type":"address"}],"outputs":[{"name":"","type":"uint256"}]},
                {"type":"function","name":"_read","stateMutability":"view","inputs":[{"name":"owner","type":"address"}],"outputs":[{"name":"","type":"uint256"}]},
                {"type":"function","name":"$_read","stateMutability":"pure","inputs":[{"name":"owner","type":"address"}],"outputs":[{"name":"","type":"uint256"}]}
            ]),
        );
        let names = cobol::declared_program_names(&ir, true);
        assert_eq!(names.len(), 9);
        assert_eq!(names.iter().collect::<HashSet<_>>().len(), 9);
        assert!(names.iter().all(|name| name.len() <= 27 && name.starts_with(|c: char| c.is_ascii_alphabetic())));
        let files = cobol::render_cobol_artifacts(&ir, true);
        let symbols = files
            .cobol
            .lines()
            .filter_map(|line| line.trim().strip_prefix("CALL \""))
            .map(|call| call.split('"').next().expect("symbol"))
            .collect::<Vec<_>>();
        assert_eq!(symbols.len(), 9);
        assert_eq!(symbols.iter().collect::<HashSet<_>>().len(), 9);
        assert!(
            symbols
                .iter()
                .all(|symbol| files.bridge.contains(&format!("int {symbol}(")))
        );
        for function in &ir.functions {
            assert!(files.bridge.contains(&function.signature()));
        }
        assert!(cobol::declared_program_names(&ir, false).is_empty());
    }
    assert_eq!(cobol::namespace_name("$"), "CONTRACT");
    assert_eq!(cobol::namespace_name("_0"), "C-0");
    assert_eq!(cobol::namespace_name("--already__split--"), "ALREADY-SPLIT");
}
