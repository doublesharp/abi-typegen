#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

mod fetch;

use abi_typegen_codegen::{barrel, generate_contract_files};
use abi_typegen_config::{parse_target, Config, Target};
use abi_typegen_core::parser::parse_artifact;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "abi-typegen",
    about = "Generate bindings from Solidity ABI artifacts (Foundry, Hardhat)",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// Path to foundry.toml (default: auto-detect)
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    /// Use Hardhat artifact layout (artifacts/contracts/)
    #[arg(long, global = true)]
    hardhat: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate bindings from compiled artifacts
    Generate {
        /// Path to compiled artifacts (Foundry out/ or Hardhat artifacts/contracts/)
        #[arg(long)]
        artifacts: Option<PathBuf>,
        /// Output directory for generated files
        #[arg(long)]
        out: Option<PathBuf>,
        /// Target name
        #[arg(long)]
        target: Option<String>,
        /// Disable wrapper generation
        #[arg(long)]
        no_wrappers: bool,
        /// Limit generation to these contract names (repeat or comma-separate)
        #[arg(long, value_delimiter = ',')]
        contracts: Vec<String>,
        /// Exclude contracts matching these patterns (comma-separated globs: "*Test,*Mock,I*")
        #[arg(long)]
        exclude: Option<String>,
        /// Check if output is up to date (exit non-zero if stale)
        #[arg(long)]
        check: bool,
        /// Remove stale generated files not produced by this run
        #[arg(long)]
        clean: bool,
    },
    /// Watch artifact directory for changes and regenerate automatically
    Watch {
        /// Path to compiled artifacts to watch
        #[arg(long)]
        artifacts: Option<PathBuf>,
    },
    /// Show what would change without writing (dry run)
    Diff {
        /// Path to compiled artifacts
        #[arg(long)]
        artifacts: Option<PathBuf>,
        /// Output directory to compare against
        #[arg(long)]
        out: Option<PathBuf>,
        /// Target name
        #[arg(long)]
        target: Option<String>,
        /// Disable wrapper generation
        #[arg(long)]
        no_wrappers: bool,
        /// Limit generation to these contract names (repeat or comma-separate)
        #[arg(long, value_delimiter = ',')]
        contracts: Vec<String>,
        /// Exclude contracts matching these patterns (comma-separated globs: "*Test,*Mock,I*")
        #[arg(long)]
        exclude: Option<String>,
    },
    /// Dump parsed ABI as JSON (for debugging or piping to other tools)
    Json {
        /// Path to compiled artifacts
        #[arg(long)]
        artifacts: Option<PathBuf>,
        /// Limit generation to these contract names (repeat or comma-separate)
        #[arg(long, value_delimiter = ',')]
        contracts: Vec<String>,
        /// Exclude contracts matching these patterns (comma-separated globs: "*Test,*Mock,I*")
        #[arg(long)]
        exclude: Option<String>,
        /// Pretty-print the output
        #[arg(long)]
        pretty: bool,
    },
    /// Add [abi-typegen] scaffold to foundry.toml
    Init,
    /// Install Forge shell integration (enables `forge typegen` subcommand)
    #[command(name = "forge-install")]
    ForgeInstall {
        /// Shell type: bash, zsh, or fish
        #[arg(long, default_value = "bash")]
        shell: String,
    },
    /// Fetch a contract ABI from a block explorer (or a local file) and generate
    /// typed bindings. The ABI is saved as an artifact so subsequent `generate`
    /// or `watch` runs include it.
    Fetch {
        /// Contract address to fetch from a block explorer (0x...).
        /// Required unless --file is provided; mutually exclusive with --file.
        #[arg(conflicts_with = "file")]
        address: Option<String>,
        /// Contract name — used for the artifact path and generated filenames
        #[arg(long)]
        name: String,
        /// Load ABI from a local JSON file instead of fetching from a block
        /// explorer. Accepts a raw ABI array `[...]` or a Foundry/Hardhat
        /// artifact `{"abi": [...]}`. Mutually exclusive with ADDRESS.
        #[arg(long, conflicts_with = "address")]
        file: Option<PathBuf>,
        /// Full Etherscan-compatible API URL (e.g. `https://api.sonicscan.org/api`).
        /// Takes priority over --network.
        #[arg(long)]
        url: Option<String>,
        /// Named network shortcut — see docs for the full list.
        /// All Etherscan-operated chains (mainnet, base, arbitrum, polygon, …)
        /// use the Etherscan V2 unified endpoint; independent chains
        /// (zksync, fantom, cronos, …) use their own explorer APIs.
        #[arg(long, default_value = "mainnet")]
        network: String,
        /// Block explorer API key (or set ETHERSCAN_API_KEY env var)
        #[arg(long, env = "ETHERSCAN_API_KEY")]
        api_key: Option<String>,
        /// Artifacts directory to write into (default: from foundry.toml or "out/")
        #[arg(long)]
        artifacts: Option<PathBuf>,
        /// Overwrite an existing artifact without error
        #[arg(long)]
        force: bool,
    },
}

