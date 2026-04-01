//! Fuzz target: parse an arbitrary artifact JSON, then run the full code-gen
//! pipeline over it.
//!
//! Ensures that every `ContractIr` that `parse_artifact` successfully produces
//! can be fed through `render_abi_file`, `render_viem_file`,
//! `render_ethers_file`, `render_zod_file`, and `render_barrel` without panicking. Exercises the
//! type mapper (`sol_type_to_ts`), JSDoc/NatSpec formatters, overload-index
//! logic, and reserved-word sanitisation across representative target configs.
#![no_main]

use libfuzzer_sys::fuzz_target;
use std::path::PathBuf;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(ir) = abi_typegen_core::parser::parse_artifact("FuzzContract", s) else {
        return;
    };

    // Exercise each renderer independently.
    let _ = abi_typegen_codegen::abi_writer::render_abi_file(&ir);
    let _ = abi_typegen_codegen::viem::render_viem_file(&ir);
    let _ = abi_typegen_codegen::ethers::render_ethers_file(&ir);
    let _ = abi_typegen_codegen::zod::render_zod_file(&ir);

    let base = abi_typegen_config::Config {
        artifacts_dir: PathBuf::from("out"),
        out_dir: PathBuf::from("src/generated"),
        target: abi_typegen_config::Target::Viem,
        wrappers: true,
        contracts: vec![],
        exclude: vec![],
    };
    let names = vec![ir.name.clone()];

    for target in [
        abi_typegen_config::Target::Viem,
        abi_typegen_config::Target::Zod,
        abi_typegen_config::Target::Ethers,
        abi_typegen_config::Target::Rust,
    ] {
        let config = abi_typegen_config::Config {
            target,
            ..base.clone()
        };
        let _ = abi_typegen_codegen::barrel::render_barrel(&names, &config);
        let _ = abi_typegen_codegen::generate_contract_files(&ir, &config);
    }
});
