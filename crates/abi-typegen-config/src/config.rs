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
    /// The `package` value is not a valid package name for a selected target.
    #[error("invalid package '{package}' for target {target}: {reason}")]
    InvalidPackage {
        /// The rejected package value.
        package: String,
        /// Target whose package rules the value breaks.
        target: &'static str,
        /// Why the value is invalid.
        reason: &'static str,
    },
}

/// Package name used by the Go, Kotlin, Java, and PHP targets when none is configured.
pub const DEFAULT_PACKAGE: &str = "contracts";

const GO_KEYWORDS: &[&str] = &[
    "break",
    "case",
    "chan",
    "const",
    "continue",
    "default",
    "defer",
    "else",
    "fallthrough",
    "for",
    "func",
    "go",
    "goto",
    "if",
    "import",
    "interface",
    "map",
    "package",
    "range",
    "return",
    "select",
    "struct",
    "switch",
    "type",
    "var",
];

const KOTLIN_HARD_KEYWORDS: &[&str] = &[
    "as",
    "break",
    "class",
    "continue",
    "do",
    "else",
    "false",
    "for",
    "fun",
    "if",
    "in",
    "interface",
    "is",
    "null",
    "object",
    "package",
    "return",
    "super",
    "this",
    "throw",
    "true",
    "try",
    "typealias",
    "typeof",
    "val",
    "var",
    "when",
    "while",
];

const JAVA_KEYWORDS: &[&str] = &[
    "abstract",
    "assert",
    "boolean",
    "break",
    "byte",
    "case",
    "catch",
    "char",
    "class",
    "const",
    "continue",
    "default",
    "do",
    "double",
    "else",
    "enum",
    "extends",
    "final",
    "finally",
    "float",
    "for",
    "goto",
    "if",
    "implements",
    "import",
    "instanceof",
    "int",
    "interface",
    "long",
    "native",
    "new",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "short",
    "static",
    "strictfp",
    "super",
    "switch",
    "synchronized",
    "this",
    "throw",
    "throws",
    "transient",
    "try",
    "void",
    "volatile",
    "while",
    "_",
    "true",
    "false",
    "null",
];

fn is_identifier(segment: &str) -> bool {
    let mut chars = segment.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Checks `package` against the rules of each target that uses it.
///
/// Go needs a lowercase identifier that is not a keyword. Kotlin and Java need
/// dot-separated identifiers without reserved keywords. PHP needs a nonempty
/// backslash-separated namespace. Other targets ignore the package.
pub fn validate_package(package: &str, targets: &[Target]) -> Result<(), ConfigError> {
    let invalid = |target, reason| ConfigError::InvalidPackage {
        package: package.to_string(),
        target,
        reason,
    };
    for target in targets {
        match target {
            Target::Go => {
                if !is_identifier(package) {
                    return Err(invalid("go", "expected a single Go identifier"));
                }
                if package.chars().any(|c| c.is_ascii_uppercase()) {
                    return Err(invalid("go", "Go package names are lowercase"));
                }
                if GO_KEYWORDS.contains(&package) {
                    return Err(invalid("go", "package name is a Go keyword"));
                }
            }
            Target::Kotlin => {
                if !package.split('.').all(is_identifier) {
                    return Err(invalid("kotlin", "expected dot-separated identifiers"));
                }
                if package
                    .split('.')
                    .any(|segment| KOTLIN_HARD_KEYWORDS.contains(&segment))
                {
                    return Err(invalid("kotlin", "a package segment is a Kotlin keyword"));
                }
            }
            Target::Java => {
                if !package.split('.').all(is_identifier) {
                    return Err(invalid("java", "expected dot-separated identifiers"));
                }
                if package
                    .split('.')
                    .any(|segment| JAVA_KEYWORDS.contains(&segment))
                {
                    return Err(invalid("java", "a package segment is a Java keyword"));
                }
            }
            Target::Php => {
                if package.is_empty() || !package.split('\\').all(is_identifier) {
                    return Err(invalid(
                        "php",
                        "expected backslash-separated namespace identifiers",
                    ));
                }
            }
            Target::Viem
            | Target::Zod
            | Target::Wagmi
            | Target::Ethers
            | Target::Ethers5
            | Target::Web3js
            | Target::Python
            | Target::Rust
            | Target::Swift
            | Target::CSharp
            | Target::Solidity
            | Target::C
            | Target::Cpp
            | Target::Dart
            | Target::Yaml => {}
        }
    }
    Ok(())
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
        "java" => Some(Target::Java),
        "dart" => Some(Target::Dart),
        "php" => Some(Target::Php),
        "c" => Some(Target::C),
        "cpp" | "c++" => Some(Target::Cpp),
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
    /// Generate PHP classes, ABI codecs, and JSON-RPC wrappers.
    Php,
    /// Generate Dart bindings using web3dart.
    Dart,
    /// Generate Java bindings using web3j.
    Java,
    /// Generate C11 bindings with the shared ABI runtime.
    C,
    /// Generate C++17 bindings with the shared ABI runtime.
    Cpp,
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

    /// Returns whether this target should emit a TypeScript barrel (`index.ts`).
    ///
    /// The barrel re-exports the generated TypeScript modules using ES module
    /// syntax, so it is only meaningful for TypeScript-family targets. Targets
    /// that emit other languages (e.g. Python `.py`, Go `.go`, Solidity `.sol`)
    /// must not produce a stray, content-free `index.ts`.
    pub fn emits_barrel(&self) -> bool {
        match self {
            Self::Viem | Self::Zod | Self::Wagmi | Self::Ethers | Self::Ethers5 | Self::Web3js => {
                true
            }
            Self::Python
            | Self::Go
            | Self::Rust
            | Self::Swift
            | Self::CSharp
            | Self::Kotlin
            | Self::Solidity
            | Self::Java
            | Self::C
            | Self::Cpp
            | Self::Dart
            | Self::Php
            | Self::Yaml => false,
        }
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
            | Self::Java
            | Self::C
            | Self::Cpp
            | Self::Dart
            | Self::Php
            | Self::Yaml => None,
        }
    }
}

