use crate::{
    codec::{self, Value},
    *,
};
use alloy_primitives::{Address, B256, I256, U256, keccak256};
use std::{
    ffi::{CStr, CString},
    ptr, slice,
};

fn word(n: u64) -> Value {
    Value::Word(B256::from(U256::from(n).to_be_bytes()))
}

fn echo_abi(ty: &str) -> String {
    format!(
        r#"[{{"type":"function","name":"echo","inputs":[{{"name":"x","type":"{ty}"}}],"outputs":[{{"name":"x","type":"{ty}"}}],"stateMutability":"pure"}}]"#
    )
}

#[test]
fn signed_integer_boundaries_use_full_sign_extension() {
    for (bits, values) in [(8, [-128_i64, -1, 0, 127]), (16, [-32768, -1, 0, 32767])] {
        let abi = echo_abi(&format!("int{bits}"));
        let signature = format!("echo(int{bits})");
        for n in values {
            let expected = B256::from(
                I256::try_from(n)
                    .expect("signed value")
                    .into_raw()
                    .to_be_bytes(),
            );
            let encoded =
                codec::encode(&abi, &signature, &[Value::Word(expected)]).expect("boundary");
            assert_eq!(&encoded[4..], expected.as_slice());
            assert_eq!(
                codec::decode(&abi, &signature, expected.as_slice()).expect("decode"),
                Value::Seq(vec![Value::Word(expected)])
            );
        }
        for n in [-(1_i64 << (bits - 1)) - 1, 1_i64 << (bits - 1)] {
            let value = Value::Word(B256::from(
                I256::try_from(n)
                    .expect("signed value")
                    .into_raw()
                    .to_be_bytes(),
            ));
            assert_eq!(
                codec::encode(&abi, &signature, &[value])
                    .expect_err("overflow")
                    .to_string(),
                "signed integer out of range"
            );
        }
    }
}

#[test]
fn fixed_bytes_are_left_aligned_and_require_exact_length() {
    let abi = echo_abi("bytes3");
    let value = Value::Bytes(vec![0x12, 0x34, 0x56]);
    let mut expected = vec![0; 32];
    expected[..3].copy_from_slice(&[0x12, 0x34, 0x56]);
    let encoded =
        codec::encode(&abi, "echo(bytes3)", std::slice::from_ref(&value)).expect("encode");
    assert_eq!(&encoded[4..], expected);
    assert_eq!(
        codec::decode(&abi, "echo(bytes3)", &expected).expect("decode"),
        Value::Seq(vec![value])
    );
    for len in [0, 2, 4, 32] {
        assert_eq!(
            codec::encode(&abi, "echo(bytes3)", &[Value::Bytes(vec![0; len])])
                .expect_err("length")
                .to_string(),
            "value does not match bytes3"
        );
    }
    expected[31] = 1;
    assert!(codec::decode(&abi, "echo(bytes3)", &expected).is_err());
}

#[test]
fn address_padding_is_checked_on_input_and_output() {
    let abi = echo_abi("address");
    let mut address = Address::repeat_byte(0xab).into_word();
    let encoded = codec::encode(&abi, "echo(address)", &[Value::Word(address)]).expect("address");
    assert_eq!(&encoded[4..], address.as_slice());
    assert_eq!(
        codec::decode(&abi, "echo(address)", address.as_slice()).expect("decode"),
        Value::Seq(vec![Value::Word(address)])
    );
    address[0] = 1;
    assert_eq!(
        codec::encode(&abi, "echo(address)", &[Value::Word(address)])
            .expect_err("padding")
            .to_string(),
        "address exceeds 20 bytes"
    );
    assert!(codec::decode(&abi, "echo(address)", address.as_slice()).is_err());
}

