use abi_typegen_codegen::{cobol, godot, unreal};
use abi_typegen_core::{parser::parse_artifact, types::ContractIr};
use serde_json::{Value, json};
use std::collections::HashSet;

fn contract(name: &str, entries: Value) -> ContractIr {
    parse_artifact(name, &json!({"abi": entries}).to_string()).expect("valid ABI")
}

fn function(name: &str, inputs: Value, outputs: Value, mutability: &str) -> Value {
    json!({"type":"function","name":name,"inputs":inputs,"outputs":outputs,"stateMutability":mutability})
}

fn balance(name: &str, mutability: &str) -> Value {
    function(
        name,
        json!([{"name":"owner","type":"address"}]),
        json!([{"name":"amount","type":"uint256"}]),
        mutability,
    )
}

#[test]
fn cobol_callable_subset_checks_each_input_and_output_requirement() {
    let ir = contract(
        "Token",
        json!([
            balance("balanceOf", "view"), balance("read", "pure"),
            balance("write", "nonpayable"), balance("deposit", "payable"),
            function("noInput", json!([]), json!([{"type":"uint256"}]), "view"),
            function("twoInputs", json!([{"type":"address"},{"type":"address"}]), json!([{"type":"uint256"}]), "view"),
            function("wrongInput", json!([{"type":"bool"}]), json!([{"type":"uint256"}]), "view"),
            function("noOutput", json!([{"type":"address"}]), json!([]), "view"),
            function("twoOutputs", json!([{"type":"address"}]), json!([{"type":"uint256"},{"type":"uint256"}]), "view"),
            function("wrongOutput", json!([{"type":"address"}]), json!([{"type":"uint128"}]), "view"),
            {"type":"event","name":"Transfer","anonymous":false,"inputs":[]},
            {"type":"event","name":"Hidden","anonymous":true,"inputs":[]},
            {"type":"error","name":"Denied","inputs":[]}
        ]),
    );
    let files = cobol::render_cobol_artifacts(&ir, true);
    let names = cobol::declared_program_names(&ir, true);
    assert_eq!(names.len(), 6);
    assert!(names.contains(&"TOKEN-BALANCE-OF-CALL".into()));
    assert!(names.contains(&"TOKEN-READ-CALL".into()));
    assert!(
        files
            .cobol
            .contains("balanceOf(address) selector 0x70A08231")
    );
    assert!(files.cobol.contains("EVENT: Hidden() (anonymous)"));
    assert!(files.cobol.contains("EVENT: Transfer() topic0 0x"));
    assert!(files.cobol.contains("ERROR: Denied() selector 0x"));
    for f in &ir.functions {
        assert!(files.cobol.contains(&f.signature()));
    }
    let plain = cobol::render_cobol_artifacts(&ir, false);
    assert!(cobol::declared_program_names(&ir, false).is_empty());
    assert!(!plain.cobol.contains("PROGRAM-ID."));
    assert!(plain.bridge.contains("const char atg_cobol_Token_abi[]"));
    assert!(!plain.bridge.contains("static int"));
    let empty = contract("Empty", json!([]));
    assert!(cobol::render_cobol_file(&empty, true).contains("No functions"));
    assert!(!cobol::render_cobol_bridge_file(&empty, true).contains("static int"));
}

