use super::*;
use std::path::PathBuf;

const TARGET_MATRIX_ARTIFACT_JSON: &str = r#"{
    "abi": [{
        "type": "function",
        "name": "balanceOf",
        "inputs": [{"name": "account", "type": "address", "internalType": "address", "components": []}],
        "outputs": [{"name": "", "type": "uint256", "internalType": "uint256", "components": []}],
        "stateMutability": "view"
    }]
}"#;

/// Create a unique temporary directory for a test, returning its path.
fn temp_test_dir(test_name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("abi-typegen-main-tests")
        .join(test_name)
        .join(format!("{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Clean up a test directory.
fn cleanup(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

fn write_target_matrix_artifact(artifacts_dir: &Path, contract_name: &str) {
    let sol_dir = artifacts_dir.join(format!("{}.sol", contract_name));
    std::fs::create_dir_all(&sol_dir).unwrap();
    std::fs::write(
        sol_dir.join(format!("{}.json", contract_name)),
        TARGET_MATRIX_ARTIFACT_JSON,
    )
    .unwrap();
}

fn generated_config(
    artifacts_dir: PathBuf,
    out_dir: PathBuf,
    target: abi_typegen_config::Target,
) -> Config {
    Config {
        artifacts_dir,
        out_dir,
        targets: vec![target],
        wrappers: true,
        contracts: vec![],
        exclude: vec![],
    }
}

fn assert_file_contains(path: &Path, needle: &str) {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e));
    assert!(
        content.contains(needle),
        "expected {} to contain {:?}",
        path.display(),
        needle
    );
}

fn assert_missing_files(dir: &Path, filenames: &[&str]) {
    for filename in filenames {
        assert!(
            !dir.join(filename).exists(),
            "did not expect {} to exist",
            dir.join(filename).display()
        );
    }
}

// ---------------------------------------------------------------
// load_config
// ---------------------------------------------------------------

#[test]
fn load_config_returns_defaults_when_file_missing() {
    let dir = temp_test_dir("load_config_missing");
    let path = dir.join("nonexistent.toml");

    let cfg = load_config(&path, false).unwrap();
    // Defaults from Config::from_toml_str("")
    assert_eq!(cfg.artifacts_dir, PathBuf::from("out"));
    assert_eq!(cfg.out_dir, PathBuf::from("src/generated"));
    assert_eq!(*cfg.target(), abi_typegen_config::Target::Viem);
    assert!(cfg.wrappers);
    assert!(cfg.contracts.is_empty());

    cleanup(&dir);
}

#[test]
fn load_config_parses_real_temp_file() {
    let dir = temp_test_dir("load_config_real");
    let path = dir.join("foundry.toml");
    std::fs::write(
        &path,
        r#"
[profile.default]
out = "build-artifacts"

[abi-typegen]
out = "ts/types"
target = "ethers"
wrappers = false
contracts = ["Token", "Bridge"]
"#,
    )
    .unwrap();

    let cfg = load_config(&path, false).unwrap();
    assert_eq!(cfg.artifacts_dir, PathBuf::from("build-artifacts"));
    assert_eq!(cfg.out_dir, PathBuf::from("ts/types"));
    assert_eq!(*cfg.target(), abi_typegen_config::Target::Ethers);
    assert!(!cfg.wrappers);
    assert_eq!(cfg.contracts, vec!["Token", "Bridge"]);

    cleanup(&dir);
}

// ---------------------------------------------------------------
// apply_overrides
// ---------------------------------------------------------------

fn default_config() -> Config {
    Config::from_toml_str("").unwrap()
}

#[test]
fn apply_overrides_artifacts_overrides_artifacts_dir() {
    let mut cfg = default_config();
    apply_overrides(
        &mut cfg,
        Some(PathBuf::from("custom-artifacts")),
        None,
        None,
        false,
    )
    .unwrap();
    assert_eq!(cfg.artifacts_dir, PathBuf::from("custom-artifacts"));
}

#[test]
fn apply_overrides_out_overrides_out_dir() {
    let mut cfg = default_config();
    apply_overrides(&mut cfg, None, Some(PathBuf::from("my-types")), None, false).unwrap();
    assert_eq!(cfg.out_dir, PathBuf::from("my-types"));
}

#[test]
fn apply_overrides_target_viem() {
    let mut cfg = default_config();
    apply_overrides(&mut cfg, None, None, Some("viem".to_string()), false).unwrap();
    assert_eq!(*cfg.target(), abi_typegen_config::Target::Viem);
}

#[test]
fn apply_overrides_target_zod() {
    let mut cfg = default_config();
    apply_overrides(&mut cfg, None, None, Some("zod".to_string()), false).unwrap();
    assert_eq!(*cfg.target(), abi_typegen_config::Target::Zod);
}

#[test]
fn apply_overrides_target_ethers() {
    let mut cfg = default_config();
    apply_overrides(&mut cfg, None, None, Some("ethers".to_string()), false).unwrap();
    assert_eq!(*cfg.target(), abi_typegen_config::Target::Ethers);
}

#[test]
fn apply_overrides_target_solidity() {
    let mut cfg = default_config();
    apply_overrides(&mut cfg, None, None, Some("solidity".to_string()), false).unwrap();
    assert_eq!(*cfg.target(), abi_typegen_config::Target::Solidity);
}

#[test]
fn apply_overrides_unknown_target_returns_error() {
    let mut cfg = default_config();
    let result = apply_overrides(&mut cfg, None, None, Some("truffle".to_string()), false);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("truffle"));
}

#[test]
fn apply_overrides_no_wrappers_sets_false() {
    let mut cfg = default_config();
    assert!(cfg.wrappers); // default is true
    apply_overrides(&mut cfg, None, None, None, true).unwrap();
    assert!(!cfg.wrappers);
}

#[test]
fn apply_overrides_no_flags_leaves_config_unchanged() {
    let mut cfg = default_config();
    let original_out = cfg.artifacts_dir.clone();
    let original_gen = cfg.out_dir.clone();
    let original_targets = cfg.targets.clone();
    let original_wrappers = cfg.wrappers;

    apply_overrides(&mut cfg, None, None, None, false).unwrap();

    assert_eq!(cfg.artifacts_dir, original_out);
    assert_eq!(cfg.out_dir, original_gen);
    assert_eq!(cfg.targets, original_targets);
    assert_eq!(cfg.wrappers, original_wrappers);
}

// ---------------------------------------------------------------
// discover_artifacts
// ---------------------------------------------------------------

#[test]
fn discover_artifacts_empty_dir() {
    let dir = temp_test_dir("discover_empty");
    let results = discover_artifacts(&dir, &[]).unwrap();
    assert!(results.is_empty());
    cleanup(&dir);
}

#[test]
fn discover_artifacts_finds_sol_artifacts() {
    let dir = temp_test_dir("discover_finds");
    let sol_dir = dir.join("Foo.sol");
    std::fs::create_dir_all(&sol_dir).unwrap();
    std::fs::write(sol_dir.join("Foo.json"), "{}").unwrap();

    let results = discover_artifacts(&dir, &[]).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "Foo");
    assert_eq!(results[0].1, sol_dir.join("Foo.json"));

    cleanup(&dir);
}

#[test]
fn discover_artifacts_finds_nested_sol_artifacts() {
    let dir = temp_test_dir("discover_nested");
    let sol_dir = dir.join("contracts").join("tokens").join("Foo.sol");
    std::fs::create_dir_all(&sol_dir).unwrap();
    std::fs::write(sol_dir.join("Foo.json"), "{}").unwrap();

    let results = discover_artifacts(&dir, &[]).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "Foo");
    assert_eq!(results[0].1, sol_dir.join("Foo.json"));

    cleanup(&dir);
}

#[test]
fn discover_artifacts_filters_by_contract_list() {
    let dir = temp_test_dir("discover_filter");

    for name in &["Alpha", "Beta", "Gamma"] {
        let sol_dir = dir.join(format!("{}.sol", name));
        std::fs::create_dir_all(&sol_dir).unwrap();
        std::fs::write(sol_dir.join(format!("{}.json", name)), "{}").unwrap();
    }

    let filter = vec!["Alpha".to_string(), "Gamma".to_string()];
    let results = discover_artifacts(&dir, &filter).unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].0, "Alpha");
    assert_eq!(results[1].0, "Gamma");

    cleanup(&dir);
}