#[test]
fn dynamic_bytes_and_strings_match_independent_abi_layout() {
    for ty in ["bytes", "string"] {
        let abi = echo_abi(ty);
        let signature = format!("echo({ty})");
        for payload in [
            vec![],
            b"hello".to_vec(),
            "hello 🌍".as_bytes().to_vec(),
            vec![b'a'; 33],
        ] {
            // ABI head points to the length word followed by right-padded payload bytes.
            let mut expected = vec![0; 64 + payload.len().div_ceil(32) * 32];
            expected[31] = 32;
            expected[63] = payload.len() as u8;
            expected[64..64 + payload.len()].copy_from_slice(&payload);
            let encoded =
                codec::encode(&abi, &signature, &[Value::Bytes(payload.clone())]).expect("encode");
            assert_eq!(&encoded[4..], expected);
            assert_eq!(
                codec::decode(&abi, &signature, &expected).expect("decode"),
                Value::Seq(vec![Value::Bytes(payload)])
            );
        }
    }
    assert!(
        codec::encode(
            &echo_abi("string"),
            "echo(string)",
            &[Value::Bytes(vec![0xff])]
        )
        .is_err()
    );
    assert!(
        codec::encode(
            &echo_abi("bytes"),
            "echo(bytes)",
            &[Value::Bytes(vec![0xff])]
        )
        .is_ok()
    );
}

#[test]
fn fixed_arrays_check_length_and_each_element() {
    let abi = echo_abi("uint8[2]");
    let value = Value::Seq(vec![word(0), word(255)]);
    let mut expected = vec![0; 64];
    expected[63] = 255;
    assert_eq!(
        &codec::encode(&abi, "echo(uint8[2])", std::slice::from_ref(&value)).expect("encode")[4..],
        expected
    );
    assert_eq!(
        codec::decode(&abi, "echo(uint8[2])", &expected).expect("decode"),
        Value::Seq(vec![value])
    );
    for children in [
        vec![],
        vec![word(1)],
        vec![word(1), word(2), word(3)],
        vec![word(1), word(256)],
    ] {
        assert!(codec::encode(&abi, "echo(uint8[2])", &[Value::Seq(children)]).is_err());
    }
}

#[test]
fn tuple_arity_and_value_kinds_are_checked() {
    let abi = r#"[{"type":"function","name":"f","inputs":[{"name":"x","type":"tuple","components":[{"name":"n","type":"uint8"},{"name":"flag","type":"bool"}]}],"outputs":[],"stateMutability":"pure"}]"#;
    assert_eq!(
        codec::encode(
            abi,
            "f((uint8,bool))",
            &[Value::Seq(vec![word(7), Value::Bool(true)])]
        )
        .expect("tuple")
        .len(),
        68
    );
    for value in [
        Value::Bool(true),
        Value::Seq(vec![word(7)]),
        Value::Seq(vec![word(7), word(1)]),
        Value::Seq(vec![word(7), Value::Bool(true), word(9)]),
    ] {
        assert!(codec::encode(abi, "f((uint8,bool))", &[value]).is_err());
    }
}

#[test]
fn malformed_dynamic_returns_are_rejected() {
    let abi = echo_abi("bytes");
    let mut canonical = vec![0; 96];
    canonical[31] = 32;
    canonical[63] = 1;
    canonical[64] = 0xab;
    assert_eq!(
        codec::decode(&abi, "echo(bytes)", &canonical).expect("canonical"),
        Value::Seq(vec![Value::Bytes(vec![0xab])])
    );
    for (offset, byte) in [(31, 0), (31, 33), (31, 96), (63, 33), (95, 1)] {
        let mut malformed = canonical.clone();
        malformed[offset] = byte;
        assert!(
            codec::decode(&abi, "echo(bytes)", &malformed).is_err(),
            "offset {offset}, byte {byte}"
        );
    }
    for len in [0, 31, 32, 63, 64, 95] {
        assert!(
            codec::decode(&abi, "echo(bytes)", &canonical[..len]).is_err(),
            "truncated at {len}"
        );
    }
}

#[test]
fn decoded_integers_must_fit_the_declared_width() {
    for (ty, invalid) in [
        ("uint8", word(256)),
        ("int8", word(128)),
        (
            "int8",
            Value::Word(B256::from(
                I256::try_from(-129)
                    .expect("signed value")
                    .into_raw()
                    .to_be_bytes(),
            )),
        ),
    ] {
        let Value::Word(invalid) = invalid else {
            unreachable!("word fixture")
        };
        assert!(
            codec::decode(&echo_abi(ty), &format!("echo({ty})"), invalid.as_slice()).is_err(),
            "accepted out-of-range {ty}"
        );
    }
}