// Entry point — delegates to run() which is fully tested.
// Excluded from coverage: Cli::parse() reads std::env::args() which can't be
// set from within the test harness.
#[cfg_attr(coverage_nightly, coverage(off))]
fn main() -> Result<()> {
    // Load .env before clap parses so env vars are visible to #[arg(env = "...")]
    dotenvy::dotenv().ok();
    let cli = Cli::parse();
    run(cli)
}

fn run(cli: Cli) -> Result<()> {
    let config_path = resolve_config_path(&cli.config, cli.hardhat);

    match cli.command {
        Commands::Generate {
            artifacts,
            out,
            target,
            no_wrappers,
            contracts,
            exclude,
            check,
            clean,
        } => {
            let mut config = load_config(&config_path, cli.hardhat)?;
            apply_overrides(&mut config, artifacts, out, target, no_wrappers)?;
            apply_contracts(&mut config, &contracts);
            apply_exclude(&mut config, &exclude);

            if config.targets.len() > 1 {
                // Multi-target: generate into per-target subdirectories
                let base_out_dir = config.out_dir.clone();
                let mut active_target_dirs = HashSet::new();
                for target in config.targets.clone() {
                    let mut target_config = config.clone();
                    target_config.targets = vec![target.clone()];
                    active_target_dirs.insert(target_dir_name(&target).to_string());
                    target_config.out_dir = multi_target_out_dir(&base_out_dir, &target);
                    if check {
                        run_check(&target_config)?;
                    } else {
                        run_generate(&target_config, clean)?;
                    }
                }

                if check {
                    ensure_no_stale_target_dirs(&base_out_dir, &active_target_dirs)?;
                } else if clean {
                    clean_stale_target_dirs(&base_out_dir, &active_target_dirs)?;
                }
            } else {
                // Single target (existing behavior)
                if check {
                    run_check(&config)?;
                } else {
                    run_generate(&config, clean)?;
                }
            }
        }
        Commands::Watch { artifacts } => {
            let mut config = load_config(&config_path, cli.hardhat)?;
            if let Some(a) = artifacts {
                config.artifacts_dir = a;
            }
            run_watch(config)?;
        }
        Commands::Diff {
            artifacts,
            out,
            target,
            no_wrappers,
            contracts,
            exclude,
        } => {
            let mut config = load_config(&config_path, cli.hardhat)?;
            apply_overrides(&mut config, artifacts, out, target, no_wrappers)?;
            apply_contracts(&mut config, &contracts);
            apply_exclude(&mut config, &exclude);
            run_diff(&config)?;
        }
        Commands::Json {
            artifacts,
            contracts,
            exclude,
            pretty,
        } => {
            let mut config = load_config(&config_path, cli.hardhat)?;
            if let Some(a) = artifacts {
                config.artifacts_dir = a;
            }
            apply_contracts(&mut config, &contracts);
            apply_exclude(&mut config, &exclude);
            run_json(&config, pretty)?;
        }
        Commands::Init => {
            run_init(&config_path)?;
        }
        Commands::ForgeInstall { shell } => {
            run_shell(&shell);
        }
        Commands::Fetch {
            address,
            name,
            url,
            network,
            api_key,
            file,
            artifacts,
            force,
        } => {
            let mut config = load_config(&config_path, cli.hardhat)?;
            if let Some(a) = artifacts {
                config.artifacts_dir = a;
            }
            let source = FetchSource::resolve(
                address.as_deref(),
                file.as_deref(),
                &network,
                url.as_deref(),
                api_key,
            )?;
            run_fetch(source, &name, &config.artifacts_dir, force)?;
            // Generate typed bindings for the fetched contract immediately.
            config.contracts = vec![name];
            run_generate(&config, false)?;
        }
    }

    Ok(())
}

