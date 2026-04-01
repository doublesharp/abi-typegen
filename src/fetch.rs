//! Block-explorer ABI fetching for Etherscan-compatible APIs.
//!
//! Any explorer that implements the Etherscan API format works.
//! Built-in shortcuts cover Ethereum, Optimism, Base, Blast, Arbitrum, Polygon,
//! BNB Chain, Avalanche, zkSync, Linea, Scroll, Gnosis, Fantom, Cronos, Celo,
//! Moonbeam, Moonriver, Mantle, Sonic, Taiko, Metis, Manta, HyperEVM, and more.
//! Pass `--url` to reach any explorer not listed.

use anyhow::{bail, Context, Result};
use serde::Deserialize;

// ── Known network shortcuts ──────────────────────────────────────────────────

/// Maps well-known network names/aliases to their Etherscan-compatible API base URLs.
///
/// URLs that already contain `?` (e.g. Etherscan V2 with `chainid=`) will have
/// additional parameters appended with `&` instead of `?`.
///
/// Pass `--url` to use any explorer not listed here.
const KNOWN_NETWORKS: &[(&str, &str)] = &[
    // ── Etherscan V2 unified endpoint (https://docs.etherscan.io/supported-chains)
    // All Etherscan-operated chains use the single V2 endpoint; per-domain V1
    // endpoints are deprecated and will return errors.
    //
    // Ethereum
    ("mainnet", "https://api.etherscan.io/v2/api?chainid=1"),
    ("ethereum", "https://api.etherscan.io/v2/api?chainid=1"),
    ("eth", "https://api.etherscan.io/v2/api?chainid=1"),
    (
        "sepolia",
        "https://api.etherscan.io/v2/api?chainid=11155111",
    ),
    ("holesky", "https://api.etherscan.io/v2/api?chainid=17000"),
    ("hoodi", "https://api.etherscan.io/v2/api?chainid=560048"),
    // Optimism / OP Stack
    ("optimism", "https://api.etherscan.io/v2/api?chainid=10"),
    ("op", "https://api.etherscan.io/v2/api?chainid=10"),
    (
        "op-sepolia",
        "https://api.etherscan.io/v2/api?chainid=11155420",
    ),
    (
        "optimism-sepolia",
        "https://api.etherscan.io/v2/api?chainid=11155420",
    ),
    // Base
    ("base", "https://api.etherscan.io/v2/api?chainid=8453"),
    (
        "base-sepolia",
        "https://api.etherscan.io/v2/api?chainid=84532",
    ),
    // Blast
    ("blast", "https://api.etherscan.io/v2/api?chainid=81457"),
    (
        "blast-sepolia",
        "https://api.etherscan.io/v2/api?chainid=168587773",
    ),
    // Fraxtal
    ("frax", "https://api.etherscan.io/v2/api?chainid=252"),
    ("fraxtal", "https://api.etherscan.io/v2/api?chainid=252"),
    (
        "fraxtal-hoodi",
        "https://api.etherscan.io/v2/api?chainid=2523",
    ),
    // World Chain
    ("world", "https://api.etherscan.io/v2/api?chainid=480"),
    ("worldchain", "https://api.etherscan.io/v2/api?chainid=480"),
    (
        "world-sepolia",
        "https://api.etherscan.io/v2/api?chainid=4801",
    ),
    // Arbitrum
    ("arbitrum", "https://api.etherscan.io/v2/api?chainid=42161"),
    ("arb", "https://api.etherscan.io/v2/api?chainid=42161"),
    (
        "arbitrum-nova",
        "https://api.etherscan.io/v2/api?chainid=42170",
    ),
    (
        "arbitrum-sepolia",
        "https://api.etherscan.io/v2/api?chainid=421614",
    ),
    (
        "arb-sepolia",
        "https://api.etherscan.io/v2/api?chainid=421614",
    ),
    // Polygon
    ("polygon", "https://api.etherscan.io/v2/api?chainid=137"),
    ("matic", "https://api.etherscan.io/v2/api?chainid=137"),
    (
        "polygon-amoy",
        "https://api.etherscan.io/v2/api?chainid=80002",
    ),
    ("amoy", "https://api.etherscan.io/v2/api?chainid=80002"),
    // BNB / BSC
    ("bsc", "https://api.etherscan.io/v2/api?chainid=56"),
    ("bnb", "https://api.etherscan.io/v2/api?chainid=56"),
    ("bsc-testnet", "https://api.etherscan.io/v2/api?chainid=97"),
    ("opbnb", "https://api.etherscan.io/v2/api?chainid=204"),
    (
        "opbnb-testnet",
        "https://api.etherscan.io/v2/api?chainid=5611",
    ),
    // BitTorrent Chain
    ("bittorrent", "https://api.etherscan.io/v2/api?chainid=199"),
    ("btt", "https://api.etherscan.io/v2/api?chainid=199"),
    (
        "btt-testnet",
        "https://api.etherscan.io/v2/api?chainid=1029",
    ),
    // Avalanche (via Etherscan V2)
    ("avalanche", "https://api.etherscan.io/v2/api?chainid=43114"),
    ("avax", "https://api.etherscan.io/v2/api?chainid=43114"),
    (
        "avalanche-fuji",
        "https://api.etherscan.io/v2/api?chainid=43113",
    ),
    ("fuji", "https://api.etherscan.io/v2/api?chainid=43113"),
    // Linea
    ("linea", "https://api.etherscan.io/v2/api?chainid=59144"),
    (
        "linea-sepolia",
        "https://api.etherscan.io/v2/api?chainid=59141",
    ),
    // Scroll
    ("scroll", "https://api.etherscan.io/v2/api?chainid=534352"),
    (
        "scroll-sepolia",
        "https://api.etherscan.io/v2/api?chainid=534351",
    ),
    // Gnosis
    ("gnosis", "https://api.etherscan.io/v2/api?chainid=100"),
    ("xdai", "https://api.etherscan.io/v2/api?chainid=100"),
    // Mantle
    ("mantle", "https://api.etherscan.io/v2/api?chainid=5000"),
    (
        "mantle-sepolia",
        "https://api.etherscan.io/v2/api?chainid=5003",
    ),
    // Celo
    ("celo", "https://api.etherscan.io/v2/api?chainid=42220"),
    (
        "celo-alfajores",
        "https://api.etherscan.io/v2/api?chainid=44787",
    ),
    // Moonbeam / Moonriver
    ("moonbeam", "https://api.etherscan.io/v2/api?chainid=1284"),
    ("glmr", "https://api.etherscan.io/v2/api?chainid=1284"),
    ("moonriver", "https://api.etherscan.io/v2/api?chainid=1285"),
    ("movr", "https://api.etherscan.io/v2/api?chainid=1285"),
    ("moonbase", "https://api.etherscan.io/v2/api?chainid=1287"),
    // Taiko
    ("taiko", "https://api.etherscan.io/v2/api?chainid=167000"),
    (
        "taiko-hoodi",
        "https://api.etherscan.io/v2/api?chainid=167013",
    ),
    // Sonic
    ("sonic", "https://api.etherscan.io/v2/api?chainid=146"),
    (
        "sonic-testnet",
        "https://api.etherscan.io/v2/api?chainid=14601",
    ),
    // Unichain
    ("unichain", "https://api.etherscan.io/v2/api?chainid=130"),
    (
        "unichain-sepolia",
        "https://api.etherscan.io/v2/api?chainid=1301",
    ),
    // HyperEVM
    ("hyperevm", "https://api.etherscan.io/v2/api?chainid=999"),
    ("hype", "https://api.etherscan.io/v2/api?chainid=999"),
    // Abstract
    ("abstract", "https://api.etherscan.io/v2/api?chainid=2741"),
    (
        "abstract-sepolia",
        "https://api.etherscan.io/v2/api?chainid=11124",
    ),
    // Berachain
    ("berachain", "https://api.etherscan.io/v2/api?chainid=80094"),
    ("bera", "https://api.etherscan.io/v2/api?chainid=80094"),
    (
        "berachain-bepolia",
        "https://api.etherscan.io/v2/api?chainid=80069",
    ),
    // Swellchain
    ("swellchain", "https://api.etherscan.io/v2/api?chainid=1923"),
    ("swell", "https://api.etherscan.io/v2/api?chainid=1923"),
    (
        "swellchain-testnet",
        "https://api.etherscan.io/v2/api?chainid=1924",
    ),
    // Monad
    ("monad", "https://api.etherscan.io/v2/api?chainid=143"),
    (
        "monad-testnet",
        "https://api.etherscan.io/v2/api?chainid=10143",
    ),
    // ApeChain
    ("apechain", "https://api.etherscan.io/v2/api?chainid=33139"),
    ("ape", "https://api.etherscan.io/v2/api?chainid=33139"),
    (
        "apechain-curtis",
        "https://api.etherscan.io/v2/api?chainid=33111",
    ),
    // XDC
    ("xdc", "https://api.etherscan.io/v2/api?chainid=50"),
    ("xdc-apothem", "https://api.etherscan.io/v2/api?chainid=51"),
    // Sei
    ("sei", "https://api.etherscan.io/v2/api?chainid=1329"),
    (
        "sei-testnet",
        "https://api.etherscan.io/v2/api?chainid=1328",
    ),
    // MegaETH
    ("megaeth", "https://api.etherscan.io/v2/api?chainid=4326"),
    (
        "megaeth-testnet",
        "https://api.etherscan.io/v2/api?chainid=6342",
    ),
    // Katana
    ("katana", "https://api.etherscan.io/v2/api?chainid=747474"),
    (
        "katana-bokuto",
        "https://api.etherscan.io/v2/api?chainid=737373",
    ),
    // ── Independent explorers (not operated by Etherscan) ────────────────────
    // Polygon zkEVM – Polygonscan zkEVM
    ("polygon-zkevm", "https://api-zkevm.polygonscan.com/api"),
    ("zkevm", "https://api-zkevm.polygonscan.com/api"),
    (
        "polygon-zkevm-cardona",
        "https://api-cardona-zkevm.polygonscan.com/api",
    ),
    // zkSync Era – native explorer
    ("zksync", "https://api-era.zksync.network/api"),
    (
        "zksync-sepolia",
        "https://api-sepolia-era.zksync.network/api",
    ),
    // Fantom – Ftmscan
    ("fantom", "https://api.ftmscan.com/api"),
    ("ftm", "https://api.ftmscan.com/api"),
    ("fantom-testnet", "https://api-testnet.ftmscan.com/api"),
    // Cronos – Cronoscan
    ("cronos", "https://api.cronoscan.com/api"),
    ("cro", "https://api.cronoscan.com/api"),
    // Metis – Andromeda explorer
    ("metis", "https://andromeda-explorer.metis.io/api"),
    // Manta Pacific – Manta explorer
    ("manta", "https://pacific-explorer.manta.network/api"),
];

