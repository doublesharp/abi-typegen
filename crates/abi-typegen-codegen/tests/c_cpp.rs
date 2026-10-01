use abi_typegen_codegen::{c, cpp, generate_contract_files};
use abi_typegen_config::{Config, Target};
use abi_typegen_core::{parser::parse_artifact, types::ContractIr};
use serde_json::{Value, json};

fn contract(entries: Value) -> ContractIr {
    parse_artifact("Sample", &json!({"abi":entries}).to_string()).expect("valid ABI")
}

fn function(name: &str, inputs: Value, outputs: Value, mutability: &str) -> Value {
    json!({"type":"function","name":name,"inputs":inputs,"outputs":outputs,"stateMutability":mutability})
}

fn line<'a>(source: &'a str, signature: &str) -> &'a str {
    source
        .lines()
        .find(|line| line.contains(signature))
        .unwrap_or_else(|| panic!("missing {signature}"))
}

#[test]
fn c_and_cpp_dispatch_emit_headers_using_the_shared_runtime() {
    let ir = contract(json!([]));
    for (target, count) in [(Target::C, 1), (Target::Cpp, 2)] {
        for wrappers in [false, true] {
            let config = Config {
                targets: vec![target.clone()],
                wrappers,
                ..Config::from_toml_str("").expect("config")
            };
            let files = generate_contract_files(&ir, &config);
            assert_eq!(files.len(), count);
            assert!(files["atg_Sample.h"].contains("#include \"abi_typegen.h\""));
            assert!(files["atg_Sample.h"].contains("static const char atg_Sample_abi[] = \"[]\""));
            if target == Target::Cpp {
                assert!(files["atg_Sample.hpp"].contains("namespace atg_Sample {"));
            }
        }
    }
    assert_eq!(
        c::RUNTIME_HEADER,
        include_str!("../../abi-typegen-runtime/include/abi_typegen.h")
    );
}

#[test]
fn c_identifiers_handle_keywords_punctuation_and_empty_names() {
    for (input, expected) in [
        ("class", "atg_class"),
        ("123", "atg_123"),
        ("__foo__bar_", "atg_foo_bar"),
        ("", "atg_field"),
        ("a-b", "atg_a_b"),
    ] {
        assert_eq!(c::namespace_name(input), expected);
    }
    let ir = contract(json!([function(
        "set",
        json!([
            {"name":"class","type":"bool"}, {"name":"","type":"bool"},
            {"name":"","type":"bool"}, {"name":"field","type":"bool"}
        ]),
        json!([]),
        "nonpayable"
    )]));
    let source = c::render_c_file(&ir, true);
    for field in ["atg_class;", "atg_field;", "atg_field2;", "atg_field3;"] {
        assert!(source.contains(field), "missing {field}");
    }
}

#[test]
fn metadata_mode_keeps_all_value_types_and_identities_without_callables() {
    let ir = parse_artifact("Token", include_str!("../../../tests/fixtures/erc20.json"))
        .expect("fixture");
    let c_source = c::render_c_file(&ir, false);
    let cpp_source = cpp::render_cpp_file(&ir, false);
    for expected in [
        "atg_Token_atg_transfer_params",
        "atg_Token_atg_transfer_returns",
        "atg_Token_atg_Transfer_event_fields",
        "atg_Token_atg_transfer_signature[] = \"transfer(address,uint256)\"",
        "atg_Token_atg_transfer_selector[4] = {169,5,156,187}",
    ] {
        assert!(c_source.contains(expected), "missing {expected}");
    }
    for expected in [
        "using ConstructorParams",
        "using atg_transfer_params",
        "using atg_transfer_returns",
        "using atg_Transfer_event_fields",
        "inline constexpr const char *abi = ::atg_Token_abi",
    ] {
        assert!(cpp_source.contains(expected), "missing {expected}");
    }
    for absent in [
        "atg_encode(",
        "_to_value(",
        "atg_decode_event(",
        "atg_Token_client",
    ] {
        assert!(!c_source.contains(absent), "unexpected {absent}");
    }
    for absent in ["RuntimeResult", "class Client", "Decoded<", "copy_bytes("] {
        assert!(!cpp_source.contains(absent), "unexpected {absent}");
    }
}

#[test]
fn scalar_codecs_use_exact_words_addresses_and_byte_lengths() {
    let params = json!([
        {"name":"flag","type":"bool"}, {"name":"amount","type":"uint256"},
        {"name":"delta","type":"int8"}, {"name":"owner","type":"address"},
        {"name":"blob","type":"bytes"}, {"name":"label","type":"string"},
        {"name":"key","type":"bytes3"}
    ]);
    let source = c::render_c_file(
        &contract(json!([function("echo", params.clone(), params, "pure")])),
        true,
    );
    for expected in [
        "return atg_value_bool(*x)",
        "return atg_value_word(x->bytes)",
        "memcpy(word + 12, x->bytes, 20)",
        "memcpy(x->bytes, atg_value_data(v) + 12, 20)",
        "return atg_value_bytes(x->data, x->len)",
        "atg_value_len(v) != 3",
        "memcpy(x->bytes, atg_value_data(v), 3)",
        "x->data = atg_value_data(v); x->len = atg_value_len(v)",
    ] {
        assert!(source.contains(expected), "missing {expected}");
    }
}

