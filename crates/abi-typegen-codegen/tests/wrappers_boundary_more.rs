use abi_typegen_codegen::{csharp, go};
use abi_typegen_core::{parser::parse_artifact, types::ContractIr};
use serde_json::{Value, json};

fn contract(entries: Value) -> ContractIr {
    parse_artifact("Sample", &json!({"abi":entries}).to_string()).expect("valid ABI")
}

fn function(name: &str, inputs: Value, outputs: Value, mutability: &str) -> Value {
    json!({"type":"function", "name":name, "inputs":inputs, "outputs":outputs, "stateMutability":mutability})
}

#[test]
fn csharp_constructor_and_write_guards_follow_transaction_payability() {
    for (mutability, inputs) in [
        ("payable", json!([])),
        ("nonpayable", json!([{"name":"owner","type":"address"}])),
    ] {
        let ir = contract(json!([
            {"type":"constructor","inputs":inputs,"stateMutability":mutability},
            function("deposit", json!([]), json!([]), "payable"),
            function("set", json!([{"name":"flag","type":"bool"}]), json!([]), "nonpayable"),
            function("ping", json!([]), json!([]), "view"),
            function("pair", json!([]), json!([{"name":"x","type":"bool"},{"name":"y","type":"string"}]), "pure")
        ]));
        let source = csharp::render_csharp_file_with_wrappers(&ir, true);
        assert!(source.contains("public static string EncodeDeployment("));
        assert!(source.contains("public static Task<string> DeployAsync("));
        assert_eq!(
            source.contains("Constructor is not payable"),
            mutability == "nonpayable"
        );
        assert_eq!(source.matches("Function is not payable").count(), 1);
        assert!(source.contains("GetData(Array.Empty<object>())"));
        assert!(source.contains("GetData(new object[] { args.Flag })"));
        assert!(source.contains("Task<SamplePingResult> PingAsync("));
        assert!(source.contains("Task<SamplePairResult> PairAsync("));
        assert!(source.contains("public bool X { get; set; }"));
        assert!(source.contains("public string Y { get; set; }"));
        let plain = csharp::render_csharp_file_with_wrappers(&ir, false);
        assert!(plain.contains("SampleConstructorParams"));
        assert!(plain.contains("SamplePairResult"));
        assert!(plain.contains("SampleAbiMetadata"));
        assert!(!plain.contains("class SampleBinding"));
    }
}

