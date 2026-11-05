use abi_typegen_codegen::{go, kotlin, rust, swift};
use abi_typegen_core::parser::parse_artifact;

// ABI entries from Diesis contracts 17f837ac61c4e1088fb4acbb95e45d6f1d7b8e9d:
// IDiesisPosition, IDiesisNameVerifier, and IDiesisPublicResolver. In particular,
// the verifier's second attestations argument is bytes32, not uint16.
const OVERLOADS: &str = include_str!("../../../tests/fixtures/overloads.json");

const OVERLOAD_SIGNATURES: [(&str, &str); 6] = [
    ("safeTransferFrom(address,address,uint256)", "0x42842e0e"),
    (
        "safeTransferFrom(address,address,uint256,bytes)",
        "0xb88d4fde",
    ),
    ("attestations(bytes32,bytes32)", "0x92abd17b"),
    ("attestations(bytes32)", "0x940992a3"),
    ("setAddr(bytes32,uint256,bytes)", "0x8b95dd71"),
    ("setAddr(bytes32,address)", "0xd5fa2b00"),
];

#[test]
fn fixture_overload_aliases_keep_their_signatures_and_selectors() {
    use abi_typegen_codegen::{fsharp, ocaml};
    let ir = parse_artifact("Overloads", OVERLOADS).unwrap();
    let go = go::render_go_file_with_wrappers(&ir, "contracts", true);
    let swift = swift::render_swift_file_with_wrappers(&ir, true);
    let kotlin = kotlin::render_kotlin_file_with_wrappers(&ir, "contracts", true);
    let fsharp = fsharp::render_fsharp_file(&ir, true);
    let ocaml = ocaml::render_ocaml_artifacts(&ir, true).source;
    let compact = |s: &str| s.split_whitespace().collect::<String>();
    let go = compact(&go);
    let swift = compact(&swift);
    let kotlin = compact(&kotlin);
    let fsharp = compact(&fsharp);
    for (i, (signature, selector)) in OVERLOAD_SIGNATURES.iter().enumerate() {
        let lower = ["safeTransferFrom", "attestations", "setAddr"][i / 2];
        let upper = ["SafeTransferFrom", "Attestations", "SetAddr"][i / 2];
        let snake = ["safe_transfer_from", "attestations", "set_addr"][i / 2];
        let shout = ["SAFE_TRANSFER_FROM", "ATTESTATIONS", "SET_ADDR"][i / 2];
        let index = i % 2;
        let go_suffix = if index == 0 { "" } else { "0" };
        let suffix = if index == 0 { "" } else { "2" };
        let bytes = (2..10)
            .step_by(2)
            .map(|n| format!("0x{}", &selector[n..n + 2]))
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(ir.functions[i].signature(), *signature);
        assert_eq!(ir.functions[i].selector().to_string(), *selector);
        for (source, expected) in [
            (
                &go,
                format!("Overloads{upper}{go_suffix}Signature=\"{signature}\""),
            ),
            (
                &go,
                format!("Overloads{upper}{go_suffix}Selector=[4]byte{{{bytes}}}"),
            ),
            (
                &swift,
                format!("let{lower}{index}Signature=\"{signature}\""),
            ),
            (&swift, format!("let{lower}{index}Selector=Data([{bytes}])")),
            (
                &kotlin,
                format!("val{shout}_{index}_SIGNATURE:String=\"{signature}\""),
            ),
            (
                &kotlin,
                format!("val{shout}_{index}_SELECTOR:String=\"{selector}\""),
            ),
            (
                &fsharp,
                format!(
                    "letencode{upper}{suffix}(contract:Contract)(args:{upper}{suffix}Params)=contract.GetFunctionBySignature(\"{selector}\")"
                ),
            ),
            (
                &ocaml,
                format!("let atg_{snake}{suffix}_signature = \"{signature}\""),
            ),
        ] {
            assert!(source.contains(&expected), "missing {expected}");
        }
    }
}

