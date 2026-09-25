//! ABI encoding and decoding shared by generated C and C++ bindings.

mod codec;
mod ffi;
pub use ffi::*;

#[cfg(test)]
mod tests {
    use crate::codec::{Value, decode, encode};
    use alloy_primitives::{B256, U256};
    const ABI: &str = r#"[{"type":"function","name":"transfer","inputs":[{"name":"to","type":"address"},{"name":"amount","type":"uint256"}],"outputs":[{"name":"ok","type":"bool"}],"stateMutability":"nonpayable"}]"#;
    #[test]
    fn transfer_matches_ethereum_calldata() {
        let mut address = B256::ZERO;
        address[31] = 1;
        let encoded = encode(
            ABI,
            "transfer(address,uint256)",
            &[
                Value::Word(address),
                Value::Word(B256::from(U256::from(42).to_be_bytes())),
            ],
        )
        .expect("encode");
        assert_eq!(&encoded[..4], &[0xa9, 0x05, 0x9c, 0xbb]);
        assert_eq!(encoded.len(), 68);
        assert_eq!(encoded[35], 1);
        assert_eq!(encoded[67], 42);
    }
    #[test]
    fn decoder_rejects_noncanonical_boolean_and_trailing_bytes() {
        let mut bytes = vec![0; 32];
        bytes[31] = 2;
        assert!(decode(ABI, "transfer(address,uint256)", &bytes).is_err());
        bytes[31] = 1;
        assert_eq!(
            decode(ABI, "transfer(address,uint256)", &bytes).expect("decode"),
            Value::Seq(vec![Value::Bool(true)])
        );
        bytes.push(0);
        assert!(decode(ABI, "transfer(address,uint256)", &bytes).is_err());
    }
    #[test]
    fn integer_width_is_validated_before_encoding() {
        let abi = ABI.replace("uint256", "uint8");
        let args = [
            Value::Word(B256::ZERO),
            Value::Word(B256::from(U256::from(256).to_be_bytes())),
        ];
        assert!(encode(&abi, "transfer(address,uint8)", &args).is_err());
    }
}

#[cfg(test)]
mod ffi_tests {
    use super::*;
    use std::ffi::{CStr, CString};
    #[test]
    fn ffi_ownership_and_null_errors() {
        // SAFETY: All pointers below originate from runtime constructors and are freed exactly once.
        unsafe {
            let args = atg_value_seq();
            let child = atg_value_bool(1);
            assert_eq!(atg_value_push(args, child), 0);
            atg_value_free(child);
            assert_eq!(atg_value_get_bool(atg_value_at(args, 0)), 1);
            assert!(atg_value_at(args, 1).is_null());
            let abi = CString::new(r#"[{"type":"function","name":"f","inputs":[{"name":"x","type":"bool"}],"outputs":[],"stateMutability":"view"}]"#).expect("cstring");
            let result = atg_encode(abi.as_ptr(), c"f(bool)".as_ptr(), args);
            assert!(atg_result_error(result).is_null());
            assert_eq!(atg_result_len(result), 36);
            let scratch = atg_result_alloc(result, 100, 16);
            assert!(!scratch.is_null());
            assert_eq!((scratch as usize) % 16, 0);
            atg_result_free(result);
            atg_value_free(args);
            let failure = atg_decode(std::ptr::null(), c"f()".as_ptr(), std::ptr::null(), 0);
            assert_eq!(
                CStr::from_ptr(atg_result_error(failure))
                    .to_str()
                    .expect("utf8"),
                "null string"
            );
            atg_result_free(failure);
            atg_result_free(std::ptr::null_mut());
        }
    }
}

#[cfg(test)]
mod codec_cases {
    use crate::codec::{self, Value};
    use alloy_primitives::{B256, U256, keccak256};
    #[test]
    fn nested_arrays_and_tuple_outputs_roundtrip() {
        let abi = r#"[{"type":"function","name":"echo","inputs":[{"name":"x","type":"tuple","components":[{"name":"grid","type":"uint256[][]"},{"name":"text","type":"string"}]}],"outputs":[{"name":"x","type":"tuple","components":[{"name":"grid","type":"uint256[][]"},{"name":"text","type":"string"}]}],"stateMutability":"pure"}]"#;
        let value = Value::Seq(vec![
            Value::Seq(vec![
                Value::Seq(vec![Value::Word(B256::from(U256::MAX.to_be_bytes()))]),
                Value::Seq(vec![]),
            ]),
            Value::Bytes("hello 🌍".as_bytes().to_vec()),
        ]);
        let encoded = codec::encode(
            abi,
            "echo((uint256[][],string))",
            std::slice::from_ref(&value),
        )
        .expect("encode");
        assert_eq!(
            codec::decode(abi, "echo((uint256[][],string))", &encoded[4..]).expect("decode"),
            Value::Seq(vec![value])
        );
    }
    #[test]
    fn custom_errors_and_indexed_events_are_checked() {
        let abi = r#"[{"type":"error","name":"Denied","inputs":[{"name":"code","type":"uint256"}]},{"type":"event","name":"Message","inputs":[{"name":"text","type":"string","indexed":true},{"name":"code","type":"uint256","indexed":false}],"anonymous":false}]"#;
        let word = B256::from(U256::from(7).to_be_bytes());
        let mut bytes = keccak256("Denied(uint256)")[..4].to_vec();
        bytes.extend_from_slice(word.as_slice());
        assert_eq!(
            codec::decode_error(abi, "Denied(uint256)", &bytes).expect("error"),
            Value::Seq(vec![Value::Word(word)])
        );
        bytes[0] ^= 1;
        assert!(codec::decode_error(abi, "Denied(uint256)", &bytes).is_err());
        let hash = keccak256("hello");
        let topics = [keccak256("Message(string,uint256)"), hash];
        assert_eq!(
            codec::decode_event(abi, "Message(string,uint256)", &topics, word.as_slice())
                .expect("event"),
            Value::Seq(vec![Value::Bytes(hash.to_vec()), Value::Word(word)])
        );
        assert!(
            codec::decode_event(
                abi,
                "Message(string,uint256)",
                &topics[..1],
                word.as_slice()
            )
            .is_err()
        );
    }
}

#[cfg(test)]
mod constructor_tests {
    use crate::codec::{Value, encode_constructor};
    use alloy_primitives::{B256, U256};
    #[test]
    fn deployment_has_no_function_selector() {
        let abi = r#"[{"type":"constructor","inputs":[{"name":"x","type":"uint256"}],"stateMutability":"nonpayable"}]"#;
        let args = [Value::Word(B256::from(U256::from(42).to_be_bytes()))];
        let encoded = encode_constructor(abi, &[0x60, 0x00], &args).expect("constructor");
        assert_eq!(encoded.len(), 34);
        assert_eq!(&encoded[..2], &[0x60, 0x00]);
        assert_eq!(encoded[33], 42);
        assert!(encode_constructor(abi, &[], &args).is_err());
        assert!(encode_constructor(abi, &[0x60], &[]).is_err());
    }
}
