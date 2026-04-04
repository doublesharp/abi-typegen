use serde::Deserialize;
use std::path::PathBuf;

/// Error returned by configuration parsing functions.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The configuration file could not be read from disk.
    #[error("cannot read {path}: {source}")]
    Io {
        /// Path that could not be read.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// The TOML content was invalid or contained an unknown configuration value.
    #[error("failed to parse foundry.toml: {0}")]
    TomlParse(#[from] toml::de::Error),
}

/// Parses a target name string into a [`Target`] enum variant.
///
/// Returns `None` if the string does not match any known target.
pub fn parse_target(s: &str) -> Option<Target> {
    match s {
        "viem" => Some(Target::Viem),
        "zod" => Some(Target::Zod),
        "wagmi" => Some(Target::Wagmi),
        "ethers" | "ethers6" => Some(Target::Ethers),
        "ethers5" => Some(Target::Ethers5),
        "web3js" | "web3" => Some(Target::Web3js),
        "python" => Some(Target::Python),
        "go" => Some(Target::Go),
        "rust" => Some(Target::Rust),
        "swift" => Some(Target::Swift),
        "csharp" | "cs" => Some(Target::CSharp),
        "kotlin" | "kt" => Some(Target::Kotlin),
        "solidity" | "sol" => Some(Target::Solidity),
        "yaml" | "yml" => Some(Target::Yaml),
        _ => None,
    }
}

/// Code generation target.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Target {
    /// Generate viem typed helpers (TypeScript).
    #[default]
    Viem,
    /// Generate Zod validation schemas (TypeScript).
    Zod,
    /// Generate wagmi React hooks (TypeScript).
    Wagmi,
    /// Generate ethers v6 typed interfaces (TypeScript).
    Ethers,
    /// Generate ethers v5 typed interfaces (TypeScript).
    Ethers5,
    /// Generate web3.js typed interfaces (TypeScript).
    Web3js,
    /// Generate Python type stubs (web3.py compatible).
    Python,
    /// Generate Go bindings (go-ethereum compatible).
    Go,
    /// Generate Rust types (alloy compatible).
    Rust,
    /// Generate Swift types (web3swift compatible).
    Swift,
    /// Generate C# types (Nethereum compatible).
    CSharp,
    /// Generate Kotlin types (web3j compatible).
    Kotlin,
    /// Generate Solidity interfaces.
    Solidity,
    /// Generate YAML ABI descriptions.
    Yaml,
}

impl Target {
    /// Returns whether this target emits a TypeScript ABI module.
    pub fn emits_typescript_abi(&self) -> bool {
        matches!(
            self,
            Self::Viem | Self::Zod | Self::Wagmi | Self::Ethers | Self::Ethers5 | Self::Web3js
        )
    }

    /// Returns the generated wrapper module suffix for targets that emit wrappers.
    pub fn wrapper_module_suffix(&self) -> Option<&'static str> {
        match self {
            Self::Viem => Some("viem"),
            Self::Wagmi => Some("wagmi"),
            Self::Ethers => Some("ethers"),
            Self::Ethers5 => Some("ethers5"),
            Self::Web3js => Some("web3"),
            Self::Zod
            | Self::Python
            | Self::Go
            | Self::Rust
            | Self::Swift
            | Self::CSharp
            | Self::Kotlin
            | Self::Solidity
            | Self::Yaml => None,
        }
    }
}

impl<'de> Deserialize<'de> for Target {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        parse_target(&s).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "unknown target '{}', expected viem|zod|wagmi|ethers|ethers5|web3js|python|go|rust|swift|csharp|kotlin|solidity|yaml",
                s
            ))
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct AbiTypegenSection {
    #[serde(default = "default_out_dir")]
    out: PathBuf,
    #[serde(
        default = "default_targets",
        deserialize_with = "deserialize_targets",
        rename = "target"
    )]
    targets: Vec<Target>,
    #[serde(default = "default_true")]
    wrappers: bool,
    #[serde(default)]
    contracts: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
}