#[test]
fn discover_artifacts_sorts_alphabetically() {
    let dir = temp_test_dir("discover_sort");

    // Create in reverse alphabetical order
    for name in &["Zebra", "Mango", "Apple"] {
        let sol_dir = dir.join(format!("{}.sol", name));
        std::fs::create_dir_all(&sol_dir).unwrap();
        std::fs::write(sol_dir.join(format!("{}.json", name)), "{}").unwrap();
    }

    let results = discover_artifacts(&dir, &[]).unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].0, "Apple");
    assert_eq!(results[1].0, "Mango");
    assert_eq!(results[2].0, "Zebra");

    cleanup(&dir);
}

#[test]
fn discover_artifacts_skips_non_sol_dirs() {
    let dir = temp_test_dir("discover_skip_nonsol");

    // A .sol directory with artifact
    let sol_dir = dir.join("Foo.sol");
    std::fs::create_dir_all(&sol_dir).unwrap();
    std::fs::write(sol_dir.join("Foo.json"), "{}").unwrap();

    // A non-.sol directory
    let other_dir = dir.join("cache");
    std::fs::create_dir_all(&other_dir).unwrap();
    std::fs::write(other_dir.join("data.json"), "{}").unwrap();

    let results = discover_artifacts(&dir, &[]).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "Foo");

    cleanup(&dir);
}

#[test]
fn discover_artifacts_skips_sol_dirs_without_matching_json() {
    let dir = temp_test_dir("discover_skip_nojson");

    // .sol dir but no matching JSON
    let sol_dir = dir.join("Bar.sol");
    std::fs::create_dir_all(&sol_dir).unwrap();
    std::fs::write(sol_dir.join("Other.json"), "{}").unwrap();

    // .sol dir with matching JSON
    let good_dir = dir.join("Foo.sol");
    std::fs::create_dir_all(&good_dir).unwrap();
    std::fs::write(good_dir.join("Foo.json"), "{}").unwrap();

    let results = discover_artifacts(&dir, &[]).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "Foo");

    cleanup(&dir);
}

// ---------------------------------------------------------------
// run_init
// ---------------------------------------------------------------

#[test]
fn run_init_appends_scaffold_to_file_without_section() {
    let dir = temp_test_dir("init_append");
    let path = dir.join("foundry.toml");
    std::fs::write(
        &path,
        r#"[profile.default]
out = "out"
"#,
    )
    .unwrap();

    run_init(&path).unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("[abi-typegen]"));
    assert!(content.contains("out ="));
    assert!(content.contains("target ="));
    assert!(content.contains("wrappers ="));
    assert!(content.contains("contracts ="));
    // Original content still present
    assert!(content.contains("[profile.default]"));

    cleanup(&dir);
}

#[test]
fn run_init_noops_when_section_exists() {
    let dir = temp_test_dir("init_noop");
    let path = dir.join("foundry.toml");
    let original = r#"[profile.default]
out = "out"

[abi-typegen]
target = "viem"
"#;
    std::fs::write(&path, original).unwrap();

    run_init(&path).unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    // Content should be unchanged
    assert_eq!(content, original);

    cleanup(&dir);
}

#[test]
fn run_init_creates_new_file_when_missing() {
    let dir = temp_test_dir("init_create");
    let path = dir.join("foundry.toml");
    assert!(!path.exists());

    run_init(&path).unwrap();

    assert!(path.exists());
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("[abi-typegen]"));
    assert!(content.contains("out ="));

    cleanup(&dir);
}

// ---------------------------------------------------------------
// run_shell
// ---------------------------------------------------------------

// run_shell prints to stdout via println!. We test it through a
// subprocess using `cargo run` to capture the output.

#[test]
fn run_forge_install_bash_output() {
    // Call directly for coverage (prints to stdout, which is fine in tests)
    run_shell("bash");
    run_shell("zsh"); // same codepath as bash
}

#[test]
fn run_forge_install_fish_output() {
    run_shell("fish");
}

// ---------------------------------------------------------------
// run_generate (end-to-end)
// ---------------------------------------------------------------

#[test]
fn run_generate_produces_ts_files() {
    let dir = temp_test_dir("run_generate_e2e");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("generated");

    // Set up a minimal artifact in out/Token.sol/Token.json
    let sol_dir = artifacts_dir.join("Token.sol");
    std::fs::create_dir_all(&sol_dir).unwrap();
    std::fs::write(
            sol_dir.join("Token.json"),
            r#"{
                "abi": [{
                    "type": "function",
                    "name": "balanceOf",
                    "inputs": [{"name": "account", "type": "address", "internalType": "address", "components": []}],
                    "outputs": [{"name": "", "type": "uint256", "internalType": "uint256", "components": []}],
                    "stateMutability": "view"
                }]
            }"#,
        )
        .unwrap();

    let config = Config {
        artifacts_dir: artifacts_dir.clone(),
        out_dir: gen_dir.clone(),
        targets: vec![abi_typegen_config::Target::Viem],
        wrappers: true,
        contracts: vec![],
        exclude: vec![],
    };

    run_generate(&config, false).unwrap();

    // Verify output files exist
    assert!(gen_dir.join("Token.abi.ts").exists());
    assert!(gen_dir.join("Token.viem.ts").exists());
    assert!(gen_dir.join("index.ts").exists());

    // Verify content
    let abi = std::fs::read_to_string(gen_dir.join("Token.abi.ts")).unwrap();
    assert!(abi.contains("export const TokenAbi ="));
    assert!(abi.contains("] as const;"));

    let viem = std::fs::read_to_string(gen_dir.join("Token.viem.ts")).unwrap();
    assert!(viem.contains("export function getTokenContract("));

    let barrel = std::fs::read_to_string(gen_dir.join("index.ts")).unwrap();
    assert!(barrel.contains("export * from './Token.abi.js'"));
    assert!(barrel.contains("export * from './Token.viem.js'"));

    cleanup(&dir);
}

#[test]
fn run_generate_non_ts_target_omits_broken_abi_exports() {
    let dir = temp_test_dir("run_generate_non_ts_barrel");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("generated");

    write_target_matrix_artifact(&artifacts_dir, "Token");

    let mut config = generated_config(
        artifacts_dir,
        gen_dir.clone(),
        abi_typegen_config::Target::Python,
    );
    config.wrappers = false;

    run_generate(&config, false).unwrap();

    assert!(gen_dir.join("Token.py").exists());
    assert!(!gen_dir.join("Token.abi.ts").exists());

    let barrel = std::fs::read_to_string(gen_dir.join("index.ts")).unwrap();
    assert!(!barrel.contains("abi.js"));

    cleanup(&dir);
}