#[test]
fn cobol_names_are_bounded_unique_and_stable_after_normalization() {
    for (name, expected) in [
        ("", "CONTRACT"),
        ("___", "CONTRACT"),
        ("_0", "C-0"),
        ("fooBar", "FOO-BAR"),
        ("_foo__bar_", "FOO-BAR"),
    ] {
        assert_eq!(cobol::namespace_name(name), expected);
    }
    assert_eq!(
        cobol::namespace_name("ANameLongEnoughToNeedAnIdentifierHash").len(),
        22
    );
    for name in ["", "123", "_", "LongContractNameWithManyWords"] {
        let ir = contract(
            name,
            json!([
                balance("fooBar", "view"),
                balance("foo_bar", "view"),
                balance("_", "view"),
                balance("__", "view"),
                balance("overload", "view"),
                function("overload", json!([]), json!([]), "pure"),
                balance("ANameLongEnoughToNeedAnIdentifierHash", "pure")
            ]),
        );
        let names = cobol::declared_program_names(&ir, true);
        assert_eq!(names.len(), 18);
        assert_eq!(names.iter().collect::<HashSet<_>>().len(), names.len());
        assert!(names.iter().all(|name| name.len() <= 27));
        let files = cobol::render_cobol_artifacts(&ir, true);
        let bridge_names: Vec<_> = files
            .bridge
            .lines()
            .filter_map(|line| {
                line.strip_prefix("int ")
                    .and_then(|line| line.split_once('(').map(|(name, _)| name))
            })
            .collect();
        assert_eq!(bridge_names.len(), 18);
        assert_eq!(
            bridge_names.iter().collect::<HashSet<_>>().len(),
            bridge_names.len()
        );
        assert_eq!(files.cobol, cobol::render_cobol_file(&ir, true));
        assert!(
            files
                .cobol
                .contains("FUNCTION overload 0: overload(address)")
        );
    }
}

#[test]
fn godot_metadata_preserves_constructor_events_and_lossless_descriptors() {
    let ir = contract(
        "Types",
        json!([
            {"type":"constructor","stateMutability":"payable","inputs":[{"name":"bytecode","type":"bytes"}]},
            {"type":"fallback","stateMutability":"payable"},
            {"type":"receive","stateMutability":"payable"},
            function("types", json!([
                {"name":"a","type":"bool"}, {"name":"b","type":"address"},
                {"name":"c","type":"string"}, {"name":"d","type":"bytes"},
                {"name":"e","type":"bytes4"}, {"name":"f","type":"int8"},
                {"name":"g","type":"uint128[2]"}, {"name":"h","type":"uint256[]"},
                {"name":"i","type":"tuple","internalType":"struct Types.Item","components":[{"name":"return","type":"bool"},{"name":"return","type":"address"}]}
            ]), json!([]), "pure"),
            {"type":"event","name":"Changed","anonymous":false,"inputs":[
                {"name":"label","type":"string","indexed":true},
                {"name":"bytes","type":"bytes","indexed":false},
                {"name":"flag","type":"bool","indexed":true},
                {"name":"rows","type":"uint256[]","indexed":true}
            ]},
            {"type":"event","name":"Changed","anonymous":true,"inputs":[{"name":"item","type":"tuple","indexed":true,"components":[{"name":"x","type":"bytes32"}]}]},
            {"type":"error","name":"Denied","inputs":[]},
            {"type":"error","name":"Denied","inputs":[{"name":"why","type":"string"}]}
        ]),
    );
    let plain = godot::render_godot_file(&ir, false);
    for descriptor in [
        "\"kind\":\"bool\"",
        "\"kind\":\"address\"",
        "\"kind\":\"string\"",
        "\"kind\":\"bytes\"",
        "\"kind\":\"bytes_n\",\"size\":4",
        "\"kind\":\"int\",\"bits\":8",
        "\"kind\":\"fixed_array\",\"size\":2",
        "\"kind\":\"array\"",
        "\"kind\":\"tuple\"",
    ] {
        assert!(plain.contains(descriptor), "missing {descriptor}");
    }
    assert!(plain.contains("\"fields\":[\"return_\",\"return_2\"]"));
    assert!(plain.contains("CONSTRUCTOR_SIGNATURE := \"constructor(bytes)\""));
    assert!(plain.contains("HAS_FALLBACK := true"));
    assert!(plain.contains("HAS_RECEIVE := true"));
    assert!(plain.contains("CHANGED_1_EVENT_TOPIC :="));
    assert!(!plain.contains("CHANGED_2_EVENT_TOPIC :="));
    assert!(plain.contains("DENIED_1_ERROR_SELECTOR :="));
    assert!(plain.contains("DENIED_2_ERROR_TYPES := [{\"kind\":\"string\"}]"));
    assert!(!plain.contains("AbiTypegenCodec"));
    let full = godot::render_godot_file(&ir, true);
    assert!(full.contains("encode_constructor(bytecode: PackedByteArray, bytecode2)"));
    assert!(full.contains("send_receive(client:"));
    assert!(full.contains("var topics: Array = []"));
    assert!(full.contains("var topics: Array = [CHANGED_1_EVENT_TOPIC]"));
    assert!(full.contains("decode_denied_1_error"));
    assert!(full.contains("decode_denied_2_error"));
}