#[test]
fn nested_fixed_bytes_padding_is_rejected_in_return_values() {
    let abi = r#"[{"type":"function","name":"f","inputs":[],"outputs":[{"name":"x","type":"tuple","components":[{"name":"keys","type":"bytes3[2]"}]}],"stateMutability":"view"}]"#;
    let mut body = vec![0; 64];
    body[0..3].copy_from_slice(&[1, 2, 3]);
    body[32..35].copy_from_slice(&[4, 5, 6]);
    assert_eq!(
        codec::decode(abi, "f()", &body).expect("valid tuple"),
        Value::Seq(vec![Value::Seq(vec![Value::Seq(vec![
            Value::Bytes(vec![1, 2, 3]),
            Value::Bytes(vec![4, 5, 6])
        ])])])
    );
    body[63] = 1;
    assert!(codec::decode(abi, "f()", &body).is_err());
}

#[test]
fn error_integer_widths_and_fixed_bytes_padding_are_rejected() {
    for (ty, signature, mut body) in [
        (
            "uint8",
            "Denied(uint8)",
            B256::from(U256::from(256).to_be_bytes()),
        ),
        ("bytes3", "Denied(bytes3)", B256::ZERO),
    ] {
        if ty == "bytes3" {
            body[31] = 1;
        }
        let abi = format!(
            r#"[{{"type":"error","name":"Denied","inputs":[{{"name":"x","type":"{ty}"}}]}}]"#
        );
        let mut data = keccak256(signature)[..4].to_vec();
        data.extend_from_slice(body.as_slice());
        assert!(
            codec::decode_error(&abi, signature, &data).is_err(),
            "accepted invalid {ty} error"
        );
    }
}

#[test]
fn event_integer_widths_and_fixed_bytes_padding_are_rejected() {
    for indexed in [false, true] {
        for (ty, signature, mut body) in [
            (
                "uint8",
                "Changed(uint8)",
                B256::from(U256::from(256).to_be_bytes()),
            ),
            ("bytes3", "Changed(bytes3)", B256::ZERO),
        ] {
            if ty == "bytes3" {
                body[31] = 1;
            }
            let abi = format!(
                r#"[{{"type":"event","name":"Changed","inputs":[{{"name":"x","type":"{ty}","indexed":{indexed}}}],"anonymous":false}}]"#
            );
            let mut topics = vec![keccak256(signature)];
            let data = if indexed {
                topics.push(body);
                &[][..]
            } else {
                body.as_slice()
            };
            assert!(
                codec::decode_event(&abi, signature, &topics, data).is_err(),
                "accepted invalid {ty}, indexed={indexed}"
            );
        }
    }
}

#[test]
fn unknown_signatures_and_wrong_argument_counts_report_specific_errors() {
    let abi = echo_abi("bool");
    assert_eq!(
        codec::encode(&abi, "echo(uint256)", &[word(1)])
            .expect_err("signature")
            .to_string(),
        "unknown function signature"
    );
    assert_eq!(
        codec::decode(&abi, "missing()", &[])
            .expect_err("signature")
            .to_string(),
        "unknown function signature"
    );
    for args in [vec![], vec![Value::Bool(true), Value::Bool(false)]] {
        assert_eq!(
            codec::encode(&abi, "echo(bool)", &args)
                .expect_err("arity")
                .to_string(),
            "argument count mismatch"
        );
    }
    assert!(codec::encode("not json", "echo(bool)", &[Value::Bool(true)]).is_err());
}

#[test]
fn custom_errors_reject_truncation_padding_and_trailing_data() {
    let abi = r#"[{"type":"error","name":"Error","inputs":[{"name":"message","type":"string"}]}]"#;
    let mut expected = vec![0x08, 0xc3, 0x79, 0xa0];
    let mut body = vec![0; 96];
    body[31] = 32;
    body[63] = 2;
    body[64..66].copy_from_slice(b"no");
    expected.extend(body);
    assert_eq!(
        codec::decode_error(abi, "Error(string)", &expected).expect("error"),
        Value::Seq(vec![Value::Bytes(b"no".to_vec())])
    );
    for len in [0, 3, 4, 99] {
        assert!(codec::decode_error(abi, "Error(string)", &expected[..len]).is_err());
    }
    let mut malformed = expected.clone();
    malformed[99] = 1;
    assert!(codec::decode_error(abi, "Error(string)", &malformed).is_err());
    expected.push(0);
    assert!(codec::decode_error(abi, "Error(string)", &expected).is_err());
    assert_eq!(
        codec::decode_error(abi, "Missing()", &[])
            .expect_err("signature")
            .to_string(),
        "unknown error signature"
    );
}

