//! Fuzz target: feed arbitrary bytes as a Solidity type string to
//! `parse_type_string`.
//!
//! Targets `parse_sol_type` in isolation, focusing on the recursive array
//! suffix stripping, uint/int/bytesN bit-width parsing, and unknown-type error
//! paths.
//!
//! Seed corpus (`fuzz/corpus/fuzz_sol_type_str/`) contains representative
//! Solidity type strings so the fuzzer starts from a meaningful state.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Must never panic — only return Ok or Err.
        let _ = abi_typegen_core::parser::parse_type_string(s);
    }
});
