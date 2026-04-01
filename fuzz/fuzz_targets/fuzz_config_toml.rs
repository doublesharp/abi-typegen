//! Fuzz target: feed arbitrary bytes as TOML to `Config::from_toml_str`.
//!
//! Tests the TOML deserialization and config resolution logic, including the
//! custom `Target` deserializer and all default-value fallbacks.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Must never panic — only return Err.
        let _ = abi_typegen_config::Config::from_toml_str(s);
    }
});