#[test]
fn events_preserve_interleaved_field_order_and_anonymous_topics() {
    for anonymous in [false, true] {
        let abi = format!(
            r#"[{{"type":"event","name":"Changed","anonymous":{anonymous},"inputs":[{{"name":"count","type":"uint8","indexed":false}},{{"name":"owner","type":"address","indexed":true}},{{"name":"flag","type":"bool","indexed":false}},{{"name":"key","type":"string","indexed":true}}]}}]"#
        );
        let owner = Address::repeat_byte(0x12).into_word();
        let hash = keccak256("key");
        let mut topics = vec![owner, hash];
        if !anonymous {
            topics.insert(0, keccak256("Changed(uint8,address,bool,string)"));
        }
        let mut data = vec![0; 64];
        data[31] = 7;
        data[63] = 1;
        assert_eq!(
            codec::decode_event(&abi, "Changed(uint8,address,bool,string)", &topics, &data)
                .expect("event"),
            Value::Seq(vec![
                word(7),
                Value::Word(owner),
                Value::Bool(true),
                Value::Bytes(hash.to_vec())
            ])
        );
        assert!(
            codec::decode_event(
                &abi,
                "Changed(uint8,address,bool,string)",
                &topics[..topics.len() - 1],
                &data
            )
            .is_err()
        );
        let mut extra_topics = topics.clone();
        extra_topics.push(B256::ZERO);
        assert!(
            codec::decode_event(
                &abi,
                "Changed(uint8,address,bool,string)",
                &extra_topics,
                &data
            )
            .is_err()
        );
        data.push(0);
        assert!(
            codec::decode_event(&abi, "Changed(uint8,address,bool,string)", &topics, &data)
                .is_err()
        );
        assert_eq!(
            codec::decode_event(&abi, "Missing()", &[], &[])
                .expect_err("signature")
                .to_string(),
            "unknown event signature"
        );
    }
}

#[test]
fn constructors_without_arguments_preserve_bytecode_exactly() {
    for abi in [
        "[]",
        r#"[{"type":"constructor","inputs":[],"stateMutability":"nonpayable"}]"#,
    ] {
        let bytecode = [0x60, 0x00, 0x60, 0x01];
        assert_eq!(
            codec::encode_constructor(abi, &bytecode, &[]).expect("deployment"),
            bytecode
        );
        assert!(codec::encode_constructor(abi, &bytecode, &[word(1)]).is_err());
    }
}

#[test]
fn ffi_values_copy_buffers_and_clone_children() {
    // SAFETY: Buffers remain readable for calls. Values are owned and freed once;
    // borrowed children are inspected only while their parent is live.
    unsafe {
        let mut bytes = [1, 2, 3];
        let data = atg_value_bytes(bytes.as_ptr(), bytes.len());
        bytes.fill(9);
        assert_eq!(atg_value_kind(data), 3);
        assert_eq!(
            slice::from_raw_parts(atg_value_data(data), atg_value_len(data)),
            &[1, 2, 3]
        );
        let mut source = B256::repeat_byte(0xab);
        let value = atg_value_word(source.as_ptr());
        source[0] = 0;
        assert_eq!(atg_value_kind(value), 2);
        assert_eq!(
            slice::from_raw_parts(atg_value_data(value), atg_value_len(value)),
            B256::repeat_byte(0xab).as_slice()
        );
        let seq = atg_value_seq();
        assert_eq!(atg_value_push(seq, data), 0);
        assert_eq!(atg_value_push(seq, value), 0);
        atg_value_free(data);
        atg_value_free(value);
        assert_eq!(atg_value_kind(seq), 4);
        assert_eq!(atg_value_len(seq), 2);
        assert_eq!(atg_value_kind(atg_value_at(seq, 0)), 3);
        assert_eq!(*atg_value_data(atg_value_at(seq, 1)), 0xab);
        assert!(atg_value_at(seq, 2).is_null());
        atg_value_free(seq);
    }
}

