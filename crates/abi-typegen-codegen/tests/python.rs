use abi_typegen_codegen::python;
use abi_typegen_core::{parser::parse_artifact, types::ContractIr};
use serde_json::{Value, json};

#[test]
fn named_tuple_class_cannot_be_overwritten_by_the_contract_wrapper() {
    let ir = ir(json!([function(
        "set",
        json!([
            {"name":"item","type":"tuple","internalType":"struct Sample.Contract","components":[{"name":"ok","type":"bool"}]}
        ]),
        json!([]),
        "nonpayable"
    )]));
    for wrappers in [false, true] {
        let source = python::render_python_file_with_wrappers(&ir, wrappers);
        assert!(source.contains("class SampleContract2(TypedDict):"));
        assert!(!source.contains("class SampleContract(TypedDict):"));
        assert_eq!(source.contains("class SampleContract:"), wrappers);
    }
}

#[test]
fn normalized_tuple_names_preserve_distinct_shapes_and_reuse_identical_types() {
    let entries = json!([function(
        "set",
        json!([
            {"name":"first","type":"tuple","internalType":"struct Sample.Dollar$Item","components":[{"name":"n","type":"uint256"}]},
            {"name":"second","type":"tuple","internalType":"struct Sample.Dollar_Item","components":[{"name":"ok","type":"bool"}]},
            {"name":"third","type":"tuple","internalType":"struct Sample.Dollar$Item","components":[{"name":"n","type":"uint256"}]}
        ]),
        json!([]),
        "nonpayable"
    )]);
    let source = python::render_python_file_with_wrappers(&ir(entries), false);
    assert_eq!(
        source
            .matches("class SampleDollar_Item(TypedDict):")
            .count(),
        1
    );
    assert!(source.contains("class SampleDollar_Item2(TypedDict):\n    ok: bool"));
}

#[test]
fn constructor_only_named_tuples_are_available_in_metadata_and_wrappers() {
    let ir = ir(
        json!([{"type":"constructor","stateMutability":"nonpayable","inputs":[
            {"name":"settings","type":"tuple","internalType":"struct Sample.Settings","components":[{"name":"owner","type":"address"}]}
        ]}]),
    );
    for wrappers in [false, true] {
        let source = python::render_python_file_with_wrappers(&ir, wrappers);
        assert!(source.contains("class SampleSettings(TypedDict):"));
        assert!(source.contains("    owner: ChecksumAddress"));
    }
}

#[test]
fn tuple_class_names_with_dollars_are_valid_python_identifiers() {
    let ir = ir(json!([function(
        "set",
        json!([
            {"name":"item","type":"tuple","internalType":"struct Sample.Dollar$Item","components":[{"name":"n","type":"uint256"}]}
        ]),
        json!([]),
        "nonpayable"
    )]));
    let source = python::render_python_file_with_wrappers(&ir, true);
    assert!(source.contains("class SampleDollar_Item(TypedDict):"));
    assert!(!source.contains("class SampleDollar$Item"));
}

fn ir(entries: Value) -> ContractIr {
    parse_artifact("Sample", &json!({"abi":entries}).to_string()).expect("ABI")
}

fn function(name: &str, inputs: Value, outputs: Value, mutability: &str) -> Value {
    json!({"type":"function", "name":name, "inputs":inputs, "outputs":outputs, "stateMutability":mutability})
}

#[test]
fn python_keywords_and_contract_handle_names_have_callable_methods() {
    let ir = ir(json!([
        function("from", json!([]), json!([]), "view"),
        function("class", json!([]), json!([]), "view"),
        function("contract", json!([]), json!([]), "view"),
        function("buildDeployment", json!([]), json!([]), "view")
    ]));
    let source = python::render_python_file_with_wrappers(&ir, true);
    assert!(source.contains("def _from(self)"));
    assert!(source.contains("def _class(self)"));
    assert!(source.contains("def contract_2(self)"));
    assert!(source.contains("def build_deployment_2(self)"));
    assert!(source.contains("get_function_by_signature(\"from()\")"));
}