/// Serde field alias so that the TOML key `target` maps to the `targets` field.
///
/// Accepts:
/// - a single string: `"viem"` -> `vec![Target::Viem]`
/// - a comma-separated string: `"viem,python"` -> `vec![Target::Viem, Target::Python]`
/// - a TOML array of strings: `["viem", "python"]` -> `vec![Target::Viem, Target::Python]`
fn deserialize_targets<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Vec<Target>, D::Error> {
    use serde::de;

    struct TargetsVisitor;

    impl<'de> de::Visitor<'de> for TargetsVisitor {
        type Value = Vec<Target>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str(
                "a target string, comma-separated target string, or array of target strings",
            )
        }

        fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Vec<Target>, E> {
            parse_targets_from_str(v).map_err(de::Error::custom)
        }

        fn visit_seq<A: de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> std::result::Result<Vec<Target>, A::Error> {
            let mut targets = Vec::new();
            while let Some(s) = seq.next_element::<String>()? {
                let t = parse_target(s.trim()).ok_or_else(|| {
                    de::Error::custom(format!(
                        "unknown target '{}', expected viem|zod|wagmi|ethers|ethers5|web3js|python|go|rust|swift|csharp|kotlin|solidity|yaml",
                        s
                    ))
                })?;
                targets.push(t);
            }
            if targets.is_empty() {
                return Err(de::Error::custom("target array must not be empty"));
            }
            Ok(targets)
        }
    }

    d.deserialize_any(TargetsVisitor)
}

/// Parses a possibly comma-separated target string into a list of targets.
fn parse_targets_from_str(s: &str) -> Result<Vec<Target>, String> {
    let parts: Vec<&str> = s.split(',').map(|p| p.trim()).collect();
    let mut targets = Vec::with_capacity(parts.len());
    for part in parts {
        let t = parse_target(part).ok_or_else(|| {
            format!(
                "unknown target '{}', expected viem|zod|wagmi|ethers|ethers5|web3js|python|go|rust|swift|csharp|kotlin|solidity|yaml",
                part
            )
        })?;
        targets.push(t);
    }
    Ok(targets)
}

fn default_targets() -> Vec<Target> {
    vec![Target::default()]
}