/// Resolved ABI source for a `fetch` invocation.
enum FetchSource<'a> {
    Network {
        address: &'a str,
        api_url: &'a str,
        api_key: Option<String>,
    },
    File(&'a Path),
}

impl<'a> FetchSource<'a> {
    fn resolve(
        address: Option<&'a str>,
        file: Option<&'a Path>,
        network: &'a str,
        explicit_url: Option<&'a str>,
        api_key: Option<String>,
    ) -> Result<Self> {
        if let Some(path) = file {
            return Ok(FetchSource::File(path));
        }
        let address = address.context("an ADDRESS or --file is required")?;
        let api_url = fetch::resolve_api_url(network, explicit_url)?;
        let effective_key = api_key.or_else(|| std::env::var("ETHERSCAN_API_KEY").ok());
        Ok(FetchSource::Network {
            address,
            api_url,
            api_key: effective_key,
        })
    }
}

fn run_fetch(source: FetchSource<'_>, name: &str, artifacts_dir: &Path, force: bool) -> Result<()> {
    let dest = fetch::artifact_path(artifacts_dir, name);
    if dest.exists() && !force {
        anyhow::bail!(
            "artifact already exists: {} — use --force to overwrite",
            dest.display()
        );
    }

    let abi = match source {
        FetchSource::File(path) => fetch::load_abi_from_file(path)?,
        FetchSource::Network {
            address,
            api_url,
            api_key,
        } => fetch::fetch_abi(api_url, address, api_key.as_deref())?,
    };

    let json = fetch::build_artifact_json(abi)?;

    let dest_parent = dest
        .parent()
        .with_context(|| format!("artifact path has no parent: {}", dest.display()))?;
    std::fs::create_dir_all(dest_parent)
        .with_context(|| format!("cannot create directory for '{}'", dest.display()))?;
    std::fs::write(&dest, &json).with_context(|| format!("cannot write '{}'", dest.display()))?;

    println!("abi-typegen: fetched {} → {}", name, dest.display());
    Ok(())
}

fn multi_target_out_dir(base_out_dir: &Path, target: &Target) -> PathBuf {
    base_out_dir.join(target_dir_name(target))
}

fn target_dir_name(target: &Target) -> &'static str {
    match target {
        Target::Viem => "viem",
        Target::Zod => "zod",
        Target::Wagmi => "wagmi",
        Target::Ethers => "ethers",
        Target::Ethers5 => "ethers5",
        Target::Web3js => "web3js",
        Target::Python => "python",
        Target::Go => "go",
        Target::Rust => "rust",
        Target::Swift => "swift",
        Target::CSharp => "csharp",
        Target::Kotlin => "kotlin",
        Target::Solidity => "solidity",
        Target::Yaml => "yaml",
    }
}

const GENERATED_TARGET_DIRS: &[&str] = &[
    "viem", "zod", "wagmi", "ethers", "ethers5", "web3js", "python", "go", "rust", "swift",
    "csharp", "kotlin", "solidity", "yaml",
];