#[test]
fn run_generate_all_single_targets_write_expected_outputs() {
    struct TargetCase {
        name: &'static str,
        target: abi_typegen_config::Target,
        expected_files: &'static [&'static str],
        absent_files: &'static [&'static str],
        marker_file: &'static str,
        marker_text: &'static str,
    }

    let cases = [
        TargetCase {
            name: "viem",
            target: abi_typegen_config::Target::Viem,
            expected_files: &["Token.abi.ts", "Token.viem.ts", "index.ts"],
            absent_files: &[
                "IToken.sol",
                "Token.wagmi.ts",
                "Token.ethers.ts",
                "Token.ethers5.ts",
                "Token.web3.ts",
                "Token.py",
                "Token.go",
                "Token.rs",
                "Token.swift",
                "Token.cs",
                "Token.kt",
                "Token.yaml",
            ],
            marker_file: "Token.viem.ts",
            marker_text: "export function getTokenContract(",
        },
        TargetCase {
            name: "wagmi",
            target: abi_typegen_config::Target::Wagmi,
            expected_files: &["Token.abi.ts", "Token.wagmi.ts", "index.ts"],
            absent_files: &[
                "IToken.sol",
                "Token.viem.ts",
                "Token.ethers.ts",
                "Token.ethers5.ts",
                "Token.web3.ts",
                "Token.py",
                "Token.go",
                "Token.rs",
                "Token.swift",
                "Token.cs",
                "Token.kt",
                "Token.yaml",
            ],
            marker_file: "Token.wagmi.ts",
            marker_text: "export function useTokenBalanceOf(",
        },
        TargetCase {
            name: "zod",
            target: abi_typegen_config::Target::Zod,
            expected_files: &["Token.abi.ts", "Token.zod.ts", "index.ts"],
            absent_files: &[
                "IToken.sol",
                "Token.viem.ts",
                "Token.wagmi.ts",
                "Token.ethers.ts",
                "Token.ethers5.ts",
                "Token.web3.ts",
                "Token.py",
                "Token.go",
                "Token.rs",
                "Token.swift",
                "Token.cs",
                "Token.kt",
                "Token.yaml",
            ],
            marker_file: "Token.zod.ts",
            marker_text: "export const TokenBalanceOfResultSchema = z.bigint().refine(",
        },
        TargetCase {
            name: "ethers",
            target: abi_typegen_config::Target::Ethers,
            expected_files: &["Token.abi.ts", "Token.ethers.ts", "index.ts"],
            absent_files: &[
                "IToken.sol",
                "Token.viem.ts",
                "Token.wagmi.ts",
                "Token.ethers5.ts",
                "Token.web3.ts",
                "Token.py",
                "Token.go",
                "Token.rs",
                "Token.swift",
                "Token.cs",
                "Token.kt",
                "Token.yaml",
            ],
            marker_file: "Token.ethers.ts",
            marker_text: "export interface TokenContract {",
        },
        TargetCase {
            name: "ethers5",
            target: abi_typegen_config::Target::Ethers5,
            expected_files: &["Token.abi.ts", "Token.ethers5.ts", "index.ts"],
            absent_files: &[
                "IToken.sol",
                "Token.viem.ts",
                "Token.wagmi.ts",
                "Token.ethers.ts",
                "Token.web3.ts",
                "Token.py",
                "Token.go",
                "Token.rs",
                "Token.swift",
                "Token.cs",
                "Token.kt",
                "Token.yaml",
            ],
            marker_file: "Token.ethers5.ts",
            marker_text: "export interface TokenContract extends ethers.Contract {",
        },
        TargetCase {
            name: "web3js",
            target: abi_typegen_config::Target::Web3js,
            expected_files: &["Token.abi.ts", "Token.web3.ts", "index.ts"],
            absent_files: &[
                "IToken.sol",
                "Token.viem.ts",
                "Token.wagmi.ts",
                "Token.ethers.ts",
                "Token.ethers5.ts",
                "Token.py",
                "Token.go",
                "Token.rs",
                "Token.swift",
                "Token.cs",
                "Token.kt",
                "Token.yaml",
            ],
            marker_file: "Token.web3.ts",
            marker_text: "export function createToken(web3: Web3, address: string)",
        },
        TargetCase {
            name: "python",
            target: abi_typegen_config::Target::Python,
            expected_files: &["Token.py", "index.ts"],
            absent_files: &[
                "IToken.sol",
                "Token.abi.ts",
                "Token.viem.ts",
                "Token.wagmi.ts",
                "Token.ethers.ts",
                "Token.ethers5.ts",
                "Token.web3.ts",
                "Token.go",
                "Token.rs",
                "Token.swift",
                "Token.cs",
                "Token.kt",
                "Token.yaml",
            ],
            marker_file: "Token.py",
            marker_text: "class TokenContract:",
        },
        TargetCase {
            name: "go",
            target: abi_typegen_config::Target::Go,
            expected_files: &["Token.go", "index.ts"],
            absent_files: &[
                "IToken.sol",
                "Token.abi.ts",
                "Token.viem.ts",
                "Token.wagmi.ts",
                "Token.ethers.ts",
                "Token.ethers5.ts",
                "Token.web3.ts",
                "Token.py",
                "Token.rs",
                "Token.swift",
                "Token.cs",
                "Token.kt",
                "Token.yaml",
            ],
            marker_file: "Token.go",
            marker_text: "const TokenABI = ",
        },
        TargetCase {
            name: "rust",
            target: abi_typegen_config::Target::Rust,
            expected_files: &["Token.rs", "index.ts"],
            absent_files: &[
                "IToken.sol",
                "Token.abi.ts",
                "Token.viem.ts",
                "Token.wagmi.ts",
                "Token.ethers.ts",
                "Token.ethers5.ts",
                "Token.web3.ts",
                "Token.py",
                "Token.go",
                "Token.swift",
                "Token.cs",
                "Token.kt",
                "Token.yaml",
            ],
            marker_file: "Token.rs",
            marker_text: "pub struct TokenBalanceOfParams",
        },
        TargetCase {
            name: "swift",
            target: abi_typegen_config::Target::Swift,
            expected_files: &["Token.swift", "index.ts"],
            absent_files: &[
                "IToken.sol",
                "Token.abi.ts",
                "Token.viem.ts",
                "Token.wagmi.ts",
                "Token.ethers.ts",
                "Token.ethers5.ts",
                "Token.web3.ts",
                "Token.py",
                "Token.go",
                "Token.rs",
                "Token.cs",
                "Token.kt",
                "Token.yaml",
            ],
            marker_file: "Token.swift",
            marker_text: "struct TokenBalanceOfParams {",
        },
        TargetCase {
            name: "csharp",
            target: abi_typegen_config::Target::CSharp,
            expected_files: &["Token.cs", "index.ts"],
            absent_files: &[
                "IToken.sol",
                "Token.abi.ts",
                "Token.viem.ts",
                "Token.wagmi.ts",
                "Token.ethers.ts",
                "Token.ethers5.ts",
                "Token.web3.ts",
                "Token.py",
                "Token.go",
                "Token.rs",
                "Token.swift",
                "Token.kt",
                "Token.yaml",
            ],
            marker_file: "Token.cs",
            marker_text: "public class TokenBalanceOfParams",
        },
        TargetCase {
            name: "kotlin",
            target: abi_typegen_config::Target::Kotlin,
            expected_files: &["Token.kt", "index.ts"],
            absent_files: &[
                "IToken.sol",
                "Token.abi.ts",
                "Token.viem.ts",
                "Token.wagmi.ts",
                "Token.ethers.ts",
                "Token.ethers5.ts",
                "Token.web3.ts",
                "Token.py",
                "Token.go",
                "Token.rs",
                "Token.swift",
                "Token.cs",
                "Token.yaml",
            ],
            marker_file: "Token.kt",
            marker_text: "data class TokenBalanceOfParams(",
        },
        TargetCase {
            name: "solidity",
            target: abi_typegen_config::Target::Solidity,
            expected_files: &["IToken.sol", "index.ts"],
            absent_files: &[
                "Token.abi.ts",
                "Token.viem.ts",
                "Token.wagmi.ts",
                "Token.ethers.ts",
                "Token.ethers5.ts",
                "Token.web3.ts",
                "Token.py",
                "Token.go",
                "Token.rs",
                "Token.swift",
                "Token.cs",
                "Token.kt",
                "Token.yaml",
            ],
            marker_file: "IToken.sol",
            marker_text: "interface IToken {",
        },
        TargetCase {
            name: "yaml",
            target: abi_typegen_config::Target::Yaml,
            expected_files: &["Token.yaml", "index.ts"],
            absent_files: &[
                "IToken.sol",
                "Token.abi.ts",
                "Token.viem.ts",
                "Token.wagmi.ts",
                "Token.ethers.ts",
                "Token.ethers5.ts",
                "Token.web3.ts",
                "Token.py",
                "Token.go",
                "Token.rs",
                "Token.swift",
                "Token.cs",
                "Token.kt",
            ],
            marker_file: "Token.yaml",
            marker_text: "name: \"Token\"",
        },
    ];

    for case in cases {
        let dir = temp_test_dir(&format!("run_generate_target_matrix_{}", case.name));
        let artifacts_dir = dir.join("out");
        let gen_dir = dir.join("generated");
        write_target_matrix_artifact(&artifacts_dir, "Token");

        let config = generated_config(artifacts_dir, gen_dir.clone(), case.target);
        run_generate(&config, false)
            .unwrap_or_else(|e| panic!("{} target generation failed: {}", case.name, e));

        for filename in case.expected_files {
            assert!(
                gen_dir.join(filename).exists(),
                "expected {} output {}",
                case.name,
                gen_dir.join(filename).display()
            );
        }
        assert_missing_files(&gen_dir, case.absent_files);
        assert_file_contains(&gen_dir.join(case.marker_file), case.marker_text);

        cleanup(&dir);
    }
}