#[test]
fn ffi_null_and_wrong_kind_accessors_return_documented_defaults() {
    // SAFETY: Null pointers are supported by these accessors; the boolean is live until freed.
    unsafe {
        let null = ptr::null();
        assert_eq!(atg_value_kind(null), 0);
        assert_eq!(atg_value_get_bool(null), 0);
        assert_eq!(atg_value_len(null), 0);
        assert!(atg_value_data(null).is_null());
        assert!(atg_value_at(null, 0).is_null());
        assert!(atg_value_word(ptr::null()).is_null());
        assert!(atg_value_bytes(ptr::null(), 1).is_null());
        let empty = atg_value_bytes(ptr::null(), 0);
        assert!(!empty.is_null());
        assert_eq!(atg_value_len(empty), 0);
        atg_value_free(empty);
        let boolean = atg_value_bool(-1);
        assert_eq!(atg_value_kind(boolean), 1);
        assert_eq!(atg_value_get_bool(boolean), 1);
        assert!(atg_value_data(boolean).is_null());
        assert!(atg_value_at(boolean, 0).is_null());
        assert_eq!(atg_value_len(boolean), 0);
        assert_eq!(atg_value_push(boolean, boolean), -1);
        assert_eq!(atg_value_push(ptr::null_mut(), boolean), -1);
        assert_eq!(atg_value_push(boolean, ptr::null()), -1);
        atg_value_free(boolean);
        assert!(atg_result_data(ptr::null()).is_null());
        assert!(atg_result_value(ptr::null()).is_null());
        assert_eq!(atg_result_len(ptr::null()), 0);
        assert_eq!(
            CStr::from_ptr(atg_result_error(ptr::null()))
                .to_str()
                .expect("error"),
            "null runtime result"
        );
        assert!(atg_result_alloc(ptr::null_mut(), 16, 8).is_null());
        atg_value_free(ptr::null_mut());
    }
}

#[test]
fn ffi_transport_results_copy_bytes_and_diagnostics() {
    // SAFETY: Input buffers are live during copying; borrowed storage is read before freeing results.
    unsafe {
        let mut bytes = [0xde, 0xad, 0xbe, 0xef];
        let result = atg_result_bytes(bytes.as_ptr(), bytes.len());
        bytes.fill(0);
        assert!(atg_result_error(result).is_null());
        assert!(atg_result_value(result).is_null());
        assert_eq!(
            slice::from_raw_parts(atg_result_data(result), atg_result_len(result)),
            &[0xde, 0xad, 0xbe, 0xef]
        );
        atg_result_free(result);
        let message = CString::new("transport failed").expect("message");
        let failure = atg_result_failure(message.as_ptr());
        drop(message);
        assert_eq!(
            CStr::from_ptr(atg_result_error(failure))
                .to_str()
                .expect("error"),
            "transport failed"
        );
        assert_eq!(atg_result_len(failure), 0);
        assert!(atg_result_value(failure).is_null());
        atg_result_free(failure);
        let invalid = atg_result_bytes(ptr::null(), 1);
        assert_eq!(
            CStr::from_ptr(atg_result_error(invalid))
                .to_str()
                .expect("error"),
            "invalid byte buffer"
        );
        atg_result_free(invalid);
    }
}

#[test]
fn ffi_scratch_storage_is_zeroed_aligned_and_rejects_bad_layouts() {
    // SAFETY: The result owns all scratch allocations. Reads use the requested allocation size.
    unsafe {
        let result = atg_result_bytes(ptr::null(), 0);
        for (size, alignment) in [(0, 1), (17, 8), (64, 64)] {
            let allocation = atg_result_alloc(result, size, alignment).cast::<u8>();
            assert!(!allocation.is_null());
            assert_eq!((allocation as usize) % alignment, 0);
            assert!(
                slice::from_raw_parts(allocation, size.max(1))
                    .iter()
                    .all(|b| *b == 0)
            );
        }
        for alignment in [0, 3, 7] {
            assert!(atg_result_alloc(result, 16, alignment).is_null());
        }
        assert!(atg_result_alloc(result, usize::MAX, 8).is_null());
        atg_result_free(result);
    }
}