/// Resolves the config file path. If `--config` is given, uses that.
/// Otherwise auto-detects: `--hardhat` checks for `hardhat.config.ts`/`.js`,
/// default checks for `foundry.toml`.
fn resolve_config_path(explicit: &Option<PathBuf>, hardhat: bool) -> PathBuf {
    if let Some(path) = explicit {
        return path.clone();
    }
    if hardhat {
        // Hardhat doesn't store abi-typegen config — return a dummy path
        // so load_config falls through to defaults
        PathBuf::from("hardhat.config.ts")
    } else {
        PathBuf::from("foundry.toml")
    }
}

fn load_config(path: &Path, hardhat: bool) -> Result<Config> {
    if hardhat {
        // Hardhat default: artifacts/ instead of out/
        let mut config = Config::from_toml_str("").expect("empty config always parses");
        config.artifacts_dir = PathBuf::from("artifacts/contracts");
        return Ok(config);
    }
    if path.exists() {
        Ok(Config::from_file(path)?)
    } else {
        Ok(Config::from_toml_str("").expect("empty config always parses"))
    }
}

fn apply_overrides(
    config: &mut Config,
    artifacts: Option<PathBuf>,
    out: Option<PathBuf>,
    target: Option<String>,
    no_wrappers: bool,
) -> Result<()> {
    if let Some(a) = artifacts {
        config.artifacts_dir = a;
    }
    if let Some(o) = out {
        config.out_dir = o;
    }
    if let Some(t) = target {
        let mut targets = Vec::new();
        for part in t.split(',') {
            let trimmed = part.trim();
            let parsed = parse_target(trimmed).ok_or_else(|| {
                anyhow::anyhow!(
                    "unknown target '{}', expected viem|zod|wagmi|ethers|ethers5|web3js|python|go|rust|swift|csharp|kotlin|solidity|yaml",
                    trimmed
                )
            })?;
            targets.push(parsed);
        }
        config.targets = targets;
    }
    if no_wrappers {
        config.wrappers = false;
    }
    Ok(())
}

/// Merges CLI `--exclude` patterns into `config.exclude`.
fn apply_exclude(config: &mut Config, exclude: &Option<String>) {
    if let Some(ref patterns) = exclude {
        for p in patterns.split(',') {
            let trimmed = p.trim().to_string();
            if !trimmed.is_empty() {
                config.exclude.push(trimmed);
            }
        }
    }
}