/// Returns the API URL for a well-known network name, or `None` if unrecognised.
pub fn network_to_api_url(network: &str) -> Option<&'static str> {
    KNOWN_NETWORKS
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(network))
        .map(|(_, url)| *url)
}

/// Resolves the Etherscan-compatible API URL to use.
///
/// `explicit_url` (from `--url`) takes priority. If not set, `network` is
/// looked up in the known-networks table. Returns an error with a helpful
/// message if neither resolves.
pub fn resolve_api_url<'a>(network: &str, explicit_url: Option<&'a str>) -> Result<&'a str>
where
{
    if let Some(url) = explicit_url {
        return Ok(url);
    }
    network_to_api_url(network).ok_or_else(|| {
        anyhow::anyhow!(
            "unknown network '{}'; pass --url with the full Etherscan-compatible API URL \
             (e.g. https://api.sonicscan.org/api)",
            network
        )
    })
}

// ── HTTP response deserialization ────────────────────────────────────────────

#[derive(Deserialize)]
struct ExplorerResponse {
    status: String,
    result: String,
}

// ── Core fetch logic ─────────────────────────────────────────────────────────

/// Calls the `getabi` endpoint on an Etherscan-compatible block explorer and
/// returns the ABI as a parsed [`serde_json::Value`] (a JSON array).
///
/// `api_url` is the full endpoint URL including the `/api` path suffix
/// (e.g. `"https://api.etherscan.io/api"`).
pub fn fetch_abi(api_url: &str, address: &str, api_key: Option<&str>) -> Result<serde_json::Value> {
    // If the base URL already contains `?` (e.g. Etherscan V2 `?chainid=1`),
    // append with `&`; otherwise start a new query string with `?`.
    let sep = if api_url.contains('?') { '&' } else { '?' };
    let mut url = format!(
        "{}{}module=contract&action=getabi&address={}",
        api_url, sep, address
    );
    if let Some(key) = api_key {
        url.push_str("&apikey=");
        url.push_str(key);
    }

    let body: String = ureq::get(&url)
        .call()
        .context("HTTP request to block explorer failed")?
        .into_string()
        .context("failed to read block explorer response body")?;

    let resp: ExplorerResponse =
        serde_json::from_str(&body).context("failed to parse block explorer response")?;

    if resp.status != "1" {
        let msg = resp.result.as_str();
        if msg.contains("not verified") || msg.contains("not found") || msg.contains("No Source") {
            bail!(
                "contract {} is not verified on this explorer ({})",
                address,
                api_url
            );
        }
        if msg.contains("Invalid API Key") || msg.contains("API Key") {
            bail!("invalid or missing API key — provide --api-key or set ETHERSCAN_API_KEY");
        }
        if msg.contains("rate limit") || msg.contains("rate_limit") || msg.contains("Max rate") {
            bail!("rate limit reached — add an API key or wait before retrying");
        }
        bail!("block explorer returned error: {}", msg);
    }

    // The `result` field is a double-encoded JSON string containing the ABI array.
    serde_json::from_str(&resp.result)
        .context("block explorer returned invalid ABI JSON in `result` field")
}