impl<'de> Deserialize<'de> for Target {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        parse_target(&s).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "unknown target '{}', expected viem|zod|wagmi|ethers|ethers5|web3js|python|go|rust|swift|csharp|kotlin|java|dart|php|solidity|c|cpp|yaml",
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
    #[serde(default = "default_package")]
    package: String,
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
                        "unknown target '{}', expected viem|zod|wagmi|ethers|ethers5|web3js|python|go|rust|swift|csharp|kotlin|java|dart|php|solidity|c|cpp|yaml",
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
                "unknown target '{}', expected viem|zod|wagmi|ethers|ethers5|web3js|python|go|rust|swift|csharp|kotlin|java|dart|php|solidity|c|cpp|yaml",
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

fn default_package() -> String {
    DEFAULT_PACKAGE.to_string()
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
    /// Package or namespace for Go, Kotlin, Java, and PHP output. See [`validate_package`].
    pub package: String,
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
            package: default_package(),
        });
        validate_package(&section.package, &section.targets)?;
        Ok(Config {
            artifacts_dir,
            out_dir: section.out,
            targets: section.targets,
            wrappers: section.wrappers,
            contracts: section.contracts,
            exclude: section.exclude,
            package: section.package,
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
    fn new_native_targets_parse_and_do_not_emit_typescript_modules() {
        for (name, target) in [
            ("java", Target::Java),
            ("dart", Target::Dart),
            ("php", Target::Php),
            ("c", Target::C),
            ("cpp", Target::Cpp),
            ("c++", Target::Cpp),
        ] {
            let config =
                Config::from_toml_str(&format!("[abi-typegen]\ntarget = \"{name}\"\n")).unwrap();
            assert_eq!(config.target(), &target);
            assert!(!target.emits_typescript_abi());
            assert!(!target.emits_barrel());
        }
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

    #[test]
    fn package_defaults_to_contracts() {
        let cfg = Config::from_toml_str("").unwrap();
        assert_eq!(cfg.package, "contracts");
    }

    #[test]
    fn package_reads_from_toml() {
        let cfg = Config::from_toml_str(
            "[abi-typegen]\ntarget = \"kotlin\"\npackage = \"com.example.contracts\"\n",
        )
        .unwrap();
        assert_eq!(cfg.package, "com.example.contracts");
    }

    #[test]
    fn go_package_rejects_dots_uppercase_and_keywords() {
        for bad in ["com.example", "Contracts", "type", "1abc", ""] {
            assert!(
                validate_package(bad, &[Target::Go]).is_err(),
                "{bad} should be rejected for go"
            );
        }
        assert!(validate_package("bindings", &[Target::Go]).is_ok());
    }

    #[test]
    fn kotlin_package_rejects_bad_segments() {
        for bad in ["com..example", "com.class", "com.1x", ".com", ""] {
            assert!(
                validate_package(bad, &[Target::Kotlin]).is_err(),
                "{bad} should be rejected for kotlin"
            );
        }
        assert!(validate_package("com.example.contracts", &[Target::Kotlin]).is_ok());
    }

    #[test]
    fn php_namespace_rejects_invalid_segments() {
        for bad in [
            "",
            "App.Contracts",
            "App\\\\Contracts",
            "App\\1Token",
            "App; echo 1",
            " App",
        ] {
            assert!(validate_package(bad, &[Target::Php]).is_err(), "{bad}");
        }
        for valid in ["contracts", "App\\Contracts"] {
            assert!(validate_package(valid, &[Target::Php]).is_ok(), "{valid}");
        }
    }

    #[test]
    fn java_package_uses_java_keywords() {
        for bad in ["com.int", "com._", "com.null", "com..example", "com.1x", ""] {
            let error = validate_package(bad, &[Target::Java]).unwrap_err();
            assert!(error.to_string().contains("target java"));
        }
        assert!(validate_package("com.object.contracts", &[Target::Java]).is_ok());
        assert!(validate_package("com.object.contracts", &[Target::Kotlin]).is_err());
    }

    #[test]
    fn package_is_checked_against_every_selected_target() {
        let err = Config::from_toml_str(
            "[abi-typegen]\ntarget = [\"kotlin\", \"go\"]\npackage = \"com.example\"\n",
        )
        .unwrap_err();
        assert!(err.to_string().contains("target go"), "{err}");
    }

    #[test]
    fn package_is_ignored_by_other_targets() {
        assert!(validate_package("Not A Package", &[Target::Viem, Target::Rust]).is_ok());
    }
}