/// Replaces configured contracts with the CLI-provided contract list.
fn apply_contracts(config: &mut Config, contracts: &[String]) {
    if contracts.is_empty() {
        return;
    }

    config.contracts = contracts
        .iter()
        .filter_map(|name| {
            let trimmed = name.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
        .collect();
}

/// Simple glob matching: `*` matches zero or more characters, `?` matches
/// exactly one character. Supports patterns like `*Test`, `I*`, `*Mock*`.
fn matches_glob(name: &str, pattern: &str) -> bool {
    let name = name.as_bytes();
    let pattern = pattern.as_bytes();
    glob_match(name, pattern, 0, 0)
}

fn glob_match(name: &[u8], pattern: &[u8], ni: usize, pi: usize) -> bool {
    if pi == pattern.len() {
        return ni == name.len();
    }
    if pattern[pi] == b'*' {
        // '*' matches zero or more characters
        for i in ni..=name.len() {
            if glob_match(name, pattern, i, pi + 1) {
                return true;
            }
        }
        false
    } else if ni < name.len() && (pattern[pi] == b'?' || pattern[pi] == name[ni]) {
        glob_match(name, pattern, ni + 1, pi + 1)
    } else {
        false
    }
}

/// Returns true if a contract name matches any of the exclude patterns.
fn is_excluded(name: &str, exclude: &[String]) -> bool {
    exclude.iter().any(|pattern| matches_glob(name, pattern))
}

/// Generated file extensions to consider when cleaning stale files.
const GENERATED_EXTENSIONS: &[&str] = &[
    ".abi.ts",
    ".zod.ts",
    ".viem.ts",
    ".ethers.ts",
    ".ethers5.ts",
    ".web3.ts",
    ".wagmi.ts",
    ".py",
    ".go",
    ".rs",
    ".swift",
    ".cs",
    ".kt",
    ".sol",
];

/// Checks if a filename looks like a generated file (matches known patterns).
fn is_generated_filename(filename: &str) -> bool {
    if filename == "index.ts" {
        return true;
    }
    GENERATED_EXTENSIONS
        .iter()
        .any(|ext| filename.ends_with(ext))
}

fn filter_excluded_artifacts(
    artifacts: Vec<(String, PathBuf)>,
    exclude: &[String],
) -> Vec<(String, PathBuf)> {
    if exclude.is_empty() {
        return artifacts;
    }

    artifacts
        .into_iter()
        .filter(|(name, _)| !is_excluded(name, exclude))
        .collect()
}

fn selected_artifacts(config: &Config) -> Result<Vec<(String, PathBuf)>> {
    let artifacts = discover_artifacts(&config.artifacts_dir, &config.contracts)?;
    Ok(filter_excluded_artifacts(artifacts, &config.exclude))
}

fn run_generate(config: &Config, clean: bool) -> Result<()> {
    if !config.artifacts_dir.exists() {
        anyhow::bail!(
            "Foundry out directory '{}' does not exist. Run `forge build` first.",
            config.artifacts_dir.display()
        );
    }

    std::fs::create_dir_all(&config.out_dir)
        .with_context(|| format!("cannot create '{}'", config.out_dir.display()))?;

    let artifacts = selected_artifacts(config)?;

    if artifacts.is_empty() {
        println!(
            "abi-typegen: no artifacts found in '{}'",
            config.artifacts_dir.display()
        );
        return Ok(());
    }

    let mut contract_names = Vec::new();
    let mut generated_files = HashSet::new();

    for (name, path) in &artifacts {
        let json = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read artifact '{}'", path.display()))?;

        let ir = match parse_artifact(name, &json) {
            Ok(ir) => ir,
            Err(e) => {
                eprintln!("abi-typegen: skipping {} — {}", name, e);
                continue;
            }
        };

        let files = generate_contract_files(&ir, config);
        for (filename, content) in &files {
            let dest = config.out_dir.join(filename);
            std::fs::write(&dest, content)
                .with_context(|| format!("cannot write '{}'", dest.display()))?;
            generated_files.insert(filename.clone());
        }

        contract_names.push(name.clone());
    }

    if contract_names.is_empty() {
        println!("abi-typegen: no contracts generated");
        return Ok(());
    }

    let barrel_content = barrel::render_barrel(&contract_names, config);
    let barrel_path = config.out_dir.join("index.ts");
    std::fs::write(&barrel_path, barrel_content)
        .with_context(|| format!("cannot write '{}'", barrel_path.display()))?;
    generated_files.insert("index.ts".to_string());

    if clean {
        clean_stale_files(&config.out_dir, &generated_files)?;
    }

    println!(
        "abi-typegen: generated {} contract(s) → {}",
        contract_names.len(),
        config.out_dir.display()
    );

    Ok(())
}

/// Removes files from `out_dir` that look like generated files but were not
/// produced during this run.
fn clean_stale_files(out_dir: &Path, generated_files: &HashSet<String>) -> Result<()> {
    let entries = match std::fs::read_dir(out_dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
            if is_generated_filename(filename) && !generated_files.contains(filename) {
                std::fs::remove_file(&path)
                    .with_context(|| format!("cannot remove stale file '{}'", path.display()))?;
                println!("abi-typegen: removed stale file '{}'", filename);
            }
        }
    }
    Ok(())
}

fn clean_stale_target_dirs(
    base_out_dir: &Path,
    active_target_dirs: &HashSet<String>,
) -> Result<()> {
    for dir_name in stale_target_dirs(base_out_dir, active_target_dirs)? {
        let path = base_out_dir.join(&dir_name);
        std::fs::remove_dir_all(&path).with_context(|| {
            format!("cannot remove stale target directory '{}'", path.display())
        })?;
        println!("abi-typegen: removed stale target directory '{}'", dir_name);
    }

    Ok(())
}

fn ensure_no_stale_target_dirs(
    base_out_dir: &Path,
    active_target_dirs: &HashSet<String>,
) -> Result<()> {
    let stale_dirs = stale_target_dirs(base_out_dir, active_target_dirs)?;
    if stale_dirs.is_empty() {
        return Ok(());
    }

    for dir_name in &stale_dirs {
        eprintln!("abi-typegen: stale: {}", dir_name);
    }

    anyhow::bail!(
        "output is not up to date ({} path(s) stale)",
        stale_dirs.len()
    )
}

fn stale_target_dirs(
    base_out_dir: &Path,
    active_target_dirs: &HashSet<String>,
) -> Result<Vec<String>> {
    let entries = match std::fs::read_dir(base_out_dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(Vec::new()),
    };

    let mut stale_dirs = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let Some(dir_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if GENERATED_TARGET_DIRS.contains(&dir_name) && !active_target_dirs.contains(dir_name) {
            stale_dirs.push(dir_name.to_string());
        }
    }

    stale_dirs.sort();
    Ok(stale_dirs)
}

/// Generates files to a temp directory and compares against existing output.
/// Returns an error if any file differs or is missing.
fn run_check(config: &Config) -> Result<()> {
    if !config.artifacts_dir.exists() {
        anyhow::bail!(
            "Foundry out directory '{}' does not exist. Run `forge build` first.",
            config.artifacts_dir.display()
        );
    }

    let temp_dir = tempfile::tempdir().context("cannot create temp directory for --check")?;
    let temp_config = Config {
        artifacts_dir: config.artifacts_dir.clone(),
        out_dir: temp_dir.path().to_path_buf(),
        targets: config.targets.clone(),
        wrappers: config.wrappers,
        contracts: config.contracts.clone(),
        exclude: config.exclude.clone(),
    };

    run_generate(&temp_config, false)?;

    let mut stale = Vec::new();
    let mut expected_files = HashSet::new();

    // Compare each generated file against the existing output
    let entries = match std::fs::read_dir(temp_dir.path()) {
        Ok(e) => e,
        Err(_) => {
            println!("abi-typegen: output is up to date");
            return Ok(());
        }
    };
    for entry in entries {
        let entry = entry?;
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy().to_string();
        expected_files.insert(filename_str.clone());
        let expected = std::fs::read_to_string(entry.path())?;
        let existing_path = config.out_dir.join(&filename_str);
        match std::fs::read_to_string(&existing_path) {
            Ok(existing) if existing == expected => {}
            Ok(_) => stale.push(filename_str),
            Err(_) => stale.push(filename_str),
        }
    }

    if let Ok(entries) = std::fs::read_dir(&config.out_dir) {
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                if is_generated_filename(filename) && !expected_files.contains(filename) {
                    stale.push(filename.to_string());
                }
            }
        }
    }

    if stale.is_empty() {
        println!("abi-typegen: output is up to date");
        Ok(())
    } else {
        stale.sort();
        stale.dedup();
        for f in &stale {
            eprintln!("abi-typegen: stale: {}", f);
        }
        anyhow::bail!("output is not up to date ({} file(s) stale)", stale.len())
    }
}