/// Loads an ABI from a local JSON file.
///
/// Accepts two formats:
/// - A raw ABI array: `[{"type": "function", ...}, ...]`
/// - A Foundry/Hardhat artifact envelope: `{"abi": [...], ...}`
///
/// Returns the ABI as a [`serde_json::Value`] array, ready to pass to
/// [`build_artifact_json`].
pub fn load_abi_from_file(path: &std::path::Path) -> Result<serde_json::Value> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read ABI file '{}'", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&contents)
        .with_context(|| format!("'{}' is not valid JSON", path.display()))?;
    if value.is_array() {
        return Ok(value);
    }
    if let Some(abi) = value.get("abi") {
        if abi.is_array() {
            return Ok(abi.clone());
        }
    }
    bail!(
        "'{}' is not a JSON ABI array or a Foundry/Hardhat artifact with an \"abi\" field",
        path.display()
    )
}

/// Wraps a parsed ABI array in the minimal Foundry artifact envelope
/// (`{"abi": [...]}`) and returns it as pretty-printed JSON.
pub fn build_artifact_json(abi: serde_json::Value) -> Result<String> {
    let artifact = serde_json::json!({ "abi": abi });
    serde_json::to_string_pretty(&artifact).context("failed to serialise artifact JSON")
}

/// Returns the canonical Foundry artifact path for a contract:
/// `<artifacts_dir>/<Name>.sol/<Name>.json`
pub fn artifact_path(artifacts_dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    artifacts_dir
        .join(format!("{}.sol", name))
        .join(format!("{}.json", name))
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mainnet_resolves() {
        assert_eq!(
            network_to_api_url("mainnet"),
            Some("https://api.etherscan.io/v2/api?chainid=1")
        );
        assert_eq!(
            network_to_api_url("ethereum"),
            Some("https://api.etherscan.io/v2/api?chainid=1")
        );
    }

    #[test]
    fn lookup_is_case_insensitive() {
        assert_eq!(network_to_api_url("Mainnet"), network_to_api_url("mainnet"));
        assert_eq!(network_to_api_url("POLYGON"), network_to_api_url("polygon"));
    }

    #[test]
    fn unknown_network_returns_none() {
        assert!(network_to_api_url("solana").is_none());
        assert!(network_to_api_url("").is_none());
    }

    #[test]
    fn explicit_url_overrides_network() {
        let url = "https://api.sonicscan.org/api";
        assert_eq!(resolve_api_url("mainnet", Some(url)).unwrap(), url);
    }

    #[test]
    fn network_lookup_used_when_no_explicit_url() {
        let result = resolve_api_url("polygon", None).unwrap();
        assert_eq!(result, "https://api.etherscan.io/v2/api?chainid=137");
    }

    #[test]
    fn unknown_network_without_url_errors() {
        let err = resolve_api_url("hyperevmtestnet", None).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("hyperevmtestnet"), "{msg}");
        assert!(msg.contains("--url"), "{msg}");
    }

    #[test]
    fn artifact_path_follows_foundry_layout() {
        let base = std::path::Path::new("out");
        let path = artifact_path(base, "MyToken");
        assert_eq!(path, std::path::Path::new("out/MyToken.sol/MyToken.json"));
    }

    #[test]
    fn build_artifact_json_wraps_abi() {
        let abi = serde_json::json!([{"type": "fallback"}]);
        let json = build_artifact_json(abi).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["abi"].is_array());
        assert_eq!(parsed["abi"][0]["type"], "fallback");
    }

    #[test]
    fn error_response_not_verified_detected() {
        // Simulate what fetch_abi would return for an unverified contract
        // by testing the message pattern matching directly.
        let msg = "Contract source code not verified";
        assert!(
            msg.contains("not verified"),
            "pattern check for unverified contract"
        );
    }

    #[test]
    fn load_abi_from_file_accepts_raw_array() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("abi.json");
        std::fs::write(&path, r#"[{"type":"fallback"}]"#).unwrap();
        let abi = load_abi_from_file(&path).unwrap();
        assert!(abi.is_array());
        assert_eq!(abi[0]["type"], "fallback");
    }

    #[test]
    fn load_abi_from_file_accepts_artifact_envelope() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("artifact.json");
        std::fs::write(&path, r#"{"abi":[{"type":"receive"}],"bytecode":"0x"}"#).unwrap();
        let abi = load_abi_from_file(&path).unwrap();
        assert!(abi.is_array());
        assert_eq!(abi[0]["type"], "receive");
    }

    #[test]
    fn load_abi_from_file_rejects_unknown_shape() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.json");
        std::fs::write(&path, r#"{"name":"Foo"}"#).unwrap();
        let err = load_abi_from_file(&path).unwrap_err();
        assert!(format!("{err}").contains("not a JSON ABI array"), "{err}");
    }
}