#[test]
fn ffi_decodes_errors_events_and_constructor_arguments() {
    let abi = c"[{\"type\":\"error\",\"name\":\"Denied\",\"inputs\":[{\"name\":\"code\",\"type\":\"uint256\"}]},{\"type\":\"event\",\"name\":\"Changed\",\"inputs\":[{\"name\":\"code\",\"type\":\"uint256\",\"indexed\":false}],\"anonymous\":false},{\"type\":\"constructor\",\"inputs\":[{\"name\":\"code\",\"type\":\"uint256\"}],\"stateMutability\":\"nonpayable\"}]";
    let word = B256::from(U256::from(7).to_be_bytes());
    let mut error_bytes = keccak256("Denied(uint256)")[..4].to_vec();
    error_bytes.extend_from_slice(word.as_slice());
    let topic = keccak256("Changed(uint256)");
    // SAFETY: All C strings and buffers are live. Values/results are freed once after borrowed reads.
    unsafe {
        for result in [
            atg_decode_error(
                abi.as_ptr(),
                c"Denied(uint256)".as_ptr(),
                error_bytes.as_ptr(),
                error_bytes.len(),
            ),
            atg_decode_event(
                abi.as_ptr(),
                c"Changed(uint256)".as_ptr(),
                topic.as_ptr(),
                1,
                word.as_ptr(),
                32,
            ),
        ] {
            assert!(atg_result_error(result).is_null());
            let value = atg_result_value(result);
            assert_eq!(atg_value_len(value), 1);
            let child = atg_value_at(value, 0);
            assert_eq!(
                slice::from_raw_parts(atg_value_data(child), atg_value_len(child)),
                word.as_slice()
            );
            atg_result_free(result);
        }
        let args = atg_value_seq();
        let child = atg_value_word(word.as_ptr());
        assert_eq!(atg_value_push(args, child), 0);
        atg_value_free(child);
        let bytecode = [0x60, 0x00];
        let result = atg_encode_constructor(abi.as_ptr(), bytecode.as_ptr(), bytecode.len(), args);
        assert!(atg_result_error(result).is_null());
        let mut expected = bytecode.to_vec();
        expected.extend_from_slice(word.as_slice());
        assert_eq!(
            slice::from_raw_parts(atg_result_data(result), atg_result_len(result)),
            expected
        );
        atg_result_free(result);
        atg_value_free(args);
    }
}

#[test]
fn ffi_reports_validation_failures_without_decoded_values() {
    // SAFETY: Strings are live, null buffers are rejected before use, and results are freed once.
    unsafe {
        for (result, expected) in [
            (
                atg_encode(c"[]".as_ptr(), c"f()".as_ptr(), ptr::null()),
                "arguments must be a sequence",
            ),
            (
                atg_encode_constructor(c"[]".as_ptr(), ptr::null(), 0, ptr::null()),
                "constructor arguments must be a sequence",
            ),
            (
                atg_decode(c"[]".as_ptr(), c"f()".as_ptr(), ptr::null(), 1),
                "invalid byte buffer",
            ),
            (
                atg_decode_event(
                    c"[]".as_ptr(),
                    c"E()".as_ptr(),
                    ptr::null(),
                    5,
                    ptr::null(),
                    0,
                ),
                "event has more than four topics",
            ),
            (
                atg_decode_event(
                    c"[]".as_ptr(),
                    c"E()".as_ptr(),
                    ptr::null(),
                    1,
                    ptr::null(),
                    0,
                ),
                "invalid byte buffer",
            ),
        ] {
            assert_eq!(
                CStr::from_ptr(atg_result_error(result))
                    .to_str()
                    .expect("error"),
                expected
            );
            assert!(atg_result_value(result).is_null());
            assert_eq!(atg_result_len(result), 0);
            atg_result_free(result);
        }
    }
}

#[test]
fn encoding_checks_the_nested_array_depth_boundary() {
    for depth in [64, 65] {
        let ty = format!("uint256{}", "[]".repeat(depth));
        let abi = echo_abi(&ty);
        let signature = format!("echo({ty})");
        let mut value = word(7);
        for _ in 0..depth {
            value = Value::Seq(vec![value]);
        }
        let result = codec::encode(&abi, &signature, std::slice::from_ref(&value));
        if depth == 64 {
            let encoded = result.expect("depth 64");
            assert_eq!(encoded.len(), 4 + 32 * (2 * depth + 1));
            assert_eq!(&encoded[4..36], U256::from(32).to_be_bytes::<32>());
            assert_eq!(
                &encoded[encoded.len() - 32..],
                U256::from(7).to_be_bytes::<32>()
            );
        } else {
            assert_eq!(
                result.expect_err("depth 65").to_string(),
                "ABI nesting exceeds 64"
            );
        }
    }
}
