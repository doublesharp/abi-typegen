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
        package: abi_typegen_config::DEFAULT_PACKAGE.to_string(),
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
        None,
    )
    .unwrap();
    assert_eq!(cfg.artifacts_dir, PathBuf::from("custom-artifacts"));
}

#[test]
fn apply_overrides_out_overrides_out_dir() {
    let mut cfg = default_config();
    apply_overrides(
        &mut cfg,
        None,
        Some(PathBuf::from("my-types")),
        None,
        false,
        None,
    )
    .unwrap();
    assert_eq!(cfg.out_dir, PathBuf::from("my-types"));
}

#[test]
fn apply_overrides_target_viem() {
    let mut cfg = default_config();
    apply_overrides(&mut cfg, None, None, Some("viem".to_string()), false, None).unwrap();
    assert_eq!(*cfg.target(), abi_typegen_config::Target::Viem);
}

#[test]
fn apply_overrides_target_zod() {
    let mut cfg = default_config();
    apply_overrides(&mut cfg, None, None, Some("zod".to_string()), false, None).unwrap();
    assert_eq!(*cfg.target(), abi_typegen_config::Target::Zod);
}

#[test]
fn apply_overrides_target_ethers() {
    let mut cfg = default_config();
    apply_overrides(
        &mut cfg,
        None,
        None,
        Some("ethers".to_string()),
        false,
        None,
    )
    .unwrap();
    assert_eq!(*cfg.target(), abi_typegen_config::Target::Ethers);
}

#[test]
fn apply_overrides_target_solidity() {
    let mut cfg = default_config();
    apply_overrides(
        &mut cfg,
        None,
        None,
        Some("solidity".to_string()),
        false,
        None,
    )
    .unwrap();
    assert_eq!(*cfg.target(), abi_typegen_config::Target::Solidity);
}

#[test]
fn apply_overrides_unknown_target_returns_error() {
    let mut cfg = default_config();
    let result = apply_overrides(
        &mut cfg,
        None,
        None,
        Some("truffle".to_string()),
        false,
        None,
    );
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("truffle"));
}

#[test]
fn apply_overrides_sets_package() {
    let mut cfg = default_config();
    apply_overrides(
        &mut cfg,
        None,
        None,
        Some("kotlin".to_string()),
        false,
        Some("com.example.bindings".to_string()),
    )
    .unwrap();
    assert_eq!(cfg.package, "com.example.bindings");
}

#[test]
fn apply_overrides_rejects_package_invalid_for_cli_target() {
    // The config file's default target accepts any package; a CLI target must re-check it.
    let mut cfg = Config::from_toml_str("[abi-typegen]\npackage = \"com.example\"\n").unwrap();
    let result = apply_overrides(&mut cfg, None, None, Some("go".to_string()), false, None);
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("com.example"), "{msg}");
}

#[test]
fn apply_overrides_no_wrappers_sets_false() {
    let mut cfg = default_config();
    assert!(cfg.wrappers); // default is true
    apply_overrides(&mut cfg, None, None, None, true, None).unwrap();
    assert!(!cfg.wrappers);
}

#[test]
fn apply_overrides_no_flags_leaves_config_unchanged() {
    let mut cfg = default_config();
    let original_out = cfg.artifacts_dir.clone();
    let original_gen = cfg.out_dir.clone();
    let original_targets = cfg.targets.clone();
    let original_wrappers = cfg.wrappers;

    apply_overrides(&mut cfg, None, None, None, false, None).unwrap();

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
fn discover_artifacts_uses_contract_names_and_ignores_hardhat_debug_files() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("contracts/Bundle.sol");
    std::fs::create_dir_all(&source).unwrap();
    for name in ["Alpha.json", "Beta.json", "Beta.dbg.json", "notes.txt"] {
        std::fs::write(source.join(name), "{}").unwrap();
    }
    let found = discover_artifacts(dir.path(), &[]).unwrap();
    assert_eq!(
        found,
        vec![
            ("Alpha".into(), source.join("Alpha.json")),
            ("Beta".into(), source.join("Beta.json")),
        ]
    );
    assert_eq!(
        discover_artifacts(dir.path(), &["Beta".into()]).unwrap(),
        vec![("Beta".into(), source.join("Beta.json")),]
    );
}

#[test]
fn selected_artifacts_rejects_colliding_contract_names() {
    let dir = tempfile::tempdir().unwrap();
    for source in ["a/Token.sol", "b/Token.sol"] {
        let path = dir.path().join(source);
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("Token.json"), r#"{"abi": []}"#).unwrap();
    }
    let mut config = generated_config(
        dir.path().to_path_buf(),
        dir.path().join("gen"),
        Target::Viem,
    );
    let error = selected_artifacts(&config).unwrap_err().to_string();
    assert!(error.contains("Token"), "{error}");
    assert!(
        error.contains("a/Token.sol") && error.contains("b/Token.sol"),
        "{error}"
    );
    config.exclude = vec!["Token".into()];
    assert!(selected_artifacts(&config).unwrap().is_empty());
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
        package: abi_typegen_config::DEFAULT_PACKAGE.to_string(),
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
    assert!(viem.contains("export function getTokenContract<TClient extends Client>("));

    let barrel = std::fs::read_to_string(gen_dir.join("index.ts")).unwrap();
    assert!(barrel.contains("export * from './Token.abi.js'"));
    assert!(barrel.contains("export * from './Token.viem.js'"));

    cleanup(&dir);
}