fn default_out_dir() -> PathBuf {
    PathBuf::from("src/generated")
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
struct FoundryProfileDefault {
    #[serde(default = "default_artifacts_dir")]
    out: PathBuf,
}

// Manual Default impl required because serde's `default` attribute on fields
// doesn't compose with #[derive(Default)] — we need the same default value.
impl Default for FoundryProfileDefault {
    #[inline]
    fn default() -> Self {
        Self {
            out: default_artifacts_dir(),
        }
    }
}

fn default_artifacts_dir() -> PathBuf {
    PathBuf::from("out")
}

#[derive(Debug, Clone, Deserialize, Default)]
struct FoundryProfile {
    #[serde(default)]
    default: FoundryProfileDefault,
}

#[derive(Debug, Clone, Deserialize)]
struct FoundryToml {
    #[serde(default)]
    profile: FoundryProfile,
    #[serde(rename = "abi-typegen")]
    abi_typegen: Option<AbiTypegenSection>,
}

/// Resolved, fully-typed configuration for abi-typegen.
#[derive(Debug, Clone)]
pub struct Config {
    /// Path to compiled artifact directory (Foundry `out/` or Hardhat `artifacts/contracts/`).
    pub artifacts_dir: PathBuf,
    /// Where to write generated files.
    pub out_dir: PathBuf,
    /// Which generation targets to use (one or more).
    pub targets: Vec<Target>,
    /// Whether to emit typed wrapper functions.
    pub wrappers: bool,
    /// Specific contracts to generate (empty = all).
    pub contracts: Vec<String>,
    /// Exclude contracts matching these glob patterns.
    pub exclude: Vec<String>,
}

impl Config {
    /// Returns the first (primary) target.
    ///
    /// This is a convenience accessor for code paths that operate on a single
    /// target at a time (e.g. codegen, which is invoked once per target).
    pub fn target(&self) -> &Target {
        self.targets
            .first()
            .expect("Config must have at least one target")
    }

    /// Parses configuration from a TOML string (the content of `foundry.toml`).
    pub fn from_toml_str(toml_str: &str) -> Result<Self, ConfigError> {
        let raw: FoundryToml = toml::from_str(toml_str)?;
        let artifacts_dir = raw.profile.default.out;
        let section = raw.abi_typegen.unwrap_or(AbiTypegenSection {
            out: default_out_dir(),
            targets: default_targets(),
            wrappers: true,
            contracts: vec![],
            exclude: vec![],
        });
        Ok(Config {
            artifacts_dir,
            out_dir: section.out,
            targets: section.targets,
            wrappers: section.wrappers,
            contracts: section.contracts,
            exclude: section.exclude,
        })
    }

    /// Reads and parses configuration from a `foundry.toml` file on disk.
    pub fn from_file(path: &std::path::Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path).map_err(|source| ConfigError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_toml_str(&content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_no_section() {
        let toml = r#"
[profile.default]
src = "src"
out = "out"
"#;
        let cfg = Config::from_toml_str(toml).unwrap();
        assert_eq!(cfg.artifacts_dir, PathBuf::from("out"));
        assert_eq!(cfg.out_dir, PathBuf::from("src/generated"));
        assert_eq!(*cfg.target(), Target::Viem);
        assert!(cfg.wrappers);
        assert!(cfg.contracts.is_empty());
    }

    #[test]
    fn reads_abi_typegen_section() {
        let toml = r#"
[profile.default]
out = "artifacts"

[abi-typegen]
out = "app/types"
target = "viem"
wrappers = false
contracts = ["MyToken", "Vault"]
"#;
        let cfg = Config::from_toml_str(toml).unwrap();
        assert_eq!(cfg.artifacts_dir, PathBuf::from("artifacts"));
        assert_eq!(cfg.out_dir, PathBuf::from("app/types"));
        assert_eq!(*cfg.target(), Target::Viem);
        assert!(!cfg.wrappers);
        assert_eq!(cfg.contracts, vec!["MyToken", "Vault"]);
    }

    #[test]
    fn target_capabilities_match_expected_outputs() {
        assert!(Target::Viem.emits_typescript_abi());
        assert!(Target::Zod.emits_typescript_abi());
        assert!(!Target::Python.emits_typescript_abi());
        assert!(!Target::Solidity.emits_typescript_abi());

        assert_eq!(Target::Viem.wrapper_module_suffix(), Some("viem"));
        assert_eq!(Target::Web3js.wrapper_module_suffix(), Some("web3"));
        assert_eq!(Target::Zod.wrapper_module_suffix(), None);
        assert_eq!(Target::Rust.wrapper_module_suffix(), None);
        assert_eq!(Target::Solidity.wrapper_module_suffix(), None);
    }

    #[test]
    fn invalid_target_errors() {
        let toml = r#"
[abi-typegen]
target = "truffle"
"#;
        let err = Config::from_toml_str(toml).unwrap_err();
        let chain = format!("{:?}", err);
        assert!(
            chain.contains("truffle"),
            "error should mention 'truffle': {chain}"
        );
        assert!(chain.contains("zod"), "error should list zod: {chain}");
        assert!(
            chain.contains("solidity"),
            "error should list solidity: {chain}"
        );
    }

    #[test]
    fn target_zod_roundtrip() {
        let toml = "[abi-typegen]\ntarget = \"zod\"\n";
        let cfg = Config::from_toml_str(toml).unwrap();
        assert_eq!(*cfg.target(), Target::Zod);
    }

    #[test]
    fn target_solidity_roundtrip() {
        let toml = "[abi-typegen]\ntarget = \"solidity\"\n";
        let cfg = Config::from_toml_str(toml).unwrap();
        assert_eq!(*cfg.target(), Target::Solidity);
    }

    #[test]
    fn aggregate_target_aliases_are_rejected() {
        for target in ["all", "all-ts"] {
            let toml = format!("[abi-typegen]\ntarget = \"{}\"\n", target);
            assert!(Config::from_toml_str(&toml).is_err());
        }
    }

    #[test]
    fn empty_toml_uses_all_defaults() {
        let cfg = Config::from_toml_str("").unwrap();
        assert_eq!(cfg.artifacts_dir, PathBuf::from("out"));
        assert_eq!(cfg.out_dir, PathBuf::from("src/generated"));
        assert_eq!(*cfg.target(), Target::Viem);
    }

    #[test]
    fn partial_section_fills_defaults() {
        let toml = r#"
[abi-typegen]
target = "ethers"
"#;
        let cfg = Config::from_toml_str(toml).unwrap();
        assert_eq!(*cfg.target(), Target::Ethers);
        assert_eq!(cfg.out_dir, PathBuf::from("src/generated"));
        assert!(cfg.wrappers);
        assert!(cfg.contracts.is_empty());
    }

    #[test]
    fn wrappers_defaults_to_true() {
        let toml = r#"
[abi-typegen]
out = "types"
"#;
        let cfg = Config::from_toml_str(toml).unwrap();
        assert!(cfg.wrappers);
    }

    #[test]
    fn from_file_reads_tempfile() {
        let dir = std::env::temp_dir().join("abi-typegen-test-config");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("foundry.toml");
        std::fs::write(
            &path,
            r#"
[profile.default]
out = "build"

[abi-typegen]
target = "viem"
"#,
        )
        .unwrap();
        let cfg = Config::from_file(&path).unwrap();
        assert_eq!(cfg.artifacts_dir, PathBuf::from("build"));
        assert_eq!(*cfg.target(), Target::Viem);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn from_file_error_on_missing() {
        let result = Config::from_file(std::path::Path::new("/nonexistent/foundry.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn malformed_toml_errors() {
        let result = Config::from_toml_str("[invalid toml{{{");
        assert!(result.is_err());
    }

    #[test]
    fn target_ethers_roundtrip() {
        let toml = "[abi-typegen]\ntarget = \"ethers\"\n";
        let cfg = Config::from_toml_str(toml).unwrap();
        assert_eq!(*cfg.target(), Target::Ethers);
    }

    #[test]
    fn custom_artifacts_dir_from_profile() {
        let toml = r#"
[profile.default]
out = "custom-out"
"#;
        let cfg = Config::from_toml_str(toml).unwrap();
        assert_eq!(cfg.artifacts_dir, PathBuf::from("custom-out"));
    }

    #[test]
    fn target_comma_separated_string() {
        let toml = r#"
[abi-typegen]
target = "viem,python"
"#;
        let cfg = Config::from_toml_str(toml).unwrap();
        assert_eq!(cfg.targets, vec![Target::Viem, Target::Python]);
        assert_eq!(*cfg.target(), Target::Viem);
    }

    #[test]
    fn target_comma_separated_with_spaces() {
        let toml = r#"
[abi-typegen]
target = "viem, python, go"
"#;
        let cfg = Config::from_toml_str(toml).unwrap();
        assert_eq!(cfg.targets, vec![Target::Viem, Target::Python, Target::Go]);
    }

    #[test]
    fn target_array_syntax() {
        let toml = r#"
[abi-typegen]
target = ["viem", "python"]
"#;
        let cfg = Config::from_toml_str(toml).unwrap();
        assert_eq!(cfg.targets, vec![Target::Viem, Target::Python]);
    }

    #[test]
    fn target_array_single_element() {
        let toml = r#"
[abi-typegen]
target = ["zod"]
"#;
        let cfg = Config::from_toml_str(toml).unwrap();
        assert_eq!(cfg.targets, vec![Target::Zod]);
        assert_eq!(*cfg.target(), Target::Zod);
    }

    #[test]
    fn target_array_empty_is_rejected() {
        let toml = r#"
[abi-typegen]
target = []
"#;
        let result = Config::from_toml_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn target_array_unknown_element_errors() {
        let toml = r#"
[abi-typegen]
target = ["viem", "invalid"]
"#;
        let result = Config::from_toml_str(toml);
        assert!(result.is_err());
        let msg = format!("{:?}", result.unwrap_err());
        assert!(
            msg.contains("invalid"),
            "error should mention 'invalid': {msg}"
        );
    }

    #[test]
    fn target_comma_separated_unknown_errors() {
        let toml = r#"
[abi-typegen]
target = "viem,badtarget"
"#;
        let result = Config::from_toml_str(toml);
        assert!(result.is_err());
        let msg = format!("{:?}", result.unwrap_err());
        assert!(
            msg.contains("badtarget"),
            "error should mention 'badtarget': {msg}"
        );
    }

    #[test]
    fn parse_target_known_names() {
        assert_eq!(parse_target("viem"), Some(Target::Viem));
        assert_eq!(parse_target("python"), Some(Target::Python));
        assert_eq!(parse_target("sol"), Some(Target::Solidity));
        assert_eq!(parse_target("cs"), Some(Target::CSharp));
        assert_eq!(parse_target("kt"), Some(Target::Kotlin));
        assert_eq!(parse_target("unknown"), None);
    }
}
