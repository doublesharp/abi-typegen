use crate::codec::{self, Value};
use crate::*;
use alloy_primitives::{Address, B256, keccak256};
use std::{
    ffi::{CStr, CString},
    ptr, slice,
};

fn echo_abi(ty: &str) -> String {
    format!(
        r#"[{{"type":"function","name":"echo","inputs":[{{"name":"x","type":"{ty}"}}],"outputs":[{{"name":"x","type":"{ty}"}}],"stateMutability":"pure"}}]"#
    )
}

#[test]
fn external_function_values_encode_address_and_selector_as_bytes24() {
    let abi = echo_abi("function");
    let mut payload = Address::repeat_byte(0x12).to_vec();
    payload.extend_from_slice(&[0xde, 0xad, 0xbe, 0xef]);
    let mut expected = B256::ZERO;
    expected[..24].copy_from_slice(&payload);
    assert_eq!(
        codec::decode(&abi, "echo(function)", expected.as_slice()).expect("function output"),
        Value::Seq(vec![Value::Bytes(payload.clone())])
    );
    let encoded =
        codec::encode(&abi, "echo(function)", &[Value::Bytes(payload)]).expect("function input");
    assert_eq!(&encoded[..4], &keccak256("echo(function)")[..4]);
    assert_eq!(&encoded[4..], expected.as_slice());
}

#[test]
fn external_function_inputs_require_exactly_24_bytes() {
    let abi = echo_abi("function");
    for len in [0, 20, 23, 25, 32] {
        assert_eq!(
            codec::encode(&abi, "echo(function)", &[Value::Bytes(vec![0; len])])
                .expect_err("wrong function length")
                .to_string(),
            "value does not match function"
        );
    }
    assert!(codec::encode(&abi, "echo(function)", &[Value::Word(B256::ZERO)]).is_err());
}

#[test]
fn full_width_signed_integer_extremes_preserve_all_256_bits() {
    let abi = echo_abi("int256");
    let mut minimum = B256::ZERO;
    minimum[0] = 0x80;
    let mut maximum = B256::repeat_byte(0xff);
    maximum[0] = 0x7f;
    for expected in [minimum, maximum, B256::repeat_byte(0xff)] {
        assert_eq!(
            &codec::encode(&abi, "echo(int256)", &[Value::Word(expected)])
                .expect("full-width integer")[4..],
            expected.as_slice()
        );
        assert_eq!(
            codec::decode(&abi, "echo(int256)", expected.as_slice()).expect("full-width output"),
            Value::Seq(vec![Value::Word(expected)])
        );
    }
}

#[test]
fn indexed_topics_reject_boolean_address_and_function_padding() {
    for anonymous in [false, true] {
        for (ty, value) in [
            ("bool", Value::Bool(true)),
            (
                "address",
                Value::Word(Address::repeat_byte(0x12).into_word()),
            ),
            ("function", Value::Bytes(vec![0x12; 24])),
        ] {
            let abi = format!(
                r#"[{{"type":"event","name":"Changed","anonymous":{anonymous},"inputs":[{{"name":"x","type":"{ty}","indexed":true}}]}}]"#
            );
            let signature = format!("Changed({ty})");
            let mut topic = B256::ZERO;
            match ty {
                "bool" => topic[31] = 1,
                "address" => topic = Address::repeat_byte(0x12).into_word(),
                "function" => topic[..24].fill(0x12),
                _ => unreachable!("fixture type"),
            }
            let mut topics = if anonymous {
                vec![topic]
            } else {
                vec![keccak256(&signature), topic]
            };
            assert_eq!(
                codec::decode_event(&abi, &signature, &topics, &[]).expect("canonical topic"),
                Value::Seq(vec![value])
            );
            let last = topics.last_mut().expect("indexed topic");
            match ty {
                "bool" => last[31] = 2,
                "address" => last[0] = 1,
                "function" => last[31] = 1,
                _ => unreachable!("fixture type"),
            }
            assert_eq!(
                codec::decode_event(&abi, &signature, &topics, &[])
                    .expect_err("noncanonical indexed value")
                    .to_string(),
                "noncanonical ABI event data"
            );
        }
    }
}

#[test]
fn external_function_return_and_error_padding_is_rejected() {
    let mut word = B256::ZERO;
    word[..24].fill(0x12);
    let abi =
        r#"[{"type":"error","name":"Denied","inputs":[{"name":"callback","type":"function"}]}]"#;
    let mut error = keccak256("Denied(function)")[..4].to_vec();
    error.extend_from_slice(word.as_slice());
    assert_eq!(
        codec::decode_error(abi, "Denied(function)", &error).expect("function error"),
        Value::Seq(vec![Value::Bytes(vec![0x12; 24])])
    );
    word[31] = 1;
    error[35] = 1;
    assert_eq!(
        codec::decode(&echo_abi("function"), "echo(function)", word.as_slice())
            .expect_err("function return padding")
            .to_string(),
        "noncanonical ABI return data"
    );
    assert_eq!(
        codec::decode_error(abi, "Denied(function)", &error)
            .expect_err("function error padding")
            .to_string(),
        "noncanonical ABI error data"
    );
}

#[test]
fn ffi_external_function_arguments_match_the_abi_word() {
    let abi = CString::new(echo_abi("function")).expect("ABI");
    let mut expected = B256::ZERO;
    expected[..24].fill(0x12);
    // SAFETY: Input buffers remain live during the calls, runtime-owned values/results
    // are freed exactly once, and borrowed result bytes are read before freeing.
    unsafe {
        let args = atg_value_seq();
        let child = atg_value_bytes(expected.as_ptr(), 24);
        assert_eq!(atg_value_push(args, child), 0);
        atg_value_free(child);
        let result = atg_encode(abi.as_ptr(), c"echo(function)".as_ptr(), args);
        assert!(atg_result_error(result).is_null());
        assert_eq!(atg_result_len(result), 36);
        let encoded = slice::from_raw_parts(atg_result_data(result), atg_result_len(result));
        assert_eq!(&encoded[4..], expected.as_slice());
        atg_result_free(result);
        atg_value_free(args);
    }
}

#[test]
fn ffi_rejects_unrepresentable_slice_lengths_before_reading_buffers() {
    let byte = 0xab;
    let oversized = isize::MAX as usize + 1;
    // SAFETY: The real non-null pointer is never dereferenced: every entrypoint
    // rejects lengths above isize::MAX before constructing or reading a slice.
    // Error results are owned and freed once.
    unsafe {
        assert!(atg_value_bytes(&byte, oversized).is_null());
        for result in [
            atg_result_bytes(&byte, oversized),
            atg_decode(c"[]".as_ptr(), c"f()".as_ptr(), &byte, oversized),
            atg_decode_error(c"[]".as_ptr(), c"E()".as_ptr(), &byte, oversized),
            atg_decode_event(
                c"[]".as_ptr(),
                c"E()".as_ptr(),
                ptr::null(),
                0,
                &byte,
                oversized,
            ),
        ] {
            assert_eq!(
                CStr::from_ptr(atg_result_error(result))
                    .to_str()
                    .expect("error"),
                "invalid byte buffer"
            );
            assert!(atg_result_value(result).is_null());
            assert_eq!(atg_result_len(result), 0);
            atg_result_free(result);
        }
    }
}