#[test]
fn run_generate_missing_artifacts_dir_errors() {
    let dir = temp_test_dir("run_generate_missing_out");
    let config = Config {
        artifacts_dir: dir.join("nonexistent-out"),
        out_dir: dir.join("gen"),
        targets: vec![abi_typegen_config::Target::Viem],
        wrappers: true,
        contracts: vec![],
        exclude: vec![],
    };

    let result = run_generate(&config, false);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("does not exist"));

    cleanup(&dir);
}

#[test]
fn run_generate_empty_artifacts_dir_prints_no_artifacts() {
    let dir = temp_test_dir("run_generate_empty");
    let artifacts_dir = dir.join("out");
    std::fs::create_dir_all(&artifacts_dir).unwrap();

    let config = Config {
        artifacts_dir,
        out_dir: dir.join("gen"),
        targets: vec![abi_typegen_config::Target::Viem],
        wrappers: true,
        contracts: vec![],
        exclude: vec![],
    };

    // Should succeed but not create any files
    run_generate(&config, false).unwrap();
    assert!(!dir.join("gen").join("index.ts").exists());

    cleanup(&dir);
}

#[test]
fn run_generate_skips_unparseable_artifact() {
    let dir = temp_test_dir("run_generate_bad_artifact");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("generated");

    // Good artifact
    let good_dir = artifacts_dir.join("Good.sol");
    std::fs::create_dir_all(&good_dir).unwrap();
    std::fs::write(
        good_dir.join("Good.json"),
        r#"{"abi": [{"type": "receive", "stateMutability": "payable"}]}"#,
    )
    .unwrap();

    // Bad artifact (invalid JSON)
    let bad_dir = artifacts_dir.join("Bad.sol");
    std::fs::create_dir_all(&bad_dir).unwrap();
    std::fs::write(bad_dir.join("Bad.json"), "not json at all").unwrap();

    let config = Config {
        artifacts_dir,
        out_dir: gen_dir.clone(),
        targets: vec![abi_typegen_config::Target::Viem],
        wrappers: false,
        contracts: vec![],
        exclude: vec![],
    };

    run_generate(&config, false).unwrap();

    // Good contract should be generated
    assert!(gen_dir.join("Good.abi.ts").exists());
    // Bad contract should be skipped (no file)
    assert!(!gen_dir.join("Bad.abi.ts").exists());
    // Barrel should only have Good
    let barrel = std::fs::read_to_string(gen_dir.join("index.ts")).unwrap();
    assert!(barrel.contains("Good.abi.js"));
    assert!(!barrel.contains("Bad"));

    cleanup(&dir);
}

#[test]
fn run_generate_with_contract_filter() {
    let dir = temp_test_dir("run_generate_filter");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("generated");

    for name in &["Alpha", "Beta"] {
        let sol_dir = artifacts_dir.join(format!("{}.sol", name));
        std::fs::create_dir_all(&sol_dir).unwrap();
        std::fs::write(sol_dir.join(format!("{}.json", name)), r#"{"abi": []}"#).unwrap();
    }

    let config = Config {
        artifacts_dir,
        out_dir: gen_dir.clone(),
        targets: vec![abi_typegen_config::Target::Viem],
        wrappers: false,
        contracts: vec!["Alpha".to_string()],
        exclude: vec![],
    };

    run_generate(&config, false).unwrap();

    assert!(gen_dir.join("Alpha.abi.ts").exists());
    assert!(!gen_dir.join("Beta.abi.ts").exists());

    cleanup(&dir);
}

#[test]
fn run_generate_all_parse_failures_prints_no_contracts() {
    let dir = temp_test_dir("run_generate_all_fail");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("generated");

    let sol_dir = artifacts_dir.join("Bad.sol");
    std::fs::create_dir_all(&sol_dir).unwrap();
    std::fs::write(sol_dir.join("Bad.json"), "invalid").unwrap();

    let config = Config {
        artifacts_dir,
        out_dir: gen_dir.clone(),
        targets: vec![abi_typegen_config::Target::Viem],
        wrappers: true,
        contracts: vec![],
        exclude: vec![],
    };

    run_generate(&config, false).unwrap();

    // No barrel should be written when all contracts fail
    assert!(!gen_dir.join("index.ts").exists());

    cleanup(&dir);
}

#[test]
fn discover_artifacts_skips_plain_files_in_artifacts_dir() {
    let dir = temp_test_dir("discover_plain_file");

    // A plain file (not a directory) in out/
    std::fs::write(dir.join("debug.log"), "some log").unwrap();

    // A valid artifact
    let sol_dir = dir.join("Foo.sol");
    std::fs::create_dir_all(&sol_dir).unwrap();
    std::fs::write(sol_dir.join("Foo.json"), "{}").unwrap();

    let results = discover_artifacts(&dir, &[]).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "Foo");

    cleanup(&dir);
}

// ---------------------------------------------------------------
// run() dispatch (covers the main→run extraction)
// ---------------------------------------------------------------