#[test]
fn godot_naming_and_payability_keep_generated_methods_distinct() {
    assert_eq!(godot::file_name(""), "contract.gd");
    assert_eq!(godot::namespace_name("___"), "AtgContractContract");
    let ir = contract(
        "Names",
        json!([
            {"type":"constructor","inputs":[]},
            function("", json!([]), json!([]), "pure"),
            function("_0", json!([{"name":"_0","type":"bool"},{"name":"return","type":"bool"},{"name":"_","type":"bool"}]), json!([]), "view"),
            function("deposit", json!([]), json!([]), "payable"),
            function("set", json!([]), json!([]), "nonpayable"),
            function("fooBar", json!([]), json!([]), "view"),
            function("foo_bar", json!([]), json!([]), "view"),
            function("f", json!([]), json!([]), "pure"),
            function("f", json!([{"name":"x","type":"bool"}]), json!([]), "pure")
        ]),
    );
    let full = godot::render_godot_file(&ir, true);
    for expected in [
        "encode_constructor(bytecode: PackedByteArray)",
        "const ITEM_SIGNATURE",
        "const _0_SIGNATURE",
        "encode_arg()",
        "encode__0(_0, return_, arg)",
        "call_foo_bar2(",
        "encode_f_1()",
        "encode_f_2(x)",
        "send_deposit(",
        "send_set(",
    ] {
        assert!(full.contains(expected), "missing {expected}");
    }
    assert_eq!(
        full.matches("nonpayable function cannot receive value")
            .count(),
        1
    );
    let methods: Vec<_> = full
        .lines()
        .filter_map(|line| {
            line.strip_prefix("static func ")
                .and_then(|line| line.split_once('(').map(|(name, _)| name))
        })
        .collect();
    assert_eq!(methods.iter().collect::<HashSet<_>>().len(), methods.len());
}

#[test]
fn unreal_exposes_scalar_reads_and_reports_each_unsupported_operation_once() {
    let ir = contract(
        "Sample",
        json!([
            function("empty", json!([]), json!([]), "pure"),
            function(
                "values",
                json!([
                    {"name":"","type":"bool"}, {"name":"owner","type":"address"},
                    {"name":"amount","type":"uint8"}, {"name":"delta","type":"int256"},
                    {"name":"label","type":"string"}, {"name":"data","type":"bytes"},
                    {"name":"tag","type":"bytes4"}
                ]),
                json!([
                    {"name":"","type":"bool"}, {"name":"owner","type":"address"},
                    {"name":"amount","type":"uint8"}, {"name":"delta","type":"int256"},
                    {"name":"label","type":"string"}, {"name":"data","type":"bytes"},
                    {"name":"tag","type":"bytes4"}
                ]),
                "view"
            ),
            function(
                "nestedInput",
                json!([{"name":"rows","type":"uint256[]"}]),
                json!([]),
                "view"
            ),
            function(
                "nestedOutput",
                json!([]),
                json!([{"name":"rows","type":"uint256[2]"}]),
                "pure"
            ),
            function(
                "tuple",
                json!([{"type":"tuple","components":[{"type":"bool"}]}]),
                json!([]),
                "view"
            ),
            function("deposit", json!([]), json!([]), "payable"),
            function("write", json!([]), json!([]), "nonpayable")
        ]),
    );
    let names = unreal::declared_symbol_names(&ir, true);
    assert_eq!(names.len(), 9);
    assert_eq!(
        unreal::declared_symbol_names(&ir, false),
        ["USampleUnrealMetadata"]
    );
    let full = unreal::render_unreal_artifacts(&ir, true, "HOST_API");
    for expected in [
        "bool bCompleted = true",
        "bool Value0",
        "FAbiTypegenAddress Owner",
        "FAbiTypegenWord256 Amount",
        "FAbiTypegenWord256 Delta",
        "FString Label",
        "FAbiTypegenBytes Data",
        "FAbiTypegenBytes Tag",
    ] {
        assert!(full.header.contains(expected), "missing {expected}");
    }
    for expected in [
        "Params.atg_field = this->Value0",
        "TryToAddress(Params.atg_owner",
        "TryToWord(Params.atg_delta",
        "FTCHARToUTF8 AtgUtf84",
        "this->Data.Data.Num()",
        "bytes4 requires exactly 4 bytes",
        "Reflected.Value0 = Native.atg_field",
        "FromAddress(Native.atg_owner)",
        "FromWord(Native.atg_delta)",
        "FUTF8ToTCHAR Converted",
        "Reflected.Data.Data.Append",
        "Reflected.Tag.Data.Append(Native.atg_tag.bytes, 4)",
    ] {
        assert!(full.source.contains(expected), "missing {expected}");
    }
    assert_eq!(
        full.source
            .matches("tuples and arrays are native-C-only")
            .count(),
        3
    );
    assert_eq!(
        full.source
            .matches("write signing/submission belongs")
            .count(),
        2
    );
    let plain = unreal::render_unreal_artifacts(&ir, false, "HOST_API");
    assert!(plain.header.contains("#include \"Generated/atg_Sample.h\""));
    assert!(plain.source.contains("UTF8_TO_TCHAR(atg_Sample_abi)"));
    assert!(!plain.header.contains("AsyncAction"));
    assert!(!plain.source.contains("write signing/submission belongs"));
}