#[test]
fn csharp_indexed_reference_fields_and_filter_limits_preserve_primary_metadata() {
    let ir = contract(json!([
        {"type":"event","name":"Empty","anonymous":false,"inputs":[]},
        {"type":"event","name":"Recorded","anonymous":false,"inputs":[
            {"name":"text","type":"string","indexed":true},
            {"name":"owner","type":"address","indexed":true},
            {"name":"rows","type":"uint256[2]","indexed":true},
            {"name":"payload","type":"bytes","indexed":false}
        ]},
        {"type":"event","name":"Hidden","anonymous":true,"inputs":[
            {"name":"a","type":"bool","indexed":true},
            {"name":"b","type":"bool","indexed":true},
            {"name":"c","type":"bool","indexed":true},
            {"name":"d","type":"bool","indexed":true}
        ]},
        {"type":"error","name":"Denied","inputs":[]},
        {"type":"error","name":"Failed","inputs":[{"name":"reason","type":"string"},{"name":"code","type":"uint16"}]}
    ]));
    let source = csharp::render_csharp_file_with_wrappers(&ir, true);
    for expected in [
        "public byte[] Text",
        "public string Owner",
        "public byte[] Rows",
        "public byte[] Payload",
        "FilterRecordedByTopics(string[] topic0 = null, string[] topic1 = null, List<BigInteger>[] topic2 = null",
        "DecodeDeniedError(string data)",
        "DecodeFailedError(string data)",
        "data.Length != 10",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
    assert!(!source.contains("FilterEmptyByTopics"));
    assert!(!source.contains("FilterHiddenByTopics"));
    assert_eq!(source.matches("Unexpected custom error payload").count(), 1);
    assert!(source.contains(r#"""anonymous"":true"#));
}

#[test]
fn csharp_normalized_names_and_repeated_declarations_remain_distinct() {
    let ir = contract(json!([
        function("fooBar", json!([{"name":"_","type":"bool"},{"name":"fooBar","type":"bool"},{"name":"foo_bar","type":"bool"}]), json!([]), "view"),
        function("foo_bar", json!([]), json!([]), "pure"),
        function("read", json!([{"name":"x","type":"bool"}]), json!([]), "view"),
        function("read", json!([]), json!([]), "view"),
        {"type":"event","name":"Seen","anonymous":false,"inputs":[]},
        {"type":"event","name":"Seen","anonymous":false,"inputs":[{"name":"x","type":"bool","indexed":true}]},
        {"type":"error","name":"Denied","inputs":[]},
        {"type":"error","name":"Denied","inputs":[{"name":"x","type":"bool"}]}
    ]));
    let source = csharp::render_csharp_file_with_wrappers(&ir, true);
    for expected in [
        "SampleFooBarParams",
        "SampleFooBar2Params",
        "public bool Arg0",
        "public bool FooBar2",
        "FilterSeen2(",
        "SampleSeen2Event",
        "DecodeDenied2Error(",
        "SampleDenied2Error",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
}

#[test]
fn go_constructor_arguments_payability_and_empty_read_results_have_distinct_apis() {
    for (mutability, inputs) in [
        ("payable", json!([{"name":"owner","type":"address"}])),
        ("nonpayable", json!([])),
    ] {
        let ir = contract(json!([
            {"type":"constructor","inputs":inputs,"stateMutability":mutability},
            function("ping", json!([]), json!([]), "view"),
            function("pair", json!([{"name":"x","type":"bool"}]), json!([{"name":"left","type":"bool"},{"name":"right","type":"string"}]), "pure"),
            function("deposit", json!([]), json!([]), "payable"),
            function("set", json!([{"name":"x","type":"bool"}]), json!([]), "nonpayable")
        ]));
        let source = go::render_go_file_with_wrappers(&ir, "contracts", true);
        assert_eq!(
            source.contains("constructor is not payable"),
            mutability == "nonpayable"
        );
        assert!(source.contains("func (c *SampleBinding) Ping(opts *bind.CallOpts) error"));
        assert!(source.contains("func (c *SampleBinding) DecodePingResult(data []byte) error"));
        assert!(source.contains("unexpected return data for %s"));
        assert!(source.contains("func (c *SampleBinding) Pair(opts *bind.CallOpts, params SamplePairParams) (SamplePairResult, error)"));
        assert!(source.contains("out.Right = *abi.ConvertType(values[1], new(string)).(*string)"));
        assert_eq!(source.matches("%s is not payable").count(), 1);
        assert_eq!(
            source.contains("bytecode []byte, params SampleConstructorParams"),
            mutability == "payable"
        );
        assert_eq!(
            source.contains("c.abi.Pack(\"\", params.Owner)"),
            mutability == "payable"
        );
    }
}

#[test]
fn go_anonymous_events_keep_hash_topics_and_custom_error_selectors() {
    for anonymous in [false, true] {
        let ir = contract(json!([
            {"type":"event","name":"Recorded","anonymous":anonymous,"inputs":[
                {"name":"item","type":"tuple","indexed":true,"components":[{"name":"x","type":"uint256"}]},
                {"name":"label","type":"string","indexed":true},
                {"name":"owner","type":"address","indexed":true},
                {"name":"flag","type":"bool","indexed":false}
            ]},
            {"type":"event","name":"Empty","anonymous":anonymous,"inputs":[]},
            {"type":"error","name":"Denied","inputs":[]},
            {"type":"error","name":"Failed","inputs":[{"name":"reason","type":"string"}]}
        ]));
        let source = go::render_go_file_with_wrappers(&ir, "contracts", true);
        let tokens = source.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(tokens.contains("Item common.Hash"));
        assert!(tokens.contains("Label common.Hash"));
        assert!(source.contains(&format!(
            "out.Item = log.Topics[{}]",
            usize::from(!anonymous)
        )));
        assert!(source.contains(&format!(
            "if len(log.Topics) != {}",
            3 + usize::from(!anonymous)
        )));
        assert_eq!(source.contains("event signature mismatch"), !anonymous);
        assert_eq!(
            source.contains("queries = append([][]any{{definition.ID}}, queries...)"),
            !anonymous
        );
        assert!(source.contains("indexed = append(indexed, definition.Inputs[1])"));
        assert!(source.contains("out.Flag = *abi.ConvertType(values[0], new(bool)).(*bool)"));
        assert!(source.contains("func (c *SampleBinding) DecodeDeniedError(raw []byte)"));
        assert!(source.contains("return c.DecodeFailedError(raw)"));
        assert!(source.contains("unknown custom error selector %x"));
    }
}