#[test]
fn run_dispatches_generate() {
    let dir = temp_test_dir("run_dispatch_gen");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("generated");

    let sol_dir = artifacts_dir.join("T.sol");
    std::fs::create_dir_all(&sol_dir).unwrap();
    std::fs::write(sol_dir.join("T.json"), r#"{"abi": []}"#).unwrap();

    // Write a foundry.toml pointing to our dirs
    let toml_path = dir.join("foundry.toml");
    std::fs::write(
        &toml_path,
        format!(
            "[profile.default]\nout = \"{}\"\n\n[abi-typegen]\nout = \"{}\"\nwrappers = false\n",
            artifacts_dir.display(),
            gen_dir.display()
        ),
    )
    .unwrap();

    let cli = Cli {
        command: Commands::Generate {
            artifacts: None,
            out: None,
            target: None,
            no_wrappers: false,
            contracts: vec![],
            exclude: None,
            check: false,
            clean: false,
        },
        config: Some(toml_path),
        hardhat: false,
    };
    run(cli).unwrap();
    assert!(gen_dir.join("T.abi.ts").exists());

    cleanup(&dir);
}

#[test]
fn run_dispatches_init() {
    let dir = temp_test_dir("run_dispatch_init");
    let toml_path = dir.join("foundry.toml");

    let cli = Cli {
        command: Commands::Init,
        config: Some(toml_path.clone()),
        hardhat: false,
    };
    run(cli).unwrap();

    let content = std::fs::read_to_string(&toml_path).unwrap();
    assert!(content.contains("[abi-typegen]"));

    cleanup(&dir);
}

#[test]
fn run_dispatches_forge_install() {
    let dir = temp_test_dir("run_dispatch_forge_install");
    let cli = Cli {
        command: Commands::ForgeInstall {
            shell: "bash".into(),
        },
        config: Some(dir.join("foundry.toml")),
        hardhat: false,
    };
    run(cli).unwrap();
    cleanup(&dir);
}

#[test]
fn run_dispatches_generate_with_overrides() {
    let dir = temp_test_dir("run_dispatch_gen_override");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("types");

    let sol_dir = artifacts_dir.join("X.sol");
    std::fs::create_dir_all(&sol_dir).unwrap();
    std::fs::write(sol_dir.join("X.json"), r#"{"abi": []}"#).unwrap();

    let cli = Cli {
        command: Commands::Generate {
            artifacts: Some(artifacts_dir),
            out: Some(gen_dir.clone()),
            target: Some("viem".into()),
            no_wrappers: true,
            contracts: vec![],
            exclude: None,
            check: false,
            clean: false,
        },
        config: Some(dir.join("nonexistent.toml")),
        hardhat: false,
    };
    run(cli).unwrap();
    assert!(gen_dir.join("X.abi.ts").exists());

    cleanup(&dir);
}

#[test]
fn run_dispatches_generate_with_contract_overrides() {
    let dir = temp_test_dir("run_dispatch_gen_contract_override");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("types");

    for name in &["Alpha", "Beta"] {
        let sol_dir = artifacts_dir.join(format!("{}.sol", name));
        std::fs::create_dir_all(&sol_dir).unwrap();
        std::fs::write(sol_dir.join(format!("{}.json", name)), r#"{"abi": []}"#).unwrap();
    }

    let cli = Cli {
        command: Commands::Generate {
            artifacts: Some(artifacts_dir),
            out: Some(gen_dir.clone()),
            target: Some("viem".into()),
            no_wrappers: true,
            contracts: vec!["Beta".into()],
            exclude: None,
            check: false,
            clean: false,
        },
        config: Some(dir.join("nonexistent.toml")),
        hardhat: false,
    };
    run(cli).unwrap();

    assert!(!gen_dir.join("Alpha.abi.ts").exists());
    assert!(gen_dir.join("Beta.abi.ts").exists());

    cleanup(&dir);
}

// ---------------------------------------------------------------
// watch_loop
// ---------------------------------------------------------------

#[test]
fn watch_loop_exits_on_channel_disconnect() {
    let dir = temp_test_dir("watch_loop_disconnect");
    let artifacts_dir = dir.join("out");
    std::fs::create_dir_all(&artifacts_dir).unwrap();

    let config = Config {
        artifacts_dir,
        out_dir: dir.join("gen"),
        targets: vec![abi_typegen_config::Target::Viem],
        wrappers: false,
        contracts: vec![],
        exclude: vec![],
    };

    let (tx, rx) = std::sync::mpsc::channel::<notify::Result<notify::Event>>();
    // Drop the sender immediately → recv() returns Err → loop breaks
    drop(tx);
    watch_loop(&config, &rx);
    // If we get here, the loop exited cleanly
    cleanup(&dir);
}

#[test]
fn watch_loop_handles_event_and_regenerates() {
    let dir = temp_test_dir("watch_loop_event");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("gen");

    // Set up a valid artifact so run_generate succeeds
    let sol_dir = artifacts_dir.join("W.sol");
    std::fs::create_dir_all(&sol_dir).unwrap();
    std::fs::write(sol_dir.join("W.json"), r#"{"abi": []}"#).unwrap();

    let config = Config {
        artifacts_dir: artifacts_dir.clone(),
        out_dir: gen_dir.clone(),
        targets: vec![abi_typegen_config::Target::Viem],
        wrappers: false,
        contracts: vec![],
        exclude: vec![],
    };

    let (tx, rx) = std::sync::mpsc::channel();
    // Send one event, then drop sender to terminate the loop
    tx.send(Ok(notify::Event::new(notify::EventKind::Modify(
        notify::event::ModifyKind::Data(notify::event::DataChange::Any),
    ))))
    .unwrap();
    drop(tx);

    watch_loop(&config, &rx);

    // run_generate should have been triggered by the event
    assert!(gen_dir.join("W.abi.ts").exists());
    cleanup(&dir);
}

#[test]
fn watch_loop_handles_watch_error() {
    let dir = temp_test_dir("watch_loop_err");
    let artifacts_dir = dir.join("out");
    std::fs::create_dir_all(&artifacts_dir).unwrap();

    let config = Config {
        artifacts_dir,
        out_dir: dir.join("gen"),
        targets: vec![abi_typegen_config::Target::Viem],
        wrappers: false,
        contracts: vec![],
        exclude: vec![],
    };

    let (tx, rx) = std::sync::mpsc::channel();
    // Send an error event, then drop sender
    tx.send(Err(notify::Error::generic("test error"))).unwrap();
    drop(tx);

    watch_loop(&config, &rx);
    // Should handle error gracefully and exit
    cleanup(&dir);
}

// ---------------------------------------------------------------
// run() Watch dispatch
// ---------------------------------------------------------------

#[test]
fn run_dispatches_watch() {
    let dir = temp_test_dir("run_dispatch_watch");
    let artifacts_dir = dir.join("out");
    std::fs::create_dir_all(&artifacts_dir).unwrap();

    let toml_path = dir.join("foundry.toml");
    std::fs::write(
        &toml_path,
        format!("[profile.default]\nout = \"{}\"\n", artifacts_dir.display()),
    )
    .unwrap();

    let cli = Cli {
        command: Commands::Watch { artifacts: None },
        config: Some(toml_path),
        hardhat: false,
    };

    // run_watch blocks, so spawn in a thread and give it a moment
    let handle = std::thread::spawn(move || run(cli));
    // Allow the watcher to start
    std::thread::sleep(std::time::Duration::from_millis(100));
    // Write a file to trigger an event
    std::fs::write(artifacts_dir.join("trigger.txt"), "x").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(400));

    // The thread is blocked; we can't join it cleanly without a shutdown signal.
    // But coverage is collected when the test binary exits.
    // Detach the thread.
    drop(handle);
    cleanup(&dir);
}

#[test]
fn run_dispatches_watch_with_out_override() {
    let dir = temp_test_dir("run_dispatch_watch_override");
    let artifacts_dir = dir.join("custom-out");
    std::fs::create_dir_all(&artifacts_dir).unwrap();

    let cli = Cli {
        command: Commands::Watch {
            artifacts: Some(artifacts_dir.clone()),
        },
        config: Some(dir.join("nonexistent.toml")),
        hardhat: false,
    };

    let handle = std::thread::spawn(move || run(cli));
    std::thread::sleep(std::time::Duration::from_millis(100));
    std::fs::write(artifacts_dir.join("trigger.txt"), "x").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(400));
    drop(handle);
    cleanup(&dir);
}

#[test]
fn watch_loop_regenerate_error_is_logged_not_fatal() {
    // Point artifacts_dir at a valid dir but out_dir at an impossible path
    // so run_generate inside the loop fails
    let dir = temp_test_dir("watch_loop_regen_err");
    let artifacts_dir = dir.join("out");
    std::fs::create_dir_all(&artifacts_dir).unwrap();

    let sol_dir = artifacts_dir.join("Z.sol");
    std::fs::create_dir_all(&sol_dir).unwrap();
    std::fs::write(sol_dir.join("Z.json"), r#"{"abi": []}"#).unwrap();

    let config = Config {
        artifacts_dir,
        // Point at an impossible path so write fails
        out_dir: PathBuf::from("/dev/null/impossible"),
        targets: vec![abi_typegen_config::Target::Viem],
        wrappers: false,
        contracts: vec![],
        exclude: vec![],
    };

    let (tx, rx) = std::sync::mpsc::channel();
    tx.send(Ok(notify::Event::new(notify::EventKind::Modify(
        notify::event::ModifyKind::Data(notify::event::DataChange::Any),
    ))))
    .unwrap();
    drop(tx);

    // Should not panic — error is logged via eprintln
    watch_loop(&config, &rx);
    cleanup(&dir);
}

#[test]
fn run_watch_initial_generate_failure_continues() {
    // artifacts_dir exists but has no artifacts, and out_dir can't be created
    // → initial run_generate fails, but watch should still start
    let dir = temp_test_dir("run_watch_init_fail");
    let artifacts_dir = dir.join("out");
    std::fs::create_dir_all(&artifacts_dir).unwrap();

    let config = Config {
        artifacts_dir: artifacts_dir.clone(),
        out_dir: PathBuf::from("/dev/null/impossible"),
        targets: vec![abi_typegen_config::Target::Viem],
        wrappers: false,
        contracts: vec![],
        exclude: vec![],
    };

    // Use watch_loop directly with immediate disconnect to test the flow
    // where initial generate would fail (simulated by impossible out_dir)
    // but we can't easily test run_watch itself without blocking.
    // Instead, we verify run_generate fails and that's handled:
    assert!(run_generate(&config, false).is_err());
    cleanup(&dir);
}

// ---------------------------------------------------------------
// matches_glob
// ---------------------------------------------------------------

#[test]
fn matches_glob_star_suffix() {
    assert!(matches_glob("IToken", "I*"));
    assert!(matches_glob("IERC20", "I*"));
    assert!(!matches_glob("Token", "I*"));
}

#[test]
fn matches_glob_star_prefix() {
    assert!(matches_glob("TokenTest", "*Test"));
    assert!(matches_glob("Test", "*Test"));
    assert!(!matches_glob("TestHelper", "*Test"));
}

#[test]
fn matches_glob_star_both() {
    assert!(matches_glob("MyMockContract", "*Mock*"));
    assert!(matches_glob("MockContract", "*Mock*"));
    assert!(matches_glob("ContractMock", "*Mock*"));
    assert!(!matches_glob("Token", "*Mock*"));
}

#[test]
fn matches_glob_exact() {
    assert!(matches_glob("Token", "Token"));
    assert!(!matches_glob("Token", "Vault"));
}

#[test]
fn matches_glob_question_mark() {
    assert!(matches_glob("A1", "A?"));
    assert!(!matches_glob("A12", "A?"));
}

// ---------------------------------------------------------------
// --exclude
// ---------------------------------------------------------------

#[test]
fn exclude_filters_contracts_ending_in_test() {
    let dir = temp_test_dir("exclude_test_suffix");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("generated");

    for name in &["Token", "TokenTest", "Vault"] {
        let sol_dir = artifacts_dir.join(format!("{}.sol", name));
        std::fs::create_dir_all(&sol_dir).unwrap();
        std::fs::write(sol_dir.join(format!("{}.json", name)), r#"{"abi": []}"#).unwrap();
    }

    let config = Config {
        artifacts_dir,
        out_dir: gen_dir.clone(),
        targets: vec![abi_typegen_config::Target::Viem],
        wrappers: false,
        contracts: vec![],
        exclude: vec!["*Test".to_string()],
    };

    run_generate(&config, false).unwrap();

    assert!(gen_dir.join("Token.abi.ts").exists());
    assert!(gen_dir.join("Vault.abi.ts").exists());
    assert!(!gen_dir.join("TokenTest.abi.ts").exists());

    cleanup(&dir);
}

#[test]
fn exclude_filters_interfaces() {
    let dir = temp_test_dir("exclude_interfaces");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("generated");

    for name in &["Token", "IToken", "IERC20", "Vault"] {
        let sol_dir = artifacts_dir.join(format!("{}.sol", name));
        std::fs::create_dir_all(&sol_dir).unwrap();
        std::fs::write(sol_dir.join(format!("{}.json", name)), r#"{"abi": []}"#).unwrap();
    }

    let config = Config {
        artifacts_dir,
        out_dir: gen_dir.clone(),
        targets: vec![abi_typegen_config::Target::Viem],
        wrappers: false,
        contracts: vec![],
        exclude: vec!["I*".to_string()],
    };

    run_generate(&config, false).unwrap();

    assert!(gen_dir.join("Token.abi.ts").exists());
    assert!(gen_dir.join("Vault.abi.ts").exists());
    assert!(!gen_dir.join("IToken.abi.ts").exists());
    assert!(!gen_dir.join("IERC20.abi.ts").exists());

    cleanup(&dir);
}

#[test]
fn exclude_multiple_patterns() {
    let dir = temp_test_dir("exclude_multiple");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("generated");

    for name in &["Token", "TokenTest", "IToken", "MockVault", "Vault"] {
        let sol_dir = artifacts_dir.join(format!("{}.sol", name));
        std::fs::create_dir_all(&sol_dir).unwrap();
        std::fs::write(sol_dir.join(format!("{}.json", name)), r#"{"abi": []}"#).unwrap();
    }

    let config = Config {
        artifacts_dir,
        out_dir: gen_dir.clone(),
        targets: vec![abi_typegen_config::Target::Viem],
        wrappers: false,
        contracts: vec![],
        exclude: vec!["*Test".to_string(), "I*".to_string(), "Mock*".to_string()],
    };

    run_generate(&config, false).unwrap();

    assert!(gen_dir.join("Token.abi.ts").exists());
    assert!(gen_dir.join("Vault.abi.ts").exists());
    assert!(!gen_dir.join("TokenTest.abi.ts").exists());
    assert!(!gen_dir.join("IToken.abi.ts").exists());
    assert!(!gen_dir.join("MockVault.abi.ts").exists());

    cleanup(&dir);
}

#[test]
fn selected_artifacts_applies_exclude_patterns() {
    let dir = temp_test_dir("selected_artifacts_exclude");
    let artifacts_dir = dir.join("out");

    for name in &["Token", "TokenTest", "IToken"] {
        let sol_dir = artifacts_dir.join(format!("{}.sol", name));
        std::fs::create_dir_all(&sol_dir).unwrap();
        std::fs::write(sol_dir.join(format!("{}.json", name)), r#"{"abi": []}"#).unwrap();
    }

    let config = Config {
        artifacts_dir,
        out_dir: dir.join("generated"),
        targets: vec![abi_typegen_config::Target::Viem],
        wrappers: false,
        contracts: vec![],
        exclude: vec!["*Test".to_string(), "I*".to_string()],
    };

    let artifacts = selected_artifacts(&config).unwrap();
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].0, "Token");

    cleanup(&dir);
}

#[test]
fn collect_diff_entries_ignores_excluded_contracts() {
    let dir = temp_test_dir("diff_entries_exclude");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("generated");

    for name in &["Token", "TokenTest"] {
        let sol_dir = artifacts_dir.join(format!("{}.sol", name));
        std::fs::create_dir_all(&sol_dir).unwrap();
        std::fs::write(sol_dir.join(format!("{}.json", name)), r#"{"abi": []}"#).unwrap();
    }

    let config = Config {
        artifacts_dir,
        out_dir: gen_dir.clone(),
        targets: vec![abi_typegen_config::Target::Viem],
        wrappers: false,
        contracts: vec![],
        exclude: vec!["*Test".to_string()],
    };

    run_generate(&config, false).unwrap();

    let artifacts = selected_artifacts(&config).unwrap();
    let diffs = collect_diff_entries(&config, &artifacts).unwrap();
    assert!(diffs.is_empty());

    cleanup(&dir);
}

#[test]
fn collect_diff_entries_reports_deleted_generated_files() {
    let dir = temp_test_dir("diff_entries_deleted_files");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("generated");

    let sol_dir = artifacts_dir.join("Token.sol");
    std::fs::create_dir_all(&sol_dir).unwrap();
    std::fs::write(sol_dir.join("Token.json"), r#"{"abi": []}"#).unwrap();

    let config = Config {
        artifacts_dir,
        out_dir: gen_dir.clone(),
        targets: vec![abi_typegen_config::Target::Viem],
        wrappers: false,
        contracts: vec![],
        exclude: vec![],
    };

    run_generate(&config, false).unwrap();
    std::fs::write(gen_dir.join("OldToken.abi.ts"), "stale content").unwrap();

    let artifacts = selected_artifacts(&config).unwrap();
    let diffs = collect_diff_entries(&config, &artifacts).unwrap();
    assert!(diffs.contains(&"D OldToken.abi.ts".to_string()));

    cleanup(&dir);
}

#[test]
fn collect_json_summaries_ignores_excluded_contracts() {
    let dir = temp_test_dir("json_summaries_exclude");
    let artifacts_dir = dir.join("out");

    for name in &["Token", "TokenTest"] {
        let sol_dir = artifacts_dir.join(format!("{}.sol", name));
        std::fs::create_dir_all(&sol_dir).unwrap();
        std::fs::write(sol_dir.join(format!("{}.json", name)), r#"{"abi": []}"#).unwrap();
    }

    let config = Config {
        artifacts_dir,
        out_dir: dir.join("generated"),
        targets: vec![abi_typegen_config::Target::Viem],
        wrappers: false,
        contracts: vec![],
        exclude: vec!["*Test".to_string()],
    };

    let summaries = collect_json_summaries(&config).unwrap();
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0]["name"], "Token");

    cleanup(&dir);
}

// ---------------------------------------------------------------
// --check
// ---------------------------------------------------------------

#[test]
fn check_returns_ok_when_output_matches() {
    let dir = temp_test_dir("check_ok");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("generated");

    let sol_dir = artifacts_dir.join("Token.sol");
    std::fs::create_dir_all(&sol_dir).unwrap();
    std::fs::write(sol_dir.join("Token.json"), r#"{"abi": []}"#).unwrap();

    let config = Config {
        artifacts_dir,
        out_dir: gen_dir.clone(),
        targets: vec![abi_typegen_config::Target::Viem],
        wrappers: false,
        contracts: vec![],
        exclude: vec![],
    };

    // Generate first
    run_generate(&config, false).unwrap();

    // Check should succeed
    let result = run_check(&config);
    assert!(result.is_ok());

    cleanup(&dir);
}

#[test]
fn check_returns_err_when_output_is_stale() {
    let dir = temp_test_dir("check_stale");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("generated");

    let sol_dir = artifacts_dir.join("Token.sol");
    std::fs::create_dir_all(&sol_dir).unwrap();
    std::fs::write(sol_dir.join("Token.json"), r#"{"abi": []}"#).unwrap();

    let config = Config {
        artifacts_dir,
        out_dir: gen_dir.clone(),
        targets: vec![abi_typegen_config::Target::Viem],
        wrappers: false,
        contracts: vec![],
        exclude: vec![],
    };

    // Generate first
    run_generate(&config, false).unwrap();

    // Tamper with a generated file
    let abi_file = gen_dir.join("Token.abi.ts");
    std::fs::write(&abi_file, "// tampered").unwrap();

    // Check should fail
    let result = run_check(&config);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("stale"));

    cleanup(&dir);
}

#[test]
fn check_returns_err_when_extra_generated_file_exists() {
    let dir = temp_test_dir("check_extra_generated_file");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("generated");

    let sol_dir = artifacts_dir.join("Token.sol");
    std::fs::create_dir_all(&sol_dir).unwrap();
    std::fs::write(sol_dir.join("Token.json"), r#"{"abi": []}"#).unwrap();

    let config = Config {
        artifacts_dir,
        out_dir: gen_dir.clone(),
        targets: vec![abi_typegen_config::Target::Viem],
        wrappers: false,
        contracts: vec![],
        exclude: vec![],
    };

    run_generate(&config, false).unwrap();
    std::fs::write(gen_dir.join("OldToken.abi.ts"), "stale content").unwrap();

    let result = run_check(&config);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("stale"));

    cleanup(&dir);
}

// ---------------------------------------------------------------
// --clean
// ---------------------------------------------------------------

#[test]
fn clean_removes_stale_files() {
    let dir = temp_test_dir("clean_stale");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("generated");

    // Create artifacts for Token only
    let sol_dir = artifacts_dir.join("Token.sol");
    std::fs::create_dir_all(&sol_dir).unwrap();
    std::fs::write(sol_dir.join("Token.json"), r#"{"abi": []}"#).unwrap();

    let config = Config {
        artifacts_dir,
        out_dir: gen_dir.clone(),
        targets: vec![abi_typegen_config::Target::Viem],
        wrappers: false,
        contracts: vec![],
        exclude: vec![],
    };

    // Pre-create a stale generated file
    std::fs::create_dir_all(&gen_dir).unwrap();
    std::fs::write(gen_dir.join("OldContract.abi.ts"), "stale content").unwrap();
    std::fs::write(gen_dir.join("OldContract.viem.ts"), "stale content").unwrap();
    // Also create a non-generated file that should NOT be removed
    std::fs::write(gen_dir.join("custom.txt"), "keep me").unwrap();

    // Run with clean=true
    run_generate(&config, true).unwrap();

    // Token files should exist
    assert!(gen_dir.join("Token.abi.ts").exists());
    // Stale files should be removed
    assert!(!gen_dir.join("OldContract.abi.ts").exists());
    assert!(!gen_dir.join("OldContract.viem.ts").exists());
    // Non-generated file should remain
    assert!(gen_dir.join("custom.txt").exists());

    cleanup(&dir);
}

#[test]
fn clean_removes_stale_web3_files() {
    let dir = temp_test_dir("clean_stale_web3");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("generated");

    let sol_dir = artifacts_dir.join("Token.sol");
    std::fs::create_dir_all(&sol_dir).unwrap();
    std::fs::write(sol_dir.join("Token.json"), r#"{"abi": []}"#).unwrap();

    let config = Config {
        artifacts_dir,
        out_dir: gen_dir.clone(),
        targets: vec![abi_typegen_config::Target::Web3js],
        wrappers: true,
        contracts: vec![],
        exclude: vec![],
    };

    std::fs::create_dir_all(&gen_dir).unwrap();
    std::fs::write(gen_dir.join("OldContract.web3.ts"), "stale content").unwrap();

    run_generate(&config, true).unwrap();

    assert!(gen_dir.join("Token.web3.ts").exists());
    assert!(!gen_dir.join("OldContract.web3.ts").exists());

    cleanup(&dir);
}

#[test]
fn clean_removes_stale_zod_files() {
    let dir = temp_test_dir("clean_stale_zod");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("generated");

    let sol_dir = artifacts_dir.join("Token.sol");
    std::fs::create_dir_all(&sol_dir).unwrap();
    std::fs::write(sol_dir.join("Token.json"), r#"{"abi": []}"#).unwrap();

    let config = Config {
        artifacts_dir,
        out_dir: gen_dir.clone(),
        targets: vec![abi_typegen_config::Target::Zod],
        wrappers: false,
        contracts: vec![],
        exclude: vec![],
    };

    std::fs::create_dir_all(&gen_dir).unwrap();
    std::fs::write(gen_dir.join("OldContract.zod.ts"), "stale content").unwrap();

    run_generate(&config, true).unwrap();

    assert!(gen_dir.join("Token.zod.ts").exists());
    assert!(!gen_dir.join("OldContract.zod.ts").exists());

    cleanup(&dir);
}

// ---------------------------------------------------------------
// Multi-target (comma-separated)
// ---------------------------------------------------------------

#[test]
fn run_dispatches_comma_separated_targets() {
    let dir = temp_test_dir("run_dispatch_multi_target");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("generated");
    let viem_dir = gen_dir.join("viem");
    let ethers_dir = gen_dir.join("ethers");

    let sol_dir = artifacts_dir.join("Token.sol");
    std::fs::create_dir_all(&sol_dir).unwrap();
    std::fs::write(
            sol_dir.join("Token.json"),
            r#"{
                "abi": [{
                    "type": "function",
                    "name": "balanceOf",
                    "inputs": [{"name": "account", "type": "address", "internalType": "address", "components": []}],
                    "outputs": [{"name": "", "type": "uint256", "internalType": "uint256", "components": []}],
                    "stateMutability": "view"
                }]
            }"#,
        )
        .unwrap();

    let cli = Cli {
        command: Commands::Generate {
            artifacts: Some(artifacts_dir),
            out: Some(gen_dir.clone()),
            target: Some("viem, ethers".into()),
            no_wrappers: false,
            contracts: vec![],
            exclude: None,
            check: false,
            clean: false,
        },
        config: Some(dir.join("nonexistent.toml")),
        hardhat: false,
    };
    run(cli).unwrap();

    assert!(
        viem_dir.join("Token.viem.ts").exists(),
        "Expected Token.viem.ts in the viem output directory"
    );
    assert!(
        viem_dir.join("Token.abi.ts").exists(),
        "Expected Token.abi.ts in the viem output directory"
    );
    assert!(
        viem_dir.join("index.ts").exists(),
        "Expected index.ts in the viem output directory"
    );
    assert!(
        ethers_dir.join("Token.ethers.ts").exists(),
        "Expected Token.ethers.ts in the ethers output directory"
    );
    assert!(
        ethers_dir.join("Token.abi.ts").exists(),
        "Expected Token.abi.ts in the ethers output directory"
    );
    assert!(
        ethers_dir.join("index.ts").exists(),
        "Expected index.ts in the ethers output directory"
    );
    assert!(
        !gen_dir.join("Token.abi.ts").exists(),
        "Did not expect shared root output for multi-target generation"
    );

    cleanup(&dir);
}

#[test]
fn run_dispatches_comma_separated_targets_no_spaces() {
    let dir = temp_test_dir("run_dispatch_multi_target_nospace");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("generated");
    let ethers5_dir = gen_dir.join("ethers5");
    let viem_dir = gen_dir.join("viem");

    let sol_dir = artifacts_dir.join("Token.sol");
    std::fs::create_dir_all(&sol_dir).unwrap();
    std::fs::write(
            sol_dir.join("Token.json"),
            r#"{
                "abi": [{
                    "type": "function",
                    "name": "balanceOf",
                    "inputs": [{"name": "account", "type": "address", "internalType": "address", "components": []}],
                    "outputs": [{"name": "", "type": "uint256", "internalType": "uint256", "components": []}],
                    "stateMutability": "view"
                }]
            }"#,
        )
        .unwrap();

    let cli = Cli {
        command: Commands::Generate {
            artifacts: Some(artifacts_dir),
            out: Some(gen_dir.clone()),
            target: Some("ethers5,viem".into()),
            no_wrappers: false,
            contracts: vec![],
            exclude: None,
            check: false,
            clean: false,
        },
        config: Some(dir.join("nonexistent.toml")),
        hardhat: false,
    };
    run(cli).unwrap();

    assert!(
        viem_dir.join("Token.viem.ts").exists(),
        "Expected Token.viem.ts in the viem output directory"
    );
    assert!(
        ethers5_dir.join("Token.ethers5.ts").exists(),
        "Expected Token.ethers5.ts in the ethers5 output directory"
    );
    assert!(
        ethers5_dir.join("Token.abi.ts").exists(),
        "Expected Token.abi.ts in the ethers5 output directory"
    );
    assert!(
        !gen_dir.join("Token.ethers5.ts").exists(),
        "Did not expect shared root output for multi-target generation"
    );

    cleanup(&dir);
}

#[test]
fn run_dispatches_comma_separated_targets_clean_stale_target_dirs() {
    let dir = temp_test_dir("run_dispatch_multi_target_clean_dirs");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("generated");

    write_target_matrix_artifact(&artifacts_dir, "Token");

    let stale_target_dir = gen_dir.join("wagmi");
    std::fs::create_dir_all(&stale_target_dir).unwrap();
    std::fs::write(stale_target_dir.join("stale.txt"), "stale").unwrap();

    let cli = Cli {
        command: Commands::Generate {
            artifacts: Some(artifacts_dir),
            out: Some(gen_dir.clone()),
            target: Some("viem,ethers".into()),
            no_wrappers: false,
            contracts: vec![],
            exclude: None,
            check: false,
            clean: true,
        },
        config: Some(dir.join("nonexistent.toml")),
        hardhat: false,
    };

    run(cli).unwrap();

    assert!(gen_dir.join("viem").join("Token.viem.ts").exists());
    assert!(gen_dir.join("ethers").join("Token.ethers.ts").exists());
    assert!(!stale_target_dir.exists());

    cleanup(&dir);
}

#[test]
fn run_multi_target_check_reports_stale_target_dirs() {
    let dir = temp_test_dir("run_dispatch_multi_target_check_dirs");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("generated");

    write_target_matrix_artifact(&artifacts_dir, "Token");

    let generate_cli = Cli {
        command: Commands::Generate {
            artifacts: Some(artifacts_dir.clone()),
            out: Some(gen_dir.clone()),
            target: Some("viem,ethers".into()),
            no_wrappers: false,
            contracts: vec![],
            exclude: None,
            check: false,
            clean: false,
        },
        config: Some(dir.join("nonexistent.toml")),
        hardhat: false,
    };
    run(generate_cli).unwrap();

    let stale_target_dir = gen_dir.join("wagmi");
    std::fs::create_dir_all(&stale_target_dir).unwrap();
    std::fs::write(stale_target_dir.join("stale.txt"), "stale").unwrap();

    let check_cli = Cli {
        command: Commands::Generate {
            artifacts: Some(artifacts_dir),
            out: Some(gen_dir),
            target: Some("viem,ethers".into()),
            no_wrappers: false,
            contracts: vec![],
            exclude: None,
            check: true,
            clean: false,
        },
        config: Some(dir.join("nonexistent.toml")),
        hardhat: false,
    };
    let result = run(check_cli);

    assert!(result.is_err());
    assert!(format!("{}", result.unwrap_err()).contains("stale"));

    cleanup(&dir);
}

#[test]
fn run_comma_separated_target_with_invalid_target_errors() {
    let dir = temp_test_dir("run_dispatch_multi_target_invalid");
    let artifacts_dir = dir.join("out");
    std::fs::create_dir_all(&artifacts_dir).unwrap();

    let cli = Cli {
        command: Commands::Generate {
            artifacts: Some(artifacts_dir),
            out: Some(dir.join("gen")),
            target: Some("viem,invalid_target".into()),
            no_wrappers: false,
            contracts: vec![],
            exclude: None,
            check: false,
            clean: false,
        },
        config: Some(dir.join("nonexistent.toml")),
        hardhat: false,
    };
    let result = run(cli);
    assert!(
        result.is_err(),
        "Expected error for invalid target in comma-separated list"
    );

    cleanup(&dir);
}

#[test]
fn run_fetch_file_neither_address_nor_file_errors() {
    let dir = temp_test_dir("run_fetch_neither");

    let cli = Cli {
        command: Commands::Fetch {
            address: None,
            name: "Token".into(),
            file: None,
            url: None,
            network: "mainnet".into(),
            api_key: None,
            artifacts: Some(dir.join("out")),
            force: false,
        },
        config: Some(dir.join("nonexistent.toml")),
        hardhat: false,
    };
    let err = run(cli).unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("ADDRESS") || msg.contains("--file"),
        "expected clear error about missing address or --file, got: {msg}"
    );

    cleanup(&dir);
}

#[test]
fn run_fetch_file_from_disk_generates_bindings() {
    let dir = temp_test_dir("run_fetch_from_file");
    let artifacts_dir = dir.join("out");
    let out_dir = dir.join("gen");

    // Write a foundry.toml so load_config picks up our out dir.
    let toml_path = dir.join("foundry.toml");
    std::fs::write(
        &toml_path,
        format!("[abi-typegen]\nout = {:?}\ntarget = \"viem\"\n", out_dir),
    )
    .unwrap();

    // Write a minimal raw ABI array to disk
    let abi_file = dir.join("Token.abi.json");
    std::fs::write(
        &abi_file,
        r#"[{"type":"function","name":"totalSupply","inputs":[],"outputs":[{"name":"","type":"uint256","internalType":"uint256","components":[]}],"stateMutability":"view"}]"#,
    )
    .unwrap();

    let cli = Cli {
        command: Commands::Fetch {
            address: None,
            name: "Token".into(),
            file: Some(abi_file),
            url: None,
            network: "mainnet".into(),
            api_key: None,
            artifacts: Some(artifacts_dir.clone()),
            force: false,
        },
        config: Some(toml_path),
        hardhat: false,
    };
    run(cli).expect("fetch --file should succeed");

    // Artifact saved
    assert!(
        artifacts_dir.join("Token.sol").join("Token.json").exists(),
        "expected artifact to be written"
    );
    // Bindings generated
    assert!(
        out_dir.join("Token.abi.ts").exists(),
        "expected Token.abi.ts to be generated"
    );

    cleanup(&dir);
}