#[test]
fn run_generate_non_ts_target_emits_no_barrel() {
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

    // A TypeScript barrel (`index.ts`) is meaningless for a Python target and
    // must not be written. Regression test for a stray, content-free index.ts.
    assert!(
        !gen_dir.join("index.ts").exists(),
        "non-TS (Python) target must not emit index.ts"
    );

    cleanup(&dir);
}

#[test]
fn run_generate_clean_removes_stale_barrel_for_non_ts_target() {
    // Upgrade scenario: an earlier version wrote a stray `index.ts` alongside a
    // Python target's output. Regenerating with `clean = true` must remove it.
    let dir = temp_test_dir("run_generate_clean_stale_barrel");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("generated");

    write_target_matrix_artifact(&artifacts_dir, "Token");
    std::fs::create_dir_all(&gen_dir).unwrap();
    // Simulate a leftover barrel from a previous (buggy) generation.
    std::fs::write(
        gen_dir.join("index.ts"),
        "// Auto-generated by abi-typegen. Do not edit manually.\n\n",
    )
    .unwrap();

    let config = generated_config(
        artifacts_dir,
        gen_dir.clone(),
        abi_typegen_config::Target::Python,
    );

    run_generate(&config, true).unwrap();

    assert!(gen_dir.join("Token.py").exists());
    assert!(
        !gen_dir.join("index.ts").exists(),
        "stale index.ts must be cleaned for a non-TS target"
    );

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
            marker_text: "export function getTokenContract<TClient extends Client>(",
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
            marker_text: "export type TokenContract = Omit<BaseContract, keyof TokenMethods | 'connect'> &",
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
            expected_files: &["Token.py"],
            absent_files: &[
                "index.ts",
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
            expected_files: &["Token.go"],
            absent_files: &[
                "index.ts",
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
            expected_files: &["token.rs", "mod.rs"],
            absent_files: &[
                "index.ts",
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
            marker_file: "token.rs",
            marker_text: "alloy::sol! {",
        },
        TargetCase {
            name: "swift",
            target: abi_typegen_config::Target::Swift,
            expected_files: &["Token.swift"],
            absent_files: &[
                "index.ts",
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
            marker_text: "public struct BalanceOfParams: Sendable, Hashable {",
        },
        TargetCase {
            name: "csharp",
            target: abi_typegen_config::Target::CSharp,
            expected_files: &["Token.cs"],
            absent_files: &[
                "index.ts",
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
            expected_files: &["Token.kt"],
            absent_files: &[
                "index.ts",
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
            marker_text: "data class BalanceOfParams(",
        },
        TargetCase {
            name: "solidity",
            target: abi_typegen_config::Target::Solidity,
            expected_files: &["IToken.sol"],
            absent_files: &[
                "index.ts",
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
            expected_files: &["Token.yaml"],
            absent_files: &[
                "index.ts",
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
        package: abi_typegen_config::DEFAULT_PACKAGE.to_string(),
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
        package: abi_typegen_config::DEFAULT_PACKAGE.to_string(),
    };

    // Should succeed but not create any files
    run_generate(&config, false).unwrap();
    assert!(!dir.join("gen").join("index.ts").exists());

    cleanup(&dir);
}

#[test]
fn invalid_artifact_fails_without_changing_existing_output() {
    let dir = tempfile::tempdir().unwrap();
    let config = generated_config(dir.path().join("out"), dir.path().join("gen"), Target::Viem);
    write_target_matrix_artifact(&config.artifacts_dir, "Alpha");
    write_target_matrix_artifact(&config.artifacts_dir, "ZBad");
    std::fs::write(config.artifacts_dir.join("ZBad.sol/ZBad.json"), "invalid").unwrap();
    std::fs::create_dir_all(&config.out_dir).unwrap();
    let existing = config.out_dir.join("Alpha.abi.ts");
    std::fs::write(&existing, "previous bindings").unwrap();
    std::fs::write(config.out_dir.join("Old.abi.ts"), "keep on failure").unwrap();
    let error = run_generate(&config, true).unwrap_err().to_string();
    assert!(error.contains("ZBad.json"), "{error}");
    assert_eq!(
        std::fs::read_to_string(existing).unwrap(),
        "previous bindings"
    );
    assert!(config.out_dir.join("Old.abi.ts").exists());
    assert!(run_check(&config).is_err());
    let artifacts = selected_artifacts(&config).unwrap();
    assert!(collect_diff_entries(&config, &artifacts).is_err());
    assert!(collect_json_summaries(&config).is_err());
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
        package: abi_typegen_config::DEFAULT_PACKAGE.to_string(),
    };

    run_generate(&config, false).unwrap();

    assert!(gen_dir.join("Alpha.abi.ts").exists());
    assert!(!gen_dir.join("Beta.abi.ts").exists());

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
            package: None,
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
            package: None,
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
            package: None,
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

#[test]
fn run_dispatches_diff_with_overrides() {
    let dir = temp_test_dir("run_dispatch_diff_override");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("types");

    write_target_matrix_artifact(&artifacts_dir, "Token");
    let mut config = generated_config(
        artifacts_dir.clone(),
        gen_dir.clone(),
        abi_typegen_config::Target::Viem,
    );
    config.wrappers = false;
    run_generate(&config, false).unwrap();

    let cli = Cli {
        command: Commands::Diff {
            artifacts: Some(artifacts_dir),
            out: Some(gen_dir),
            target: Some("viem".into()),
            no_wrappers: true,
            package: None,
            contracts: vec!["Token".into()],
            exclude: None,
        },
        config: Some(dir.join("nonexistent.toml")),
        hardhat: false,
    };

    run(cli).unwrap();
    cleanup(&dir);
}

#[test]
fn run_dispatches_json_with_overrides() {
    let dir = temp_test_dir("run_dispatch_json_override");
    let artifacts_dir = dir.join("out");

    write_target_matrix_artifact(&artifacts_dir, "Token");
    write_target_matrix_artifact(&artifacts_dir, "SkipMe");

    let cli = Cli {
        command: Commands::Json {
            artifacts: Some(artifacts_dir),
            contracts: vec!["Token".into()],
            exclude: Some("*Skip*".into()),
            pretty: true,
        },
        config: Some(dir.join("nonexistent.toml")),
        hardhat: false,
    };

    run(cli).unwrap();
    cleanup(&dir);
}

#[test]
fn run_fetch_rejects_existing_artifact_without_force() {
    let dir = temp_test_dir("run_fetch_existing_artifact");
    let artifacts_dir = dir.join("out");
    let abi_path = dir.join("Token.abi.json");
    std::fs::write(&abi_path, r#"[]"#).unwrap();

    let dest = fetch::artifact_path(&artifacts_dir, "Token");
    std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
    std::fs::write(&dest, r#"{"abi":[]}"#).unwrap();

    let result = run_fetch(FetchSource::File(&abi_path), "Token", &artifacts_dir, false);

    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("artifact already exists"));
    assert!(msg.contains("--force"));

    cleanup(&dir);
}

#[test]
fn fetch_source_file_takes_priority_over_network_fields() {
    let dir = temp_test_dir("fetch_source_file_priority");
    let abi_path = dir.join("Token.abi.json");
    std::fs::write(&abi_path, r#"[]"#).unwrap();

    let source = FetchSource::resolve(
        Some("0x0000000000000000000000000000000000000000"),
        Some(&abi_path),
        "unknown-network",
        None,
        Some("cli-key".into()),
    )
    .unwrap();

    match source {
        FetchSource::File(path) => assert_eq!(path, abi_path.as_path()),
        FetchSource::Network { .. } => panic!("expected file source to win"),
    }

    cleanup(&dir);
}

#[test]
fn resolve_config_path_handles_explicit_hardhat_and_default_modes() {
    let explicit = Some(PathBuf::from("custom.toml"));
    assert_eq!(
        resolve_config_path(&explicit, true),
        PathBuf::from("custom.toml")
    );
    assert_eq!(
        resolve_config_path(&None, true),
        PathBuf::from("hardhat.config.ts")
    );
    assert_eq!(
        resolve_config_path(&None, false),
        PathBuf::from("foundry.toml")
    );
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
        package: abi_typegen_config::DEFAULT_PACKAGE.to_string(),
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
        package: abi_typegen_config::DEFAULT_PACKAGE.to_string(),
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
        package: abi_typegen_config::DEFAULT_PACKAGE.to_string(),
    };

    let (tx, rx) = std::sync::mpsc::channel();
    // Send an error event, then drop sender
    tx.send(Err(notify::Error::generic("test error"))).unwrap();
    drop(tx);

    watch_loop(&config, &rx);
    // Should handle error gracefully and exit
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
        package: abi_typegen_config::DEFAULT_PACKAGE.to_string(),
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
        package: abi_typegen_config::DEFAULT_PACKAGE.to_string(),
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
        package: abi_typegen_config::DEFAULT_PACKAGE.to_string(),
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
        package: abi_typegen_config::DEFAULT_PACKAGE.to_string(),
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
        package: abi_typegen_config::DEFAULT_PACKAGE.to_string(),
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
        package: abi_typegen_config::DEFAULT_PACKAGE.to_string(),
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
        package: abi_typegen_config::DEFAULT_PACKAGE.to_string(),
    };

    run_generate(&config, false).unwrap();
    std::fs::write(gen_dir.join("OldToken.abi.ts"), "stale content").unwrap();

    let artifacts = selected_artifacts(&config).unwrap();
    let diffs = collect_diff_entries(&config, &artifacts).unwrap();
    assert!(diffs.contains(&"D OldToken.abi.ts".to_string()));

    cleanup(&dir);
}

#[test]
fn collect_diff_entries_omits_barrel_for_non_ts_target() {
    let dir = temp_test_dir("diff_entries_non_ts_barrel");
    let artifacts_dir = dir.join("out");
    let gen_dir = dir.join("generated");

    write_target_matrix_artifact(&artifacts_dir, "Token");

    let config = generated_config(
        artifacts_dir,
        gen_dir.clone(),
        abi_typegen_config::Target::Python,
    );

    let artifacts = selected_artifacts(&config).unwrap();
    let diffs = collect_diff_entries(&config, &artifacts).unwrap();

    // The Python file is reported as added, but no `index.ts` barrel exists for
    // a non-TS target, so it must not appear in the diff.
    assert!(diffs.contains(&"A Token.py".to_string()));
    assert!(
        !diffs.iter().any(|d| d.contains("index.ts")),
        "non-TS target diff must not mention index.ts, got: {:?}",
        diffs
    );

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
        package: abi_typegen_config::DEFAULT_PACKAGE.to_string(),
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
        package: abi_typegen_config::DEFAULT_PACKAGE.to_string(),
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
        package: abi_typegen_config::DEFAULT_PACKAGE.to_string(),
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
        package: abi_typegen_config::DEFAULT_PACKAGE.to_string(),
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
        package: abi_typegen_config::DEFAULT_PACKAGE.to_string(),
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
        package: abi_typegen_config::DEFAULT_PACKAGE.to_string(),
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
        package: abi_typegen_config::DEFAULT_PACKAGE.to_string(),
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
            package: None,
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
            package: None,
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
            package: None,
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
            package: None,
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
            package: None,
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
            package: None,
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

#[test]
fn clean_empty_selection_removes_stale_outputs_including_yaml() {
    let dir = tempfile::tempdir().unwrap();
    let config = generated_config(dir.path().join("out"), dir.path().join("gen"), Target::Yaml);
    std::fs::create_dir_all(&config.artifacts_dir).unwrap();
    std::fs::create_dir_all(&config.out_dir).unwrap();
    for name in ["Old.yaml", "Old.abi.ts", "index.ts", "README.md"] {
        std::fs::write(config.out_dir.join(name), "old").unwrap();
    }
    assert_eq!(
        collect_diff_entries(&config, &[]).unwrap(),
        vec!["D Old.abi.ts", "D Old.yaml", "D index.ts"]
    );
    run_generate(&config, true).unwrap();
    assert_missing_files(&config.out_dir, &["Old.yaml", "Old.abi.ts", "index.ts"]);
    assert_eq!(
        std::fs::read_to_string(config.out_dir.join("README.md")).unwrap(),
        "old"
    );
    run_check(&config).unwrap();
}

#[test]
fn regeneration_preserves_unchanged_files_and_updates_changed_files() {
    let dir = tempfile::tempdir().unwrap();
    let config = generated_config(dir.path().join("out"), dir.path().join("gen"), Target::Viem);
    write_target_matrix_artifact(&config.artifacts_dir, "Token");
    run_generate(&config, false).unwrap();
    let old_time = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
    for name in ["Token.abi.ts", "Token.viem.ts", "index.ts"] {
        let file = std::fs::File::options()
            .write(true)
            .open(config.out_dir.join(name))
            .unwrap();
        file.set_modified(old_time).unwrap();
    }
    run_generate(&config, false).unwrap();
    for name in ["Token.abi.ts", "Token.viem.ts", "index.ts"] {
        assert_eq!(
            std::fs::metadata(config.out_dir.join(name))
                .unwrap()
                .modified()
                .unwrap(),
            old_time,
            "rewrote {name}"
        );
    }
    std::fs::write(
        config.artifacts_dir.join("Token.sol/Token.json"),
        r#"{"abi": []}"#,
    )
    .unwrap();
    run_generate(&config, false).unwrap();
    assert_file_contains(&config.out_dir.join("Token.abi.ts"), "[] as const");
    assert_eq!(
        std::fs::metadata(config.out_dir.join("index.ts"))
            .unwrap()
            .modified()
            .unwrap(),
        old_time
    );
}

#[test]
fn output_filename_collisions_fail_before_writing() {
    let dir = tempfile::tempdir().unwrap();
    let config = generated_config(
        dir.path().join("out"),
        dir.path().join("gen"),
        Target::Solidity,
    );
    for name in ["Token", "IToken"] {
        write_target_matrix_artifact(&config.artifacts_dir, name);
    }
    std::fs::create_dir_all(&config.out_dir).unwrap();
    let output = config.out_dir.join("IToken.sol");
    std::fs::write(&output, "previous interface").unwrap();
    let error = run_generate(&config, true).unwrap_err().to_string();
    assert!(
        error.contains("IToken.sol") && error.contains("Token"),
        "{error}"
    );
    assert_eq!(
        std::fs::read_to_string(output).unwrap(),
        "previous interface"
    );
    assert!(run_check(&config).is_err());
    assert!(collect_diff_entries(&config, &selected_artifacts(&config).unwrap()).is_err());
}

#[test]
fn watch_regenerates_every_configured_target() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = generated_config(dir.path().join("out"), dir.path().join("gen"), Target::Viem);
    config.targets = vec![Target::Viem, Target::Python];
    write_target_matrix_artifact(&config.artifacts_dir, "Token");
    let (tx, rx) = std::sync::mpsc::channel();
    tx.send(Ok(notify::Event::new(notify::EventKind::Modify(
        notify::event::ModifyKind::Data(notify::event::DataChange::Any),
    ))))
    .unwrap();
    drop(tx);
    watch_loop(&config, &rx);
    assert_file_contains(
        &config.out_dir.join("viem/Token.viem.ts"),
        "getTokenContract",
    );
    assert_file_contains(&config.out_dir.join("python/Token.py"), "balance_of");
    assert_missing_files(&config.out_dir, &["Token.abi.ts", "index.ts"]);
    run_check(&config).unwrap();
}

#[test]
fn fetch_rejects_invalid_abi_before_overwriting_artifact() {
    let dir = tempfile::tempdir().unwrap();
    let artifacts = dir.path().join("out");
    write_target_matrix_artifact(&artifacts, "Token");
    let path = artifacts.join("Token.sol/Token.json");
    let previous = std::fs::read(&path).unwrap();
    let invalid = dir.path().join("invalid.json");
    std::fs::write(
        &invalid,
        r#"[{"type":"function","name":"f","inputs":[{"type":"uint257"}]}]"#,
    )
    .unwrap();
    assert!(run_fetch(FetchSource::File(&invalid), "Token", &artifacts, true).is_err());
    assert_eq!(std::fs::read(path).unwrap(), previous);
}

#[test]
fn multi_target_check_and_diff_agree_with_generation() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = generated_config(dir.path().join("out"), dir.path().join("gen"), Target::Viem);
    config.targets = vec![Target::Viem, Target::Python];
    write_target_matrix_artifact(&config.artifacts_dir, "Token");
    run_generate(&config, false).unwrap();
    assert!(config.out_dir.join("python/Token.py").is_file());
    run_check(&config).unwrap();
    let artifacts = selected_artifacts(&config).unwrap();
    assert!(
        collect_diff_entries(&config, &artifacts)
            .unwrap()
            .is_empty()
    );
    std::fs::write(config.out_dir.join("python/Token.py"), "stale").unwrap();
    std::fs::write(config.out_dir.join("viem/Gone.abi.ts"), "stale").unwrap();
    assert_eq!(
        collect_diff_entries(&config, &artifacts).unwrap(),
        vec!["D viem/Gone.abi.ts", "M python/Token.py"]
    );
    assert!(run_check(&config).is_err());
    run_generate(&config, true).unwrap();
    run_check(&config).unwrap();
}

#[test]
fn multi_target_render_error_preserves_every_target() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = generated_config(dir.path().join("out"), dir.path().join("gen"), Target::Viem);
    config.targets = vec![Target::Viem, Target::Solidity];
    for name in ["Token", "IToken"] {
        write_target_matrix_artifact(&config.artifacts_dir, name);
    }
    std::fs::create_dir_all(config.out_dir.join("viem")).unwrap();
    let old = config.out_dir.join("viem/Token.abi.ts");
    std::fs::write(&old, "previous").unwrap();
    assert!(run_generate(&config, true).is_err());
    assert_eq!(std::fs::read_to_string(old).unwrap(), "previous");
}

#[test]
fn glob_matching_agrees_with_independent_dynamic_programming_oracle() {
    fn words(alphabet: &[u8], max_length: usize) -> Vec<Vec<u8>> {
        let mut all = vec![vec![]];
        let mut layer = all.clone();
        for _ in 0..max_length {
            layer = layer
                .iter()
                .flat_map(|prefix| {
                    alphabet.iter().map(move |&byte| {
                        let mut word = prefix.clone();
                        word.push(byte);
                        word
                    })
                })
                .collect();
            all.extend(layer.clone());
        }
        all
    }
    fn oracle(name: &[u8], pattern: &[u8]) -> bool {
        let mut table = vec![vec![false; pattern.len() + 1]; name.len() + 1];
        table[0][0] = true;
        for j in 1..=pattern.len() {
            table[0][j] = pattern[j - 1] == b'*' && table[0][j - 1];
            for i in 1..=name.len() {
                table[i][j] = match pattern[j - 1] {
                    b'*' => table[i][j - 1] || table[i - 1][j],
                    b'?' => table[i - 1][j - 1],
                    byte => byte == name[i - 1] && table[i - 1][j - 1],
                };
            }
        }
        table[name.len()][pattern.len()]
    }
    let patterns = words(b"ab*?", 5);
    for name in words(b"ab", 5) {
        for pattern in &patterns {
            assert_eq!(
                matches_glob(
                    std::str::from_utf8(&name).unwrap(),
                    std::str::from_utf8(pattern).unwrap()
                ),
                oracle(&name, pattern),
                "name={name:?}, pattern={pattern:?}"
            );
        }
    }
}

#[test]
fn rust_mod_contract_does_not_overwrite_module_index() {
    let dir = tempfile::tempdir().unwrap();
    let config = generated_config(dir.path().join("out"), dir.path().join("gen"), Target::Rust);
    write_target_matrix_artifact(&config.artifacts_dir, "Mod");
    run_generate(&config, false).unwrap();
    assert_file_contains(&config.out_dir.join("mod.rs"), "pub mod mod_;");
    assert_file_contains(&config.out_dir.join("mod_.rs"), "contract Mod");
    run_check(&config).unwrap();
}

#[test]
fn native_namespace_collisions_fail_before_writing() {
    for target in [Target::Swift, Target::Kotlin] {
        let dir = tempfile::tempdir().unwrap();
        let config = generated_config(dir.path().join("out"), dir.path().join("gen"), target);
        for name in ["String", "String2"] {
            write_target_matrix_artifact(&config.artifacts_dir, name);
        }
        std::fs::create_dir_all(&config.out_dir).unwrap();
        let previous = config.out_dir.join("String.swift");
        std::fs::write(&previous, "previous bindings").unwrap();
        let error = run_generate(&config, true).unwrap_err().to_string();
        assert!(error.contains("namespace 'String2'"), "{error}");
        assert_eq!(
            std::fs::read_to_string(previous).unwrap(),
            "previous bindings"
        );
    }
}

#[test]
fn php_contract_classes_collide_case_insensitively() {
    let dir = tempfile::tempdir().unwrap();
    let config = generated_config(dir.path().join("out"), dir.path().join("gen"), Target::Php);
    // Separate source paths keep this test valid on case-insensitive filesystems.
    let first = dir.path().join("first.json");
    let second = dir.path().join("second.json");
    std::fs::write(&first, TARGET_MATRIX_ARTIFACT_JSON).unwrap();
    std::fs::write(&second, TARGET_MATRIX_ARTIFACT_JSON).unwrap();
    let artifacts = vec![("Token".to_string(), first), ("TOKEN".to_string(), second)];
    let error = render_artifacts(&config, &artifacts)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("Token") && error.contains("TOKEN"),
        "{error}"
    );
}

#[test]
fn php_auxiliary_class_collisions_fail_before_writing() {
    let dir = tempfile::tempdir().unwrap();
    let config = generated_config(dir.path().join("out"), dir.path().join("gen"), Target::Php);
    for name in ["Token", "TokenClient"] {
        write_target_matrix_artifact(&config.artifacts_dir, name);
    }
    std::fs::create_dir_all(&config.out_dir).unwrap();
    let existing = config.out_dir.join("Token.php");
    std::fs::write(&existing, "previous bindings").unwrap();
    let error = run_generate(&config, true).unwrap_err().to_string();
    assert!(
        error.contains("TokenClient") && error.contains("Token"),
        "{error}"
    );
    assert_eq!(
        std::fs::read_to_string(existing).unwrap(),
        "previous bindings"
    );
    assert!(run_check(&config).is_err());
}

#[test]
fn php_no_wrappers_keeps_distinct_contract_classes_without_auxiliaries() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = generated_config(dir.path().join("out"), dir.path().join("gen"), Target::Php);
    config.wrappers = false;
    for name in ["Token", "TokenClient"] {
        write_target_matrix_artifact(&config.artifacts_dir, name);
    }
    run_generate(&config, false).unwrap();
    assert_file_contains(&config.out_dir.join("Token.php"), "final class Token");
    assert_file_contains(
        &config.out_dir.join("TokenClient.php"),
        "final class TokenClient",
    );
    run_check(&config).unwrap();
}

#[test]
fn c_and_cpp_share_one_runtime_header_and_keep_types_without_wrappers() {
    let dir = temp_test_dir("native_c_targets");
    let artifacts_dir = dir.join("out");
    for name in ["Token", "Vault", "String"] {
        let folder = artifacts_dir.join(format!("{name}.sol"));
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join(format!("{name}.json")), r#"{"abi":[{"type":"function","name":"read","inputs":[],"outputs":[{"name":"amount","type":"uint256"}],"stateMutability":"view"}]}"#).unwrap();
    }
    let config = Config {
        artifacts_dir,
        out_dir: dir.join("generated"),
        targets: vec![Target::C, Target::Cpp],
        wrappers: false,
        contracts: vec![],
        exclude: vec![],
        package: abi_typegen_config::DEFAULT_PACKAGE.into(),
    };
    run_generate(&config, false).unwrap();
    for target in ["c", "cpp"] {
        let out = config.out_dir.join(target);
        assert!(out.join("abi_typegen.h").exists());
        let header = std::fs::read_to_string(out.join("atg_Token.h")).unwrap();
        assert!(header.contains("atg_Token_atg_read_returns"));
        assert!(!header.contains("client->transport"));
        assert!(out.join("atg_Vault.h").exists());
        assert!(out.join("atg_String.h").exists());
        assert!(!out.join("String.h").exists());
    }
    assert!(config.out_dir.join("cpp/atg_Token.hpp").exists());
    cleanup(&dir);
}

#[test]
fn native_headers_do_not_shadow_runtime_or_standard_headers() {
    let dir = temp_test_dir("native_runtime_collision");
    let artifacts_dir = dir.join("out");
    let folder = artifacts_dir.join("abi_typegen.sol");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("abi_typegen.json"), r#"{"abi":[]}"#).unwrap();
    let config = Config {
        artifacts_dir,
        out_dir: dir.join("generated"),
        targets: vec![Target::C],
        wrappers: true,
        contracts: vec![],
        exclude: vec![],
        package: abi_typegen_config::DEFAULT_PACKAGE.into(),
    };
    run_generate(&config, false).unwrap();
    assert!(config.out_dir.join("atg_abi_typegen.h").exists());
    assert!(config.out_dir.join("abi_typegen.h").exists());
    cleanup(&dir);
}

#[test]
fn cobol_target_emits_bridge_runtime_and_preserves_metadata_without_wrappers() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = generated_config(
        dir.path().join("out"),
        dir.path().join("gen"),
        Target::Cobol,
    );
    let folder = config.artifacts_dir.join("Token.sol");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("Token.json"), r#"{"abi":[{"type":"function","name":"balanceOf","inputs":[{"name":"owner","type":"address"}],"outputs":[{"name":"balance","type":"uint256"}],"stateMutability":"view"}]}"#).unwrap();
    run_generate(&config, false).unwrap();
    assert_file_contains(&config.out_dir.join("Token.cob"), "TOKEN-BALANCE-OF-CALL");
    assert_file_contains(&config.out_dir.join("Token.cobol.c"), "atg_encode(");
    assert_file_contains(&config.out_dir.join("abi_typegen.h"), "ATG_ABI_VERSION");
    run_check(&config).unwrap();
    config.wrappers = false;
    run_generate(&config, false).unwrap();
    assert_file_contains(&config.out_dir.join("Token.cob"), "balanceOf(address)");
    assert!(
        !std::fs::read_to_string(config.out_dir.join("Token.cob"))
            .unwrap()
            .contains("PROGRAM-ID")
    );
    assert!(
        !std::fs::read_to_string(config.out_dir.join("Token.cobol.c"))
            .unwrap()
            .contains("atg_encode(")
    );
    run_check(&config).unwrap();
}

#[test]
fn cobol_normalized_contract_collisions_preserve_existing_output() {
    let dir = tempfile::tempdir().unwrap();
    let config = generated_config(
        dir.path().join("out"),
        dir.path().join("gen"),
        Target::Cobol,
    );
    for name in ["FooBar", "Foo_Bar"] {
        write_target_matrix_artifact(&config.artifacts_dir, name);
    }
    std::fs::create_dir_all(&config.out_dir).unwrap();
    let existing = config.out_dir.join("FooBar.cob");
    std::fs::write(&existing, "previous bindings").unwrap();
    assert!(
        run_generate(&config, true)
            .unwrap_err()
            .to_string()
            .contains("namespace")
    );
    assert_eq!(
        std::fs::read_to_string(existing).unwrap(),
        "previous bindings"
    );
}

#[test]
fn cobol_program_collisions_across_contracts_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let config = generated_config(
        dir.path().join("out"),
        dir.path().join("gen"),
        Target::Cobol,
    );
    for (contract, function) in [("FooBar", "read"), ("Foo", "barRead")] {
        let folder = config.artifacts_dir.join(format!("{contract}.sol"));
        std::fs::create_dir_all(&folder).unwrap();
        let artifact = serde_json::json!({"abi":[{"type":"function","name":function,"inputs":[{"name":"owner","type":"address"}],"outputs":[{"name":"balance","type":"uint256"}],"stateMutability":"view"}]});
        std::fs::write(
            folder.join(format!("{contract}.json")),
            artifact.to_string(),
        )
        .unwrap();
    }
    assert!(
        run_generate(&config, true)
            .unwrap_err()
            .to_string()
            .contains("program:FOO-BAR-READ")
    );
    assert!(!config.out_dir.exists());
}

#[test]
fn ruby_and_shell_keep_metadata_without_callable_wrappers() {
    for (target, extension, marker, runtime_marker) in [
        (Target::Ruby, "rb", "ABI", "class TokenContract"),
        (Target::Shell, "sh", "ATG_TOKEN_ABI", "atg_token_"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut config = generated_config(dir.path().join("out"), dir.path().join("gen"), target);
        write_target_matrix_artifact(&config.artifacts_dir, "Token");
        run_generate(&config, false).unwrap();
        let file = config.out_dir.join(format!("Token.{extension}"));
        assert_file_contains(&file, marker);
        assert_file_contains(&file, runtime_marker);
        run_check(&config).unwrap();
        config.wrappers = false;
        run_generate(&config, false).unwrap();
        assert_file_contains(&file, marker);
        assert!(
            !std::fs::read_to_string(file)
                .unwrap()
                .contains(runtime_marker)
        );
        run_check(&config).unwrap();
    }
}

#[test]
fn shell_normalized_contract_collisions_fail_before_writing() {
    let dir = tempfile::tempdir().unwrap();
    let config = generated_config(
        dir.path().join("out"),
        dir.path().join("gen"),
        Target::Shell,
    );
    for name in ["FooBar", "Foo_Bar"] {
        write_target_matrix_artifact(&config.artifacts_dir, name);
    }
    assert!(
        run_generate(&config, true)
            .unwrap_err()
            .to_string()
            .contains("namespace")
    );
    assert!(!config.out_dir.exists());
}

#[test]
fn ruby_tuple_and_other_contract_constants_cannot_collide() {
    let dir = tempfile::tempdir().unwrap();
    let config = generated_config(dir.path().join("out"), dir.path().join("gen"), Target::Ruby);
    write_target_matrix_artifact(&config.artifacts_dir, "TokenOther");
    let folder = config.artifacts_dir.join("Token.sol");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("Token.json"), r#"{"abi":[{"type":"function","name":"read","inputs":[{"name":"item","type":"tuple","internalType":"struct Token.OtherContract","components":[{"name":"amount","type":"uint256"}]}],"outputs":[],"stateMutability":"view"}]}"#).unwrap();
    let error = run_generate(&config, true).unwrap_err().to_string();
    assert!(error.contains("TokenOtherContract"), "{error}");
    assert!(!config.out_dir.exists());
}

#[test]
fn shell_global_symbol_collisions_are_rejected_across_contracts() {
    let dir = tempfile::tempdir().unwrap();
    let config = generated_config(
        dir.path().join("out"),
        dir.path().join("gen"),
        Target::Shell,
    );
    for (contract, function) in [("FooBar", "baz"), ("Foo", "barBaz")] {
        let folder = config.artifacts_dir.join(format!("{contract}.sol"));
        std::fs::create_dir_all(&folder).unwrap();
        let artifact = serde_json::json!({"abi":[{"type":"function","name":function,"inputs":[],"outputs":[{"name":"balance","type":"uint256"}],"stateMutability":"view"}]});
        std::fs::write(
            folder.join(format!("{contract}.json")),
            artifact.to_string(),
        )
        .unwrap();
    }
    let error = run_generate(&config, true).unwrap_err().to_string();
    assert!(
        error.contains("FOO_BAR_BAZ") || error.contains("foo_bar_baz"),
        "{error}"
    );
    assert!(!config.out_dir.exists());
}

#[test]
fn elixir_metadata_mode_omits_sdk_dependency_and_preserves_abi() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = generated_config(
        dir.path().join("out"),
        dir.path().join("gen"),
        Target::Elixir,
    );
    write_target_matrix_artifact(&config.artifacts_dir, "Token");
    run_generate(&config, false).unwrap();
    let file = config.out_dir.join("token.ex");
    assert_file_contains(&file, "use Ethers.Contract");
    config.wrappers = false;
    run_generate(&config, false).unwrap();
    assert_file_contains(&file, "def abi_json");
    assert!(
        !std::fs::read_to_string(file)
            .unwrap()
            .contains("use Ethers.Contract")
    );
    run_check(&config).unwrap();
}

#[test]
fn elixir_reserved_module_normalization_cannot_overwrite_another_contract() {
    let dir = tempfile::tempdir().unwrap();
    let config = generated_config(
        dir.path().join("out"),
        dir.path().join("gen"),
        Target::Elixir,
    );
    for name in ["String", "StringContract"] {
        write_target_matrix_artifact(&config.artifacts_dir, name);
    }
    let error = run_generate(&config, true).unwrap_err().to_string();
    assert!(error.contains("StringContract"), "{error}");
    assert!(!config.out_dir.exists());
}

#[test]
fn elixir_event_filter_shadowing_fails_before_writing() {
    let dir = tempfile::tempdir().unwrap();
    let config = generated_config(
        dir.path().join("out"),
        dir.path().join("gen"),
        Target::Elixir,
    );
    let folder = config.artifacts_dir.join("Events.sol");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("Events.json"), r#"{"abi":[{"type":"event","name":"FooBar","inputs":[{"name":"x","type":"uint256","indexed":true}],"anonymous":false},{"type":"event","name":"foo_bar","inputs":[{"name":"x","type":"address","indexed":true}],"anonymous":false}]}"#)
        .unwrap();
    let error = run_generate(&config, false).unwrap_err().to_string();
    assert!(
        error.contains("event filter name/arity collision"),
        "{error}"
    );
    assert!(!config.out_dir.exists());
}

#[test]
fn elixir_sdk_helper_name_collision_fails_before_writing() {
    let dir = tempfile::tempdir().unwrap();
    let config = generated_config(
        dir.path().join("out"),
        dir.path().join("gen"),
        Target::Elixir,
    );
    let folder = config.artifacts_dir.join("Helpers.sol");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("Helpers.json"), r#"{"abi":[{"type":"function","name":"__default_address__","inputs":[],"outputs":[{"name":"","type":"address"}],"stateMutability":"view"}]}"#)
        .unwrap();
    let error = run_generate(&config, false).unwrap_err().to_string();
    assert!(error.contains("SDK helper name/arity collision"), "{error}");
    assert!(!config.out_dir.exists());
}

#[test]
fn elixir_event_filter_helper_name_collision_fails_before_writing() {
    let dir = tempfile::tempdir().unwrap();
    let config = generated_config(
        dir.path().join("out"),
        dir.path().join("gen"),
        Target::Elixir,
    );
    let folder = config.artifacts_dir.join("Events.sol");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(
        folder.join("Events.json"),
        r#"{"abi":[{"type":"event","name":"__all__","inputs":[],"anonymous":false}]}"#,
    )
    .unwrap();
    let error = run_generate(&config, false).unwrap_err().to_string();
    assert!(
        error.contains("event-filter helper name/arity collision"),
        "{error}"
    );
    assert!(!config.out_dir.exists());
}

#[test]
fn elixir_ambiguous_event_overload_fails_before_writing_but_keeps_metadata_mode() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = generated_config(
        dir.path().join("out"),
        dir.path().join("gen"),
        Target::Elixir,
    );
    let folder = config.artifacts_dir.join("Events.sol");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(
        folder.join("Events.json"),
        r#"{"abi":[{"type":"event","name":"Changed","inputs":[{"name":"owner","type":"address","indexed":true},{"name":"amount","type":"uint256","indexed":false}],"anonymous":false},{"type":"event","name":"Changed","inputs":[{"name":"owner","type":"address","indexed":true},{"name":"label","type":"string","indexed":false}],"anonymous":false}]}"#,
    )
    .unwrap();
    let error = run_generate(&config, false).unwrap_err().to_string();
    assert!(error.contains("event filter overload ambiguity"), "{error}");
    assert!(!config.out_dir.exists());

    config.wrappers = false;
    run_generate(&config, false).unwrap();
    assert_file_contains(&config.out_dir.join("events.ex"), "Changed");
}

#[test]
fn elixir_metadata_only_allows_sdk_reserved_abi_names() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = generated_config(
        dir.path().join("out"),
        dir.path().join("gen"),
        Target::Elixir,
    );
    config.wrappers = false;
    let folder = config.artifacts_dir.join("Helpers.sol");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("Helpers.json"), r#"{"abi":[{"type":"function","name":"__default_address__","inputs":[],"outputs":[],"stateMutability":"view"}]}"#)
        .unwrap();
    run_generate(&config, false).unwrap();
    let file = config.out_dir.join("helpers.ex");
    assert_file_contains(&file, "__default_address__");
    assert_file_contains(&file, "def abi_json");
    assert!(
        !std::fs::read_to_string(file)
            .unwrap()
            .contains("use Ethers.Contract")
    );
}