#[test]
fn nested_arrays_allocate_owned_storage_and_fixed_arrays_check_length() {
    let params =
        json!([{ "name":"grid", "type":"uint256[][]" }, {"name":"flags","type":"bool[2]"}]);
    let source = c::render_c_file(
        &contract(json!([function("echo", params.clone(), params, "pure")])),
        true,
    );
    assert!(source.contains("x->len > SIZE_MAX / sizeof("));
    assert!(source.contains("atg_result_alloc(owner, x->len * sizeof("));
    assert!(source.contains("if (x->len && !x->data) return false"));
    assert!(source.contains("if (x->len && !x->data) { atg_value_free(seq); return NULL; }"));
    assert!(source.contains("if (atg_value_len(v) != 2) return false"));
    assert!(!source.contains("if (2 && !x->data)"));
    assert!(source.contains("atg_value_at(v,i)"));
}

#[test]
fn repeated_tuple_shapes_share_one_type_and_empty_structs_are_legal_c() {
    let tuple = json!({"name":"point","type":"tuple","internalType":"struct Sample.Point","components":[{"name":"x","type":"uint256"},{"name":"ok","type":"bool"}]});
    let source = c::render_c_file(
        &contract(json!([
            function("echo", json!([tuple.clone()]), json!([tuple]), "pure"),
            function("noop", json!([]), json!([]), "view")
        ])),
        true,
    );
    assert_eq!(source.matches("} atg_Sample_tuple_atg_Point;").count(), 1);
    assert_eq!(
        source
            .matches("atg_Sample_tuple_atg_Point atg_point;")
            .count(),
        2
    );
    assert!(source.contains("unsigned char reserved;"));
    assert!(source.contains("atg_value_len(v) != 0"));
}

#[test]
fn function_reads_decode_responses_while_writes_return_transport_results() {
    let ir = contract(json!([
        function(
            "read",
            json!([]),
            json!([{"name":"n","type":"uint256"}]),
            "view"
        ),
        function("noop", json!([]), json!([]), "pure"),
        function("write", json!([]), json!([]), "nonpayable"),
        function("deposit", json!([]), json!([]), "payable")
    ]));
    let source = c::render_c_file(&ir, true);
    for name in ["read", "noop"] {
        let wrapper = line(&source, &format!("*atg_Sample_atg_{name}("));
        assert!(wrapper.contains("0,options)"));
        assert!(wrapper.contains("atg_result_free(response); return decoded"));
        assert!(wrapper.contains("nonpayable function cannot receive value"));
    }
    for name in ["write", "deposit"] {
        let wrapper = line(&source, &format!("*atg_Sample_atg_{name}("));
        assert!(wrapper.contains("1,options)"));
        assert!(wrapper.ends_with("return response; }"));
        assert_eq!(
            wrapper.contains("nonpayable function cannot receive value"),
            name == "write"
        );
    }
    let cpp_source = cpp::render_cpp_file(&ir, true);
    assert!(cpp_source.contains("Decoded<atg_read_returns> atg_read("));
    assert!(cpp_source.contains("std::vector<uint8_t> atg_write("));
}

#[test]
fn constructors_send_deployment_bytecode_with_no_destination() {
    for payable in [false, true] {
        let ir = contract(
            json!([{"type":"constructor","inputs":[{"name":"n","type":"uint256"}],"stateMutability":if payable {"payable"} else {"nonpayable"}}]),
        );
        let source = c::render_c_file(&ir, true);
        assert!(source.contains("atg_encode_constructor(atg_Sample_abi,bytecode,len,v)"));
        let deploy = line(&source, "*atg_Sample_deploy(");
        assert!(deploy.contains(
            "transport(context,NULL,atg_result_data(encoded),atg_result_len(encoded),2,options)"
        ));
        assert!(deploy.contains("atg_result_free(encoded); return response"));
        assert_eq!(
            deploy.contains("nonpayable constructor cannot receive value"),
            !payable
        );
        let cpp_source = cpp::render_cpp_file(&ir, true);
        assert!(cpp_source.contains("encode_constructor(const std::vector<uint8_t>& bytecode, const ConstructorParams& args)"));
        assert!(
            cpp_source.contains("inline std::vector<uint8_t> deploy(atg_transport_fn transport")
        );
    }
}