/// Discovers all `out/<Name>.sol/<Name>.json` artifacts.
fn discover_artifacts(artifacts_dir: &Path, filter: &[String]) -> Result<Vec<(String, PathBuf)>> {
    let mut results = Vec::new();

    discover_artifacts_in_dir(artifacts_dir, filter, &mut results)?;

    results.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(results)
}

fn discover_artifacts_in_dir(
    artifacts_dir: &Path,
    filter: &[String],
    results: &mut Vec<(String, PathBuf)>,
) -> Result<()> {
    let entries = std::fs::read_dir(artifacts_dir)
        .with_context(|| format!("cannot read directory '{}'", artifacts_dir.display()))?;

    for entry in entries {
        let entry = entry
            .with_context(|| format!("error reading entry in '{}'", artifacts_dir.display()))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(dir_name) = path.file_name().and_then(|f| f.to_str()) else {
            continue;
        };
        if !dir_name.ends_with(".sol") {
            discover_artifacts_in_dir(&path, filter, results)?;
            continue;
        }
        let contract_name = dir_name.trim_end_matches(".sol").to_string();

        if !filter.is_empty() && !filter.contains(&contract_name) {
            continue;
        }

        let artifact_file = path.join(format!("{}.json", contract_name));
        if artifact_file.exists() {
            results.push((contract_name, artifact_file));
        }
    }

    Ok(())
}

