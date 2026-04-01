//! Fuzz target: feed arbitrary bytes as JSON to `parse_artifact`.
//!
//! This exercises the full parsing pipeline: JSON deserialization, ABI item
//! dispatch, `parse_sol_type` recursion, and NatSpec extraction.
//!
//! Seed corpus lives in `fuzz/corpus/fuzz_parse_artifact/` (Foundry fixture
//! JSON files make excellent seeds).
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // The function must never panic — only return Err.
        let _ = abi_typegen_core::parser::parse_artifact("FuzzContract", s);
    }
});