#[test]
fn signature_based_javascript_and_python_aliases_stay_compatible() {
    use abi_typegen_codegen::{ethers, ethers5, python};
    use serde_json::Value;
    let mut artifact: Value = serde_json::from_str(OVERLOADS).unwrap();
    for reversed in [false, true] {
        if reversed {
            artifact["abi"].as_array_mut().unwrap().reverse();
        }
        let ir = parse_artifact("Overloads", &artifact.to_string()).unwrap();
        for (source, ethers_v5) in [
            (ethers::render_ethers_file(&ir), false),
            (ethers5::render_ethers5_file(&ir), true),
        ] {
            for (alias, (signature, _)) in [
                "safeTransferFromAddressAddressUint256",
                "safeTransferFromAddressAddressUint256Bytes",
                "attestationsBytes32Bytes32",
                "attestationsBytes32",
                "setAddrBytes32Uint256Bytes",
                "setAddrBytes32Address",
            ]
            .iter()
            .zip(OVERLOAD_SIGNATURES)
            {
                let expected = if ethers_v5 {
                    format!("{alias}: contract[\"{signature}\"]")
                } else {
                    format!("{alias}: contract.getFunction(\"{signature}\")")
                };
                assert!(source.contains(&expected), "missing {expected}");
            }
        }
        let source = python::render_python_file_with_wrappers(&ir, true);
        for (alias, (signature, _)) in [
            "safe_transfer_from_address_address_uint256",
            "safe_transfer_from_address_address_uint256_bytes",
            "attestations_bytes32_bytes32",
            "attestations_bytes32",
            "set_addr_bytes32_uint256_bytes",
            "set_addr_bytes32_address",
        ]
        .iter()
        .zip(OVERLOAD_SIGNATURES)
        {
            let body = source
                .split(&format!("def encode_{alias}("))
                .nth(1)
                .expect("signature-based alias");
            assert!(
                body.split("\n    def ")
                    .next()
                    .unwrap()
                    .contains(&format!("self.contract.encode_abi(\"{signature}\""))
            );
        }
    }
}

fn without_abi_line(source: &str, ir: &abi_typegen_core::types::ContractIr) -> String {
    let abi = serde_json::to_string(&ir.raw_abi).unwrap();
    let escaped = serde_json::to_string(&abi).unwrap();
    source
        .lines()
        .filter(|line| !line.contains(&abi) && !line.contains(&escaped))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn overload_aliases_survive_compiler_abi_reordering() {
    use abi_typegen_codegen::generate_contract_files;
    use abi_typegen_config::Config;
    use serde_json::Value;

    let original = parse_artifact("Overloads", OVERLOADS).expect("fixture parses");
    let mut artifact: Value = serde_json::from_str(OVERLOADS).unwrap();
    for group in artifact["abi"].as_array_mut().unwrap().chunks_mut(2) {
        group.reverse();
    }
    let reversed = parse_artifact("Overloads", &artifact.to_string()).unwrap();

    for target in ["rust", "go", "swift", "kotlin", "fsharp", "ocaml"] {
        let config = Config::from_toml_str(&format!("[abi-typegen]\ntarget = '{target}'")).unwrap();
        let files = generate_contract_files(&original, &config);
        let reversed_files = generate_contract_files(&reversed, &config);
        // Embedded ABI JSON deliberately retains input order. Compare every
        // generated declaration and wrapper after removing only that ABI line.
        for (name, source) in files {
            assert_eq!(
                without_abi_line(&source, &original),
                without_abi_line(&reversed_files[&name], &reversed),
                "{target}: {name} aliases changed"
            );
        }
    }
}

#[test]
fn equal_arity_tuple_and_array_overloads_generate_stable_aliases() {
    use abi_typegen_codegen::generate_contract_files;
    use abi_typegen_config::Config;
    use serde_json::json;
    let nested = json!({"type":"function","name":"route","stateMutability":"view","inputs":[
        {"name":"items","type":"tuple[][2]","internalType":"struct Routes.Item[][2]","components":[
            {"name":"amount","type":"uint"},
            {"name":"children","type":"tuple[]","internalType":"struct Routes.Child[]","components":[
                {"name":"owner","type":"address"},{"name":"tags","type":"bytes32[2]"}
            ]}
        ]},{"name":"weights","type":"uint256[3][]"}
    ],"outputs":[]});
    let simple = json!({"type":"function","name":"route","stateMutability":"view","inputs":[
        {"name":"keys","type":"bytes32[2][]"},{"name":"weights","type":"uint256[3][]"}
    ],"outputs":[]});
    let original = parse_artifact("Routes", &json!({"abi":[nested, simple]}).to_string()).unwrap();
    let mut reversed = original.clone();
    // The raw ABI must keep the caller's order; only the parsed model is normalized.
    reversed.raw_abi.as_array_mut().unwrap().reverse();
    let reordered = parse_artifact("Routes", &json!({"abi":reversed.raw_abi}).to_string()).unwrap();
    for target in ["rust", "go", "swift", "kotlin", "fsharp", "ocaml"] {
        let config = Config::from_toml_str(&format!("[abi-typegen]\ntarget = '{target}'")).unwrap();
        let clean = |ir: &abi_typegen_core::types::ContractIr| {
            let mut files = generate_contract_files(ir, &config);
            for source in files.values_mut() {
                *source = without_abi_line(source, ir);
            }
            files
        };
        assert_eq!(clean(&original), clean(&reordered), "{target}");
    }
    assert_eq!(original.functions[0].selector().to_string(), "0x2c65d8fc");
    assert_eq!(original.functions[1].selector().to_string(), "0x7248a337");
}

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