// Excluded from coverage: creates a blocking OS file watcher that can't be
// cleanly terminated in tests. The core logic is in watch_loop() which is tested.
#[cfg_attr(coverage_nightly, coverage(off))]
fn run_watch(config: Config) -> Result<()> {
    use notify::{Config as NotifyConfig, RecommendedWatcher, RecursiveMode, Watcher};
    use std::sync::mpsc::channel;

    println!(
        "abi-typegen: watching '{}' for changes (Ctrl-C to stop)",
        config.artifacts_dir.display()
    );

    if let Err(e) = run_generate(&config, false) {
        eprintln!("abi-typegen: initial generate failed — {}", e);
    }

    let (tx, rx) = channel();
    let mut watcher = RecommendedWatcher::new(
        move |event| {
            let _ = tx.send(event);
        },
        NotifyConfig::default().with_poll_interval(std::time::Duration::from_millis(200)),
    )?;
    watcher.watch(config.artifacts_dir.as_ref(), RecursiveMode::Recursive)?;

    watch_loop(&config, &rx);
    Ok(())
}

/// Blocking event loop: waits for filesystem events and regenerates.
/// Returns when the channel is disconnected (all senders dropped).
fn watch_loop(config: &Config, rx: &std::sync::mpsc::Receiver<notify::Result<notify::Event>>) {
    use std::time::{Duration, Instant};

    const DEBOUNCE: Duration = Duration::from_millis(200);
    let mut last_generate = Instant::now() - DEBOUNCE;

    loop {
        match rx.recv() {
            Ok(Ok(_event)) => {
                // Debounce: drain pending events and wait for a quiet period
                let now = Instant::now();
                if now.duration_since(last_generate) < DEBOUNCE {
                    while rx.recv_timeout(DEBOUNCE).is_ok() {}
                }
                last_generate = Instant::now();
                println!("abi-typegen: change detected — regenerating...");
                if let Err(e) = run_generate(config, false) {
                    eprintln!("abi-typegen: generate error — {}", e);
                }
            }
            Ok(Err(e)) => eprintln!("abi-typegen: watch error — {}", e),
            Err(_) => break,
        }
    }
}

fn run_init(config_path: &Path) -> Result<()> {
    const SCAFFOLD: &str = r#"
[abi-typegen]
out = "src/generated"
target = "viem"
wrappers = true
contracts = []
"#;

    if config_path.exists() {
        let content = std::fs::read_to_string(config_path)?;
        if content.contains("[abi-typegen]") {
            println!(
                "abi-typegen: [abi-typegen] section already exists in {}",
                config_path.display()
            );
            return Ok(());
        }
        let mut new_content = content;
        new_content.push_str(SCAFFOLD);
        std::fs::write(config_path, new_content)?;
    } else {
        std::fs::write(config_path, SCAFFOLD.trim_start())?;
    }

    println!(
        "abi-typegen: added [abi-typegen] config to {}",
        config_path.display()
    );
    println!("  → Run `abi-typegen forge-install | source /dev/stdin` to enable `forge typegen`");
    Ok(())
}

fn run_shell(shell: &str) {
    match shell {
        "fish" => {
            println!(
                r#"# Add to ~/.config/fish/config.fish
# Enables `forge typegen` as a Forge subcommand

function forge
    if test "$argv[1]" = "typegen"
        abi-typegen $argv[2..]
    else
        command forge $argv
    end
end"#
            );
        }
        _ => {
            // bash and zsh use the same syntax
            println!(
                r#"# Add to ~/.bashrc or ~/.zshrc
# Enables `forge typegen` as a Forge subcommand

forge() {{
  if [ "$1" = "typegen" ]; then
    shift
    abi-typegen "$@"
  else
    command forge "$@"
  fi
}}"#
            );
        }
    }
}