#[test]
fn embedded_abi_preserves_backslashes_quotes_and_newlines() {
    let entries = json!([{"type":"function", "name":"read", "inputs":[], "outputs":[], "stateMutability":"view", "extra":"line\nbackslash \\ quote ''' end"}]);
    let ir = ir(entries.clone());
    let source = python::render_python_file_with_wrappers(&ir, false);
    let expression = source
        .lines()
        .find(|line| line.starts_with("SAMPLE_ABI:"))
        .expect("ABI assignment");
    let literal = expression
        .split_once("json.loads(")
        .expect("JSON call")
        .1
        .strip_suffix(')')
        .expect("closed call");
    let json_text: String =
        serde_json::from_str(literal).expect("escaped Python-compatible string");
    assert_eq!(
        serde_json::from_str::<Value>(&json_text).expect("ABI JSON"),
        entries
    );
}

#[test]
fn named_tuple_fields_remain_unique_after_keyword_and_snake_case_conversion() {
    let ir = ir(json!([function(
        "set",
        json!([{"name":"item", "type":"tuple", "internalType":"struct Sample.Item", "components":[
            {"name":"class","type":"uint256"}, {"name":"_class","type":"bool"},
            {"name":"fooBar","type":"string"}, {"name":"foo_bar","type":"bytes"}
        ]}]),
        json!([]),
        "nonpayable"
    )]));
    let source = python::render_python_file_with_wrappers(&ir, false);
    for expected in [
        "    _class: int",
        "    _class2: bool",
        "    foo_bar: str",
        "    foo_bar2: bytes",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
}

#[test]
fn read_write_and_constructor_guards_follow_mutability() {
    for mutability in ["payable", "nonpayable"] {
        let ir = ir(json!([
            {"type":"constructor", "stateMutability":mutability, "inputs":[{"name":"transaction","type":"uint256"}]},
            function("write", json!([]), json!([]), mutability),
            function("read", json!([]), json!([{"name":"ok","type":"bool"}]), "pure")
        ]));
        let source = python::render_python_file_with_wrappers(&ir, true);
        assert!(source.contains("transaction2: int"));
        assert!(
            source.contains("contract.constructor(transaction2).build_transaction(transaction)")
        );
        assert!(source.contains("def write(self, transaction: dict[str, Any])"));
        assert!(source.contains("def read(self) -> bool"));
        assert_eq!(
            source.contains("Constructor is not payable"),
            mutability == "nonpayable"
        );
        assert_eq!(
            source.contains("Function is not payable"),
            mutability == "nonpayable"
        );
    }
}

#[test]
fn wrapper_result_shapes_and_recursive_parameter_types_are_exact() {
    let tuple = json!({"name":"items","type":"tuple[][2]", "components":[{"name":"owner","type":"address"},{"name":"blob","type":"bytes3"},{"name":"text","type":"string"}]});
    let ir = ir(json!([
        function("empty", json!([]), json!([]), "view"),
        function("one", json!([tuple.clone()]), json!([tuple]), "view"),
        function(
            "pair",
            json!([]),
            json!([{"name":"x","type":"int256"},{"name":"ok","type":"bool"}]),
            "pure"
        )
    ]));
    let source = python::render_python_file_with_wrappers(&ir, true);
    assert!(source.contains("def decode_empty_result(self, data: str) -> None"));
    assert!(source.contains("Unexpected output for void function"));
    assert!(source.contains("Sequence[Sequence[tuple[ChecksumAddress, bytes, str]]]"));
    assert!(source.contains("def decode_pair_result(self, data: str) -> tuple[int, bool]"));
    assert!(source.contains("_strict_abi_decode([\"int256\", \"bool\"]"));
}

#[test]
fn function_and_event_and_error_helpers_cannot_overwrite_each_other() {
    let ir = ir(json!([
        function("filterChanged", json!([]), json!([]), "view"),
        function("decodeChangedLog", json!([]), json!([]), "view"),
        function("decodeDeniedError", json!([]), json!([]), "view"),
        function("read", json!([]), json!([]), "view"),
        function("encodeRead", json!([]), json!([]), "view"),
        {"type":"event","name":"Changed","anonymous":false,"inputs":[]},
        {"type":"event","name":"Changed","anonymous":false,"inputs":[{"name":"x","type":"bool","indexed":true}]},
        {"type":"error","name":"Denied","inputs":[]},
        {"type":"error","name":"Denied","inputs":[{"name":"x","type":"uint256"}]}
    ]));
    let source = python::render_python_file_with_wrappers(&ir, true);
    for expected in [
        "def encode_read_2(",
        "def filter_changed_2(",
        "def filter_changed_3(",
        "def decode_denied_2_error(",
        "def decode_denied_3_error(",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
    let declarations = source
        .lines()
        .filter(|line| line.starts_with("    def "))
        .map(|line| line.split('(').next().expect("method"))
        .collect::<Vec<_>>();
    let unique = declarations
        .iter()
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(declarations.len(), unique.len());
}

#[test]
fn overloads_across_read_and_write_partitions_use_canonical_signatures() {
    let ir = ir(json!([
        function("echo", json!([]), json!([]), "view"),
        function(
            "echo",
            json!([{"name":"x","type":"uint256"}]),
            json!([]),
            "payable"
        )
    ]));
    let source = python::render_python_file_with_wrappers(&ir, true);
    assert!(source.contains("def echo(self)"));
    assert!(source.contains("def echo_uint256(self, x: int, transaction:"));
    assert!(source.contains("get_function_by_signature(\"echo(uint256)\")"));
    let stub = python::render_python_file(&ir);
    assert!(stub.contains("def echo(self) -> None: ..."));
}

#[test]
fn legacy_stub_types_natspec_and_metadata_modes_remain_available() {
    let artifact = json!({"abi":[
        function("read",json!([{"name":"from","type":"uint256[]"}]),json!([{"name":"out","type":"bool[2]"}]),"view"),
        function("write",json!([]),json!([]),"nonpayable")
    ], "metadata":{"output":{"userdoc":{"notice":"Contract docs", "methods":{"read(uint256[])":{"notice":"Reads values"},"write()":{"notice":"Writes values"}}}}}});
    let ir = parse_artifact("Sample", &artifact.to_string()).expect("ABI/docs");
    let stub = python::render_python_file(&ir);
    assert!(stub.contains("_from: list[int]"));
    assert!(stub.contains("Reads values"));
    assert!(stub.contains("Writes values"));
    assert!(stub.contains("Contract docs"));
    let metadata = python::render_python_file_with_wrappers(&ir, false);
    assert!(!metadata.contains("class SampleContract"));
    assert!(!metadata.contains("def encode_"));
}

#[test]
fn tuples_in_events_errors_and_unnamed_parameters_keep_declarations() {
    let tuple = json!({"name":"item","type":"tuple","components":[{"name":"x","type":"uint256"}]});
    let mut event_tuple = tuple.clone();
    event_tuple["indexed"] = json!(false);
    let ir = ir(json!([
        {"type":"event","name":"Changed","anonymous":true,"inputs":[event_tuple]},
        {"type":"error","name":"Denied","inputs":[tuple.clone()]},
        function("set",json!([tuple, {"name":"","type":"tuple","components":[{"name":"x","type":"bool"}]}]),json!([]),"payable")
    ]));
    let source = python::render_python_file_with_wrappers(&ir, true);
    assert_eq!(source.matches("class Sampleitem(TypedDict)").count(), 1);
    assert!(source.contains("process_log(log)"));
    assert!(source.contains("Custom error selector mismatch"));
    assert!(source.contains("arg1: tuple[bool]"));
}

#[test]
fn quoted_multiline_natspec_is_emitted_as_a_safe_python_docstring() {
    let notice = "Quoted \"\"\" docs\nbackslash \\ and newline";
    let artifact = json!({"abi":[function("read",json!([]),json!([]),"view")], "metadata":{"output":{"userdoc":{"notice":notice,"methods":{"read()":{"notice":notice}}}}}});
    let ir = parse_artifact("Sample", &artifact.to_string()).expect("ABI/docs");
    let source = python::render_python_file(&ir);
    assert!(source.contains(&format!(
        "    {}",
        serde_json::to_string(&format!("{notice}.")).expect("docstring")
    )));
    assert!(source.contains(&format!(
        "        {}",
        serde_json::to_string(notice).expect("method docstring")
    )));
}

#[test]
fn helper_families_are_safe_in_either_declaration_order() {
    for first in ["encodeRead", "decodeReadResult"] {
        let ir = ir(json!([
            function(first, json!([]), json!([]), "view"),
            function("read", json!([]), json!([]), "view")
        ]));
        let source = python::render_python_file_with_wrappers(&ir, true);
        assert!(source.contains("def read_2(self)"));
        assert!(source.contains("def encode_read_2(self)"));
        assert!(source.contains("def decode_read_2_result(self, data:"));
    }
    let ir = ir(json!([
        function("decodeChangedLog",json!([]),json!([]),"view"),
        {"type":"event","name":"Changed","anonymous":false,"inputs":[]}
    ]));
    assert!(
        python::render_python_file_with_wrappers(&ir, true).contains("def decode_changed_2_log(")
    );
}

#[test]
fn zero_argument_constructors_and_missing_natspec_notice_remain_usable() {
    let mut ir = ir(json!([{"type":"constructor","inputs":[],"stateMutability":"payable"}]));
    ir.natspec = Some(Default::default());
    let source = python::render_python_file_with_wrappers(&ir, true);
    assert!(
        source
            .contains("def build_deployment(w3: Web3, bytecode: str, transaction: dict[str, Any])")
    );
    assert!(source.contains("contract.constructor().build_transaction(transaction)"));
    assert!(source.contains("Typed wrapper for the Sample contract"));
}

#[test]
fn unusual_but_valid_solidity_parameter_names_fall_back_to_safe_python_identifiers() {
    let ir = ir(json!([function(
        "read",
        json!([{"name":"$","type":"bool"},{"name":"_9","type":"uint256"}]),
        json!([]),
        "view"
    )]));
    let source = python::render_python_file_with_wrappers(&ir, true);
    assert!(source.contains("arg0: bool, arg1: int"));
    assert!(source.contains("(arg0, arg1).call()"));
}

#[test]
fn duplicate_named_and_unnamed_event_and_error_tuples_do_not_duplicate_declarations() {
    let named = json!({"name":"item","type":"tuple","internalType":"struct Sample.Item","components":[{"name":"x","type":"uint256"}]});
    let unnamed = json!({"name":"","type":"tuple","components":[{"name":"x","type":"bool"}]});
    let mut event = named.clone();
    event["indexed"] = json!(false);
    let mut unnamed_event = unnamed.clone();
    unnamed_event["indexed"] = json!(false);
    let ir = ir(json!([
        {"type":"event","name":"First","anonymous":false,"inputs":[event.clone(),unnamed_event.clone()]},
        {"type":"event","name":"Second","anonymous":false,"inputs":[event,unnamed_event]},
        {"type":"error","name":"Denied","inputs":[named.clone(),unnamed.clone()]},
        {"type":"error","name":"Failed","inputs":[named,unnamed]}
    ]));
    let source = python::render_python_file_with_wrappers(&ir, false);
    assert_eq!(source.matches("class SampleItem(TypedDict)").count(), 1);
    assert!(!source.contains("class Sample(TypedDict)"));
}