#[test]
fn event_filters_keep_topic_positions_and_anonymous_events_omit_topic_zero() {
    for anonymous in [false, true] {
        let ir = contract(
            json!([{ "type":"event","name":"Changed","anonymous":anonymous,"inputs":[
                {"name":"owner","type":"address","indexed":true}, {"name":"n","type":"uint256","indexed":false},
                {"name":"key","type":"string","indexed":true}
            ]}]),
        );
        let source = c::render_c_file(&ir, true);
        let filter = line(&source, "bool atg_Sample_atg_Changed_event_filter(");
        let start = usize::from(!anonymous);
        assert!(filter.contains(&format!("out->topic_count = {}", start + 2)));
        assert!(filter.contains(&format!(
            "if (topic0) {{ out->topics[{start}] = *topic0; out->has_topic[{start}] = 1; }}"
        )));
        assert!(filter.contains(&format!(
            "if (topic1) {{ out->topics[{}] = *topic1;",
            start + 1
        )));
        assert_eq!(
            source.contains("atg_Sample_atg_Changed_event_topic[32]"),
            !anonymous
        );
        assert_eq!(filter.contains("memcpy(out->topics[0].bytes"), !anonymous);
        assert!(source.contains("atg_decode_event(atg_Sample_abi,atg_Sample_atg_Changed_event_signature,topics,count,data,len)"));
    }
}

#[test]
fn filters_with_more_than_four_topics_fail_without_writing_topic_slots() {
    let inputs: Vec<_> = (0..4)
        .map(|i| json!({"name":format!("n{i}"),"type":"uint256","indexed":true}))
        .collect();
    let source = c::render_c_file(
        &contract(json!([{"type":"event","name":"Changed","inputs":inputs,"anonymous":false}])),
        true,
    );
    let filter = line(&source, "bool atg_Sample_atg_Changed_event_filter(");
    assert!(filter.contains("out->topic_count = 5; return false;"));
    assert!(!filter.contains("out->topics["));
}

#[test]
fn indexed_reference_types_become_hash_fields_with_original_event_signatures() {
    let ir = contract(
        json!([{ "type":"event","name":"Mixed","anonymous":false,"inputs":[
            {"name":"key","type":"tuple","indexed":true,"components":[{"name":"x","type":"uint256"}]},
            {"name":"values","type":"uint256[]","indexed":true}, {"name":"text","type":"string","indexed":true}
        ]}]),
    );
    let source = c::render_c_file(&ir, true);
    assert_eq!(source.matches("uint8_t bytes[32];").count(), 1);
    assert!(source.contains(
        "atg_Sample_atg_Mixed_event_signature[] = \"Mixed((uint256),uint256[],string)\""
    ));
    assert!(!source.contains("size_t len;"));
    assert!(
        cpp::render_cpp_file(&ir, true)
            .contains("atg_Mixed_event_decode(const std::vector<atg_word>& topics")
    );
}

#[test]
fn overloaded_and_colliding_function_families_match_between_c_and_cpp() {
    let ir = contract(json!([
        function("f", json!([]), json!([]), "view"),
        function("f", json!([{"name":"n","type":"uint256"}]), json!([]), "view"),
        function("f_encode", json!([]), json!([]), "view"),
        {"type":"error","name":"Denied","inputs":[]},
        {"type":"error","name":"Denied","inputs":[{"name":"n","type":"uint256"}]}
    ]));
    let c_source = c::render_c_file(&ir, true);
    let cpp_source = cpp::render_cpp_file(&ir, true);
    for name in ["atg_f", "atg_f2", "atg_f_encode2"] {
        assert!(c_source.contains(&format!("atg_Sample_{name}_signature[]")));
        assert!(cpp_source.contains(&format!("using {name}_params = ::atg_Sample_{name}_params")));
        assert!(cpp_source.contains(&format!("{name}_encode(const {name}_params& args)")));
    }
    for name in ["atg_Denied_error", "atg_Denied_error2"] {
        assert!(c_source.contains(&format!("atg_Sample_{name}_selector[4]")));
        assert!(cpp_source.contains(&format!("Decoded<{name}_fields> {name}_decode(")));
    }
}

#[test]
fn decoded_values_keep_owned_results_and_conversion_failures_free_them() {
    let ir = contract(json!([function(
        "read",
        json!([]),
        json!([{"name":"values","type":"string[]"}]),
        "view"
    )]));
    let c_source = c::render_c_file(&ir, true);
    let decode = line(&c_source, "*atg_Sample_atg_read_decode(");
    assert!(decode.contains("if (!out) return atg_result_failure(\"null output\")"));
    assert!(decode.contains(
        "atg_result_free(r); return atg_result_failure(\"decoded type conversion failed\")"
    ));
    let cpp_source = cpp::render_cpp_file(&ir, true);
    assert!(cpp_source.contains("std::unique_ptr<atg_result, decltype(&atg_result_free)>"));
    assert!(
        cpp_source
            .contains("Decoded(Decoded&&) = default; Decoded& operator=(Decoded&&) = default;")
    );
    assert!(cpp_source.contains("return {std::move(owner),value}"));
    assert!(cpp_source.contains("if (!n) return {}; return {p, p + n}"));
    assert!(
        cpp_source.contains("if (!transport) throw std::invalid_argument(\"missing transport\")")
    );
}
