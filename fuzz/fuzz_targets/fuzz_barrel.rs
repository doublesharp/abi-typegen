//! Fuzz target: feed an arbitrary list of contract name strings to
//! `render_barrel`.
//!
//! Exercises the barrel/index generator's name handling, heck case conversion,
//! and TypeScript export formatting across representative target configurations.
//! Interesting edge cases: empty names, names with Unicode, names that are
//! JS reserved words, very long names, names with leading digits.
#![no_main]

use libfuzzer_sys::fuzz_target;
use std::path::PathBuf;

fuzz_target!(|names: Vec<String>| {
    let base = abi_typegen_config::Config {
        artifacts_dir: PathBuf::from("out"),
        out_dir: PathBuf::from("src/generated"),
        target: abi_typegen_config::Target::Viem,
        wrappers: true,
        contracts: vec![],
        exclude: vec![],
    };

    for target in [
        abi_typegen_config::Target::Viem,
        abi_typegen_config::Target::Zod,
        abi_typegen_config::Target::Ethers,
        abi_typegen_config::Target::Wagmi,
    ] {
        let config = abi_typegen_config::Config {
            target,
            ..base.clone()
        };
        // Must never panic regardless of what names contains.
        let _ = abi_typegen_codegen::barrel::render_barrel(&names, &config);
    }
});