#[test]
fn unreal_names_handle_empty_numeric_and_case_insensitive_collisions() {
    assert_eq!(unreal::contract_name(""), "Atg");
    assert_eq!(unreal::contract_name("_0"), "Atg0");
    let ir = contract(
        "_0",
        json!([
            function(
                "read",
                json!([{"name":"","type":"bool"},{"name":"_","type":"bool"},{"name":"foo_bar","type":"bool"},{"name":"fooBar","type":"bool"}]),
                json!([{"name":"bCompleted","type":"bool"}]),
                "view"
            ),
            function("read", json!([]), json!([]), "pure"),
            function("fooBar", json!([]), json!([]), "pure"),
            function("foo_bar", json!([]), json!([]), "pure")
        ]),
    );
    let names = unreal::declared_symbol_names(&ir, true);
    assert_eq!(names.len(), 17);
    assert_eq!(
        names
            .iter()
            .map(|name| name.to_ascii_lowercase())
            .collect::<HashSet<_>>()
            .len(),
        names.len()
    );
    let full = unreal::render_unreal_artifacts(&ir, true, "HOST_API");
    assert!(full.header.contains("FooBar2AsyncAction"));
    assert!(full.header.contains("bool BCompleted2;"));
    assert!(full.header.contains("bool FooBar2"));
    assert!(full.source.contains("Params.atg_field2 = this->Atg"));
}

#[test]
fn unreal_reflected_members_do_not_redeclare_generated_types_or_reflection_methods() {
    let ir = contract(
        "Token",
        json!([function(
            "read",
            json!([
                {"name":"uTokenReadAsyncAction","type":"bool"},
                {"name":"staticClass","type":"bool"}
            ]),
            json!([
                {"name":"fTokenReadResult","type":"bool"},
                {"name":"staticStruct","type":"bool"}
            ]),
            "view"
        )]),
    );
    let output = unreal::render_unreal_artifacts(&ir, true, "HOST_API");
    for expected in [
        "bool UTokenReadAsyncAction2;",
        "bool StaticClass2;",
        "bool FTokenReadResult2;",
        "bool StaticStruct2;",
    ] {
        assert!(output.header.contains(expected), "missing {expected}");
    }
    assert!(
        output
            .source
            .contains("Action->UTokenReadAsyncAction2 = UTokenReadAsyncAction2")
    );
    assert!(
        output
            .source
            .contains("Reflected.FTokenReadResult2 = Native.atg_fTokenReadResult")
    );
}