fn run_diff(config: &Config) -> Result<()> {
    if !config.artifacts_dir.exists() {
        anyhow::bail!(
            "Artifact directory '{}' does not exist.",
            config.artifacts_dir.display()
        );
    }
    let artifacts = selected_artifacts(config)?;
    if artifacts.is_empty() {
        println!("abi-typegen: no artifacts found");
        return Ok(());
    }
    let diffs = collect_diff_entries(config, &artifacts)?;
    if diffs.is_empty() {
        println!("abi-typegen: output is up to date");
    } else {
        for diff in &diffs {
            println!("{}", diff);
        }
        std::process::exit(1);
    }
    Ok(())
}

fn collect_diff_entries(config: &Config, artifacts: &[(String, PathBuf)]) -> Result<Vec<String>> {
    let mut diffs = Vec::new();
    let mut expected_files = HashSet::new();
    let mut contract_names = Vec::new();

    for (name, path) in artifacts {
        let json = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read artifact '{}'", path.display()))?;
        let ir = match parse_artifact(name, &json) {
            Ok(ir) => ir,
            Err(e) => {
                eprintln!("abi-typegen: skipping {} — {}", name, e);
                continue;
            }
        };
        let files = generate_contract_files(&ir, config);
        for (filename, content) in &files {
            expected_files.insert(filename.clone());
            let dest = config.out_dir.join(filename);
            if dest.exists() {
                let existing = std::fs::read_to_string(&dest)?;
                if existing != *content {
                    diffs.push(format!("M {}", filename));
                }
            } else {
                diffs.push(format!("A {}", filename));
            }
        }
        contract_names.push(name.clone());
    }

    // Check the barrel file the same way run_generate does.
    if !contract_names.is_empty() {
        let barrel_content = barrel::render_barrel(&contract_names, config);
        let barrel_path = config.out_dir.join("index.ts");
        expected_files.insert("index.ts".to_string());
        if barrel_path.exists() {
            let existing = std::fs::read_to_string(&barrel_path)?;
            if existing != barrel_content {
                diffs.push("M index.ts".to_string());
            }
        } else {
            diffs.push("A index.ts".to_string());
        }
    }

    if let Ok(entries) = std::fs::read_dir(&config.out_dir) {
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            if let Some(filename) = path.file_name().and_then(|name| name.to_str()) {
                if is_generated_filename(filename) && !expected_files.contains(filename) {
                    diffs.push(format!("D {}", filename));
                }
            }
        }
    }

    diffs.sort();
    diffs.dedup();
    Ok(diffs)
}

fn run_json(config: &Config, pretty: bool) -> Result<()> {
    if !config.artifacts_dir.exists() {
        anyhow::bail!(
            "Artifact directory '{}' does not exist.",
            config.artifacts_dir.display()
        );
    }
    let ir_list = collect_json_summaries(config)?;
    let output = if pretty {
        serde_json::to_string_pretty(&ir_list)?
    } else {
        serde_json::to_string(&ir_list)?
    };
    println!("{}", output);
    Ok(())
}

fn collect_json_summaries(config: &Config) -> Result<Vec<serde_json::Value>> {
    let artifacts = selected_artifacts(config)?;
    let mut ir_list = Vec::new();

    for (name, path) in &artifacts {
        let json = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read artifact '{}'", path.display()))?;
        match parse_artifact(name, &json) {
            Ok(ir) => ir_list.push(serde_json::json!({
                "name": ir.name, "abi": ir.raw_abi,
                "functions": ir.functions.len(), "events": ir.events.len(),
                "errors": ir.errors.len(), "has_constructor": ir.constructor.is_some(),
                "has_fallback": ir.has_fallback, "has_receive": ir.has_receive,
            })),
            Err(e) => {
                eprintln!("abi-typegen: skipping {} — {}", name, e);
            }
        }
    }

    Ok(ir_list)
}

#[cfg(test)]
mod tests;
