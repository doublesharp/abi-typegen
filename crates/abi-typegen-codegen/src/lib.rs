//! Code generation from Solidity ABI intermediate representation.

pub mod abi_writer;
pub mod barrel;
pub mod renderers;
pub mod type_mapper;

// Re-export renderers at crate root for convenience.
pub use renderers::csharp;
pub use renderers::ethers5;
pub use renderers::ethers6 as ethers;
pub use renderers::go;
pub use renderers::kotlin;
pub use renderers::python;
pub use renderers::rust;
pub use renderers::solidity;
pub use renderers::swift;
pub use renderers::viem;
pub use renderers::wagmi;
pub use renderers::web3js;
pub use renderers::yaml;
pub use renderers::zod;

use abi_typegen_config::{Config, Target};
use abi_typegen_core::types::ContractIr;
use std::collections::HashMap;

/// Generates output files for a single contract based on the configured target.
///
/// Returns a map of filename (e.g. `"ERC20.abi.ts"` or `"IERC20.sol"`) to file content.
pub fn generate_contract_files(ir: &ContractIr, config: &Config) -> HashMap<String, String> {
    let mut files = HashMap::new();

    // TypeScript targets emit an as-const ABI file
    if config.target().emits_typescript_abi() {
        files.insert(
            format!("{}.abi.ts", ir.name),
            abi_writer::render_abi_file(ir),
        );
    }

    match *config.target() {
        Target::Viem => {
            if config.wrappers {
                files.insert(format!("{}.viem.ts", ir.name), viem::render_viem_file(ir));
            }
        }
        Target::Zod => {
            files.insert(format!("{}.zod.ts", ir.name), zod::render_zod_file(ir));
        }
        Target::Wagmi => {
            if config.wrappers {
                files.insert(
                    format!("{}.wagmi.ts", ir.name),
                    wagmi::render_wagmi_file(ir),
                );
            }
        }
        Target::Ethers => {
            if config.wrappers {
                files.insert(
                    format!("{}.ethers.ts", ir.name),
                    ethers::render_ethers_file(ir),
                );
            }
        }
        Target::Ethers5 => {
            if config.wrappers {
                files.insert(
                    format!("{}.ethers5.ts", ir.name),
                    ethers5::render_ethers5_file(ir),
                );
            }
        }
        Target::Web3js => {
            if config.wrappers {
                files.insert(
                    format!("{}.web3.ts", ir.name),
                    web3js::render_web3js_file(ir),
                );
            }
        }
        Target::Python => {
            files.insert(format!("{}.py", ir.name), python::render_python_file(ir));
        }
        Target::Go => {
            files.insert(format!("{}.go", ir.name), go::render_go_file(ir));
        }
        Target::Rust => {
            files.insert(format!("{}.rs", ir.name), rust::render_rust_file(ir));
        }
        Target::Swift => {
            files.insert(format!("{}.swift", ir.name), swift::render_swift_file(ir));
        }
        Target::CSharp => {
            files.insert(format!("{}.cs", ir.name), csharp::render_csharp_file(ir));
        }
        Target::Kotlin => {
            files.insert(format!("{}.kt", ir.name), kotlin::render_kotlin_file(ir));
        }
        Target::Solidity => {
            let interface_name = solidity::interface_name(&ir.name);
            files.insert(
                format!("{}.sol", interface_name),
                solidity::render_solidity_file(ir),
            );
        }
        Target::Yaml => {
            files.insert(format!("{}.yaml", ir.name), yaml::render_yaml_file(ir));
        }
    }

    files
}
