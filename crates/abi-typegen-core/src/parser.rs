//! Foundry artifact JSON parser — deserializes ABI and NatSpec into [`ContractIr`].

use crate::types::*;
use serde::Deserialize;
use std::collections::HashMap;

/// Error returned by artifact and type-string parsing functions.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// The artifact JSON could not be parsed.
    #[error("failed to parse artifact JSON: {0}")]
    Json(#[from] serde_json::Error),
    /// A type string did not match any known Solidity type.
    #[error("unknown Solidity type: '{0}'")]
    UnknownType(String),
    /// A `uintN`/`intN` bit width was out of range or not a multiple of 8.
    #[error("invalid bit width in '{0}': must be a multiple of 8 in range 8–256")]
    InvalidBitWidth(String),
    /// A `bytesN` size was out of range; valid range is 1–32.
    #[error("invalid bytesN size in '{0}': must be in range 1–32")]
    InvalidBytesN(String),
    /// A tuple ABI type had no components.
    #[error("tuple has no components")]
    EmptyTuple,
    /// A type parse error with additional context about which parameter it occurred in.
    #[error("in {context}: {source}")]
    InParam {
        /// Human-readable context (e.g. `"param 'amount'"`).
        context: String,
        /// The underlying parse error.
        #[source]
        source: Box<ParseError>,
    },
}

/// Convenience alias used throughout this module.
type Result<T> = std::result::Result<T, ParseError>;

// ── Raw artifact deserialization ────────────────────────────────────────────

#[derive(Deserialize)]
struct Artifact {
    abi: serde_json::Value,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
}

#[derive(Deserialize, Default)]
struct RawDevDoc {
    #[allow(dead_code)]
    title: Option<String>,
    details: Option<String>,
    #[serde(default)]
    methods: HashMap<String, RawDocEntry>,
    #[serde(default)]
    events: HashMap<String, RawDocEntry>,
    #[serde(default)]
    errors: HashMap<String, RawDocEntry>,
}

#[derive(Deserialize, Default)]
struct RawUserDoc {
    notice: Option<String>,
    #[serde(default)]
    methods: HashMap<String, RawUserEntry>,
    #[serde(default)]
    events: HashMap<String, RawUserEntry>,
    #[serde(default)]
    errors: HashMap<String, RawUserEntry>,
}

#[derive(Deserialize, Default)]
struct RawDocEntry {
    details: Option<String>,
    #[serde(default)]
    params: HashMap<String, String>,
    #[serde(default)]
    returns: HashMap<String, String>,
}

#[derive(Deserialize, Default)]
struct RawUserEntry {
    notice: Option<String>,
}

// ── ABI item deserialization ────────────────────────────────────────────────

#[derive(Deserialize)]
struct RawAbiItem {
    #[serde(rename = "type")]
    ty: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    inputs: Vec<RawParam>,
    #[serde(default)]
    outputs: Vec<RawParam>,
    #[serde(rename = "stateMutability")]
    state_mutability: Option<String>,
    #[serde(default)]
    anonymous: bool,
}

#[derive(Deserialize, Clone)]
pub(crate) struct RawParam {
    #[serde(default)]
    name: String,
    #[serde(rename = "type")]
    ty: String,
    #[serde(rename = "internalType")]
    internal_type: Option<String>,
    #[serde(default)]
    components: Vec<RawParam>,
    #[serde(default)]
    indexed: bool,
}

// ── SolType parsing ─────────────────────────────────────────────────────────

pub(crate) fn parse_sol_type(ty_str: &str, components: &[RawParam]) -> Result<SolType> {
    // Dynamic array: strip trailing []
    if let Some(rest) = ty_str.strip_suffix("[]") {
        let inner = parse_sol_type(rest, components)?;
        return Ok(SolType::Array(Box::new(inner)));
    }

    // Fixed array: strip trailing [N]
    if ty_str.ends_with(']')
        && let Some(bracket) = ty_str.rfind('[')
        && let Ok(size) = ty_str[bracket + 1..ty_str.len() - 1].parse::<usize>()
    {
        let inner = parse_sol_type(&ty_str[..bracket], components)?;
        return Ok(SolType::FixedArray(Box::new(inner), size));
    }

    match ty_str {
        "bool" => Ok(SolType::Bool),
        "address" => Ok(SolType::Address),
        "bytes" => Ok(SolType::Bytes),
        "string" => Ok(SolType::StringType),
        "tuple" => {
            if components.is_empty() {
                return Err(ParseError::EmptyTuple);
            }
            let fields = components
                .iter()
                .map(|c| {
                    Ok(TupleComponent {
                        name: c.name.clone(),
                        ty: parse_sol_type(&c.ty, &c.components)?,
                        internal_type: c.internal_type.clone(),
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(SolType::Tuple(fields))
        }
        s if s.starts_with("uint") => {
            let suffix = &s[4..];
            let bits: u16 = if suffix.is_empty() {
                256
            } else {
                suffix
                    .parse()
                    .map_err(|_| ParseError::InvalidBitWidth(s.to_string()))?
            };
            if bits == 0 || bits > 256 || bits % 8 != 0 {
                return Err(ParseError::InvalidBitWidth(s.to_string()));
            }
            Ok(SolType::Uint(bits))
        }
        s if s.starts_with("int") => {
            let suffix = &s[3..];
            let bits: u16 = if suffix.is_empty() {
                256
            } else {
                suffix
                    .parse()
                    .map_err(|_| ParseError::InvalidBitWidth(s.to_string()))?
            };
            if bits == 0 || bits > 256 || bits % 8 != 0 {
                return Err(ParseError::InvalidBitWidth(s.to_string()));
            }
            Ok(SolType::Int(bits))
        }
        s if s.starts_with("bytes") && s.len() > 5 => {
            let n: u8 = s[5..]
                .parse()
                .map_err(|_| ParseError::InvalidBytesN(s.to_string()))?;
            if n == 0 || n > 32 {
                return Err(ParseError::InvalidBytesN(s.to_string()));
            }
            Ok(SolType::BytesN(n))
        }
        _ => Err(ParseError::UnknownType(ty_str.to_string())),
    }
}

fn raw_param_to_abi_param(p: &RawParam) -> Result<AbiParam> {
    Ok(AbiParam {
        name: p.name.clone(),
        ty: parse_sol_type(&p.ty, &p.components).map_err(|e| ParseError::InParam {
            context: format!("param '{}'", p.name),
            source: Box::new(e),
        })?,
        internal_type: p.internal_type.clone(),
    })
}

fn raw_param_to_event_param(p: &RawParam) -> Result<AbiEventParam> {
    Ok(AbiEventParam {
        name: p.name.clone(),
        ty: parse_sol_type(&p.ty, &p.components).map_err(|e| ParseError::InParam {
            context: format!("event param '{}'", p.name),
            source: Box::new(e),
        })?,
        indexed: p.indexed,
        internal_type: p.internal_type.clone(),
    })
}

fn parse_state_mutability(s: Option<&str>) -> StateMutability {
    match s {
        Some("pure") => StateMutability::Pure,
        Some("view") => StateMutability::View,
        Some("payable") => StateMutability::Payable,
        _ => StateMutability::NonPayable,
    }
}

// ── NatSpec extraction ──────────────────────────────────────────────────────

fn canonical_type_str(p: &RawParam) -> String {
    if p.ty == "tuple" || p.ty.starts_with("tuple[") {
        let inner: Vec<String> = p.components.iter().map(canonical_type_str).collect();
        let base = format!("({})", inner.join(","));
        // Safe: we already confirmed p.ty starts with "tuple"
        let suffix = &p.ty["tuple".len()..];
        format!("{}{}", base, suffix)
    } else {
        p.ty.clone()
    }
}

fn function_sig(name: &str, inputs: &[RawParam]) -> String {
    let params = inputs
        .iter()
        .map(canonical_type_str)
        .collect::<Vec<_>>()
        .join(",");
    format!("{}({})", name, params)
}

fn extract_natspec(
    sig: &str,
    devdoc_methods: &HashMap<String, RawDocEntry>,
    userdoc_methods: &HashMap<String, RawUserEntry>,
) -> Option<NatSpec> {
    let dev = devdoc_methods.get(sig);
    let user = userdoc_methods.get(sig);
    if dev.is_none() && user.is_none() {
        return None;
    }
    Some(NatSpec {
        notice: user.and_then(|u| u.notice.clone()),
        dev: dev.and_then(|d| d.details.clone()),
        params: dev.map(|d| d.params.clone()).unwrap_or_default(),
        returns: dev.map(|d| d.returns.clone()).unwrap_or_default(),
    })
}

// ── Main parse entry point ──────────────────────────────────────────────────

/// Parses a single Solidity type string (no tuple components) into a [`SolType`].
///
/// Useful for fuzz testing and validating type strings in isolation.
pub fn parse_type_string(ty_str: &str) -> Result<SolType> {
    parse_sol_type(ty_str, &[])
}

/// Parses a Foundry artifact JSON string into a [`ContractIr`].
///
/// `name` is the contract name (e.g. `"ERC20"`).
/// `json` is the full content of `out/<Name>.sol/<Name>.json`.
pub fn parse_artifact(name: &str, json: &str) -> Result<ContractIr> {
    let artifact: Artifact = serde_json::from_str(json)?;

    // Extract NatSpec docs
    let (devdoc, userdoc, contract_notice, contract_dev) = if let Some(meta) = &artifact.metadata {
        // Deserialize from &Value directly — avoids cloning the doc subtrees.
        let dev: RawDevDoc = meta
            .pointer("/output/devdoc")
            .and_then(|v| RawDevDoc::deserialize(v).ok())
            .unwrap_or_default();
        let user: RawUserDoc = meta
            .pointer("/output/userdoc")
            .and_then(|v| RawUserDoc::deserialize(v).ok())
            .unwrap_or_default();
        let cn = user.notice.clone();
        let cd = dev.details.clone();
        (dev, user, cn, cd)
    } else {
        (RawDevDoc::default(), RawUserDoc::default(), None, None)
    };

    let contract_natspec = if contract_notice.is_some() || contract_dev.is_some() {
        Some(NatSpec {
            notice: contract_notice,
            dev: contract_dev,
            ..Default::default()
        })
    } else {
        None
    };

    // Parse ABI items — deserialize from &Value to avoid cloning the entire ABI JSON tree.
    let raw_items: Vec<RawAbiItem> = Deserialize::deserialize(&artifact.abi)?;

    let mut functions = Vec::new();
    let mut events = Vec::new();
    let mut errors = Vec::new();
    let mut constructor = None;
    let mut has_fallback = false;
    let mut has_receive = false;

    for item in &raw_items {
        match item.ty.as_str() {
            "function" => {
                let name = item.name.as_deref().unwrap_or("").to_string();
                let inputs: Vec<AbiParam> = item
                    .inputs
                    .iter()
                    .map(raw_param_to_abi_param)
                    .collect::<Result<_>>()?;
                let outputs: Vec<AbiParam> = item
                    .outputs
                    .iter()
                    .map(raw_param_to_abi_param)
                    .collect::<Result<_>>()?;
                let sig = function_sig(&name, &item.inputs);
                let natspec = extract_natspec(&sig, &devdoc.methods, &userdoc.methods);
                functions.push(AbiFunction {
                    name,
                    inputs,
                    outputs,
                    state_mutability: parse_state_mutability(item.state_mutability.as_deref()),
                    natspec,
                });
            }
            "event" => {
                let name = item.name.as_deref().unwrap_or("").to_string();
                let inputs: Vec<AbiEventParam> = item
                    .inputs
                    .iter()
                    .map(raw_param_to_event_param)
                    .collect::<Result<_>>()?;
                let sig = function_sig(&name, &item.inputs);
                let natspec = extract_natspec(&sig, &devdoc.events, &userdoc.events);
                events.push(AbiEvent {
                    name,
                    inputs,
                    anonymous: item.anonymous,
                    natspec,
                });
            }
            "error" => {
                let name = item.name.as_deref().unwrap_or("").to_string();
                let inputs: Vec<AbiParam> = item
                    .inputs
                    .iter()
                    .map(raw_param_to_abi_param)
                    .collect::<Result<_>>()?;
                let sig = function_sig(&name, &item.inputs);
                let natspec = extract_natspec(&sig, &devdoc.errors, &userdoc.errors);
                errors.push(AbiError {
                    name,
                    inputs,
                    natspec,
                });
            }
            "constructor" => {
                let inputs: Vec<AbiParam> = item
                    .inputs
                    .iter()
                    .map(raw_param_to_abi_param)
                    .collect::<Result<_>>()?;
                constructor = Some(AbiConstructor {
                    inputs,
                    state_mutability: parse_state_mutability(item.state_mutability.as_deref()),
                });
            }
            "fallback" => has_fallback = true,
            "receive" => has_receive = true,
            unknown => {
                tracing::warn!(
                    item_type = unknown,
                    "abi-typegen: skipping unknown ABI item type"
                );
            }
        }
    }

    Ok(ContractIr {
        name: name.to_string(),
        constructor,
        functions,
        events,
        errors,
        has_fallback,
        has_receive,
        natspec: contract_natspec,
        raw_abi: artifact.abi,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_sol_type tests ─────────────────────────────────────────────

    #[test]
    fn parses_basic_types() {
        assert_eq!(parse_sol_type("bool", &[]).unwrap(), SolType::Bool);
        assert_eq!(parse_sol_type("address", &[]).unwrap(), SolType::Address);
        assert_eq!(parse_sol_type("bytes", &[]).unwrap(), SolType::Bytes);
        assert_eq!(parse_sol_type("string", &[]).unwrap(), SolType::StringType);
    }

    #[test]
    fn parses_uint_variants() {
        assert_eq!(parse_sol_type("uint256", &[]).unwrap(), SolType::Uint(256));
        assert_eq!(parse_sol_type("uint8", &[]).unwrap(), SolType::Uint(8));
        assert_eq!(parse_sol_type("uint", &[]).unwrap(), SolType::Uint(256));
        assert_eq!(parse_sol_type("int128", &[]).unwrap(), SolType::Int(128));
    }

    #[test]
    fn parses_bytesn() {
        assert_eq!(parse_sol_type("bytes32", &[]).unwrap(), SolType::BytesN(32));
        assert_eq!(parse_sol_type("bytes1", &[]).unwrap(), SolType::BytesN(1));
    }

    #[test]
    fn parses_dynamic_array() {
        assert_eq!(
            parse_sol_type("address[]", &[]).unwrap(),
            SolType::Array(Box::new(SolType::Address))
        );
        assert_eq!(
            parse_sol_type("uint256[]", &[]).unwrap(),
            SolType::Array(Box::new(SolType::Uint(256)))
        );
    }

    #[test]
    fn parses_fixed_array() {
        assert_eq!(
            parse_sol_type("uint256[3]", &[]).unwrap(),
            SolType::FixedArray(Box::new(SolType::Uint(256)), 3)
        );
    }

    #[test]
    fn parses_nested_array() {
        assert_eq!(
            parse_sol_type("address[][]", &[]).unwrap(),
            SolType::Array(Box::new(SolType::Array(Box::new(SolType::Address))))
        );
    }

    #[test]
    fn parses_tuple() {
        let comps = vec![
            RawParam {
                name: "shares".into(),
                ty: "uint256".into(),
                internal_type: None,
                components: vec![],
                indexed: false,
            },
            RawParam {
                name: "token".into(),
                ty: "address".into(),
                internal_type: Some("address".into()),
                components: vec![],
                indexed: false,
            },
        ];
        let ty = parse_sol_type("tuple", &comps).unwrap();
        assert_eq!(
            ty,
            SolType::Tuple(vec![
                TupleComponent {
                    name: "shares".into(),
                    ty: SolType::Uint(256),
                    internal_type: None,
                },
                TupleComponent {
                    name: "token".into(),
                    ty: SolType::Address,
                    internal_type: Some("address".into()),
                },
            ])
        );
    }

    #[test]
    fn unknown_type_errors() {
        assert!(parse_sol_type("mapping(address=>uint256)", &[]).is_err());
    }

    #[test]
    fn canonical_type_str_expands_tuple() {
        // Test that NatSpec lookup works for struct params
        // Build a minimal artifact JSON with a struct param function
        let json = r#"{
            "abi": [{
                "type": "function",
                "name": "doThing",
                "inputs": [{
                    "name": "pos",
                    "type": "tuple",
                    "internalType": "struct Foo.Position",
                    "components": [
                        {"name": "x", "type": "uint256", "internalType": "uint256", "components": []},
                        {"name": "y", "type": "address", "internalType": "address", "components": []}
                    ]
                }],
                "outputs": [],
                "stateMutability": "nonpayable"
            }],
            "bytecode": {"object": "0x"},
            "deployedBytecode": {"object": "0x"},
            "methodIdentifiers": {},
            "rawMetadata": "",
            "metadata": {
                "language": "Solidity",
                "output": {
                    "devdoc": {
                        "kind": "dev",
                        "version": 1,
                        "methods": {
                            "doThing((uint256,address))": {
                                "params": {"pos": "The position struct"}
                            }
                        }
                    },
                    "userdoc": {"kind": "user", "version": 1, "methods": {}}
                }
            }
        }"#;
        let ir = parse_artifact("Foo", json).unwrap();
        let f = ir.functions.iter().find(|f| f.name == "doThing").unwrap();
        let ns = f
            .natspec
            .as_ref()
            .expect("NatSpec should be found via canonical tuple signature");
        assert_eq!(
            ns.params.get("pos").map(|s| s.as_str()),
            Some("The position struct")
        );
    }

    // ── parse_artifact tests ─────────────────────────────────────────────

    fn erc20_json() -> String {
        std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/erc20.json"),
        )
        .unwrap()
    }

    fn vault_json() -> String {
        std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/vault.json"),
        )
        .unwrap()
    }

    fn minimal_json() -> String {
        std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/minimal.json"),
        )
        .unwrap()
    }

    #[test]
    fn parses_erc20() {
        let ir = parse_artifact("ERC20", &erc20_json()).unwrap();
        assert_eq!(ir.name, "ERC20");
        assert_eq!(ir.functions.len(), 9);
        assert_eq!(ir.events.len(), 2);
        assert_eq!(ir.errors.len(), 2);
        assert!(ir.constructor.is_some());
        assert!(!ir.has_fallback);
        assert!(!ir.has_receive);
    }

    #[test]
    fn erc20_natspec_extracted() {
        let ir = parse_artifact("ERC20", &erc20_json()).unwrap();
        let transfer = ir.functions.iter().find(|f| f.name == "transfer").unwrap();
        let ns = transfer.natspec.as_ref().unwrap();
        assert_eq!(ns.notice.as_deref(), Some("Transfer tokens to a recipient"));
        assert_eq!(
            ns.params.get("to").map(|s| s.as_str()),
            Some("Recipient address")
        );
    }

    #[test]
    fn parses_vault_with_overloads() {
        let ir = parse_artifact("Vault", &vault_json()).unwrap();
        let deposit_fns: Vec<_> = ir
            .functions
            .iter()
            .filter(|f| f.name == "deposit")
            .collect();
        assert_eq!(deposit_fns.len(), 2);
        assert_eq!(deposit_fns[0].inputs.len(), 1);
        assert_eq!(deposit_fns[1].inputs.len(), 2);
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn parses_vault_tuple_output() {
        let ir = parse_artifact("Vault", &vault_json()).unwrap();
        let get_pos = ir
            .functions
            .iter()
            .find(|f| f.name == "getPosition")
            .unwrap();
        assert_eq!(get_pos.outputs.len(), 1);
        assert!(matches!(&get_pos.outputs[0].ty, SolType::Tuple(c) if c.len() == 3));
        let SolType::Tuple(comps) = &get_pos.outputs[0].ty else {
            unreachable!()
        };
        assert_eq!(comps[0].name, "shares");
        assert_eq!(comps[0].ty, SolType::Uint(256));
    }

    #[test]
    fn parses_vault_receive() {
        let ir = parse_artifact("Vault", &vault_json()).unwrap();
        assert!(ir.has_receive);
        assert!(!ir.has_fallback);
    }

    #[test]
    fn parses_minimal() {
        let ir = parse_artifact("Minimal", &minimal_json()).unwrap();
        assert!(ir.constructor.is_some());
        assert!(ir.functions.is_empty());
        assert!(ir.events.is_empty());
        assert!(ir.errors.is_empty());
    }

    // ── malformed type rejection ────────────────────────────────────────

    #[test]
    fn rejects_malformed_uint() {
        assert!(parse_sol_type("uintfoo", &[]).is_err());
        assert!(parse_sol_type("uint-8", &[]).is_err());
        assert!(parse_sol_type("uint99999", &[]).is_err());
    }

    #[test]
    fn rejects_out_of_range_uint() {
        assert!(parse_sol_type("uint0", &[]).is_err());
        assert!(parse_sol_type("uint9", &[]).is_err(), "not a multiple of 8");
        assert!(parse_sol_type("uint257", &[]).is_err());
    }

    #[test]
    fn rejects_malformed_int() {
        assert!(parse_sol_type("intabc", &[]).is_err());
        assert!(parse_sol_type("int-16", &[]).is_err());
    }

    #[test]
    fn rejects_out_of_range_int() {
        assert!(parse_sol_type("int0", &[]).is_err());
        assert!(parse_sol_type("int9", &[]).is_err(), "not a multiple of 8");
        assert!(parse_sol_type("int257", &[]).is_err());
    }

    #[test]
    fn rejects_out_of_range_bytesn() {
        assert!(parse_sol_type("bytes0", &[]).is_err());
        assert!(parse_sol_type("bytes33", &[]).is_err());
    }

    #[test]
    fn bracket_with_non_numeric_size_falls_through() {
        // "uint256[abc]" has brackets but non-numeric size → falls through to unknown type
        assert!(parse_sol_type("uint256[abc]", &[]).is_err());
        // "address[xyz]" likewise
        assert!(parse_sol_type("address[xyz]", &[]).is_err());
    }

    #[test]
    fn closing_bracket_without_opening_falls_through() {
        // Ends with ']' but no '[' → rfind returns None → falls through
        assert!(parse_sol_type("weird]", &[]).is_err());
    }

    #[test]
    fn bare_uint_int_defaults_to_256() {
        assert_eq!(parse_sol_type("uint", &[]).unwrap(), SolType::Uint(256));
        assert_eq!(parse_sol_type("int", &[]).unwrap(), SolType::Int(256));
    }

    // ── unnamed param handling ──────────────────────────────────────────

    #[test]
    fn parses_unnamed_params() {
        let json = r#"{
            "abi": [{
                "type": "function",
                "name": "get",
                "inputs": [],
                "outputs": [
                    {"name": "", "type": "uint256", "internalType": "uint256", "components": []},
                    {"name": "", "type": "address", "internalType": "address", "components": []}
                ],
                "stateMutability": "view"
            }]
        }"#;
        let ir = parse_artifact("Test", json).unwrap();
        let f = &ir.functions[0];
        assert_eq!(f.outputs[0].name, "");
        assert_eq!(f.outputs[1].name, "");
    }

    // ── artifact without metadata ───────────────────────────────────────

    #[test]
    fn parses_artifact_without_metadata() {
        let json = r#"{"abi": [{"type": "receive", "stateMutability": "payable"}]}"#;
        let ir = parse_artifact("NoMeta", json).unwrap();
        assert!(ir.natspec.is_none());
        assert!(ir.has_receive);
    }

    // ── contract-level NatSpec ──────────────────────────────────────────

    #[test]
    fn contract_level_natspec() {
        let json = r#"{
            "abi": [],
            "metadata": {
                "output": {
                    "devdoc": {"details": "Dev description of contract"},
                    "userdoc": {"notice": "User notice for contract", "methods": {}}
                }
            }
        }"#;
        let ir = parse_artifact("Documented", json).unwrap();
        let ns = ir.natspec.as_ref().expect("should have contract natspec");
        assert_eq!(ns.notice.as_deref(), Some("User notice for contract"));
        assert_eq!(ns.dev.as_deref(), Some("Dev description of contract"));
    }

    #[test]
    fn contract_level_natspec_dev_only() {
        let json = r#"{
            "abi": [],
            "metadata": {
                "output": {
                    "devdoc": {"details": "internal docs"},
                    "userdoc": {"methods": {}}
                }
            }
        }"#;
        let ir = parse_artifact("DevOnly", json).unwrap();
        let ns = ir.natspec.as_ref().expect("should have contract natspec");
        assert!(ns.notice.is_none());
        assert_eq!(ns.dev.as_deref(), Some("internal docs"));
    }

    #[test]
    fn no_contract_natspec_when_absent() {
        let json = r#"{
            "abi": [],
            "metadata": {
                "output": {
                    "devdoc": {"methods": {}},
                    "userdoc": {"methods": {}}
                }
            }
        }"#;
        let ir = parse_artifact("NoNs", json).unwrap();
        assert!(ir.natspec.is_none());
    }

    // ── fallback handling ───────────────────────────────────────────────

    #[test]
    fn parses_fallback() {
        let json = r#"{
            "abi": [
                {"type": "fallback", "stateMutability": "nonpayable"},
                {"type": "receive", "stateMutability": "payable"}
            ]
        }"#;
        let ir = parse_artifact("FallbackTest", json).unwrap();
        assert!(ir.has_fallback);
        assert!(ir.has_receive);
    }

    // ── anonymous events ────────────────────────────────────────────────

    #[test]
    fn parses_anonymous_event() {
        let json = r#"{
            "abi": [{
                "type": "event",
                "name": "Debug",
                "inputs": [
                    {"name": "msg", "type": "string", "indexed": false, "internalType": "string", "components": []}
                ],
                "anonymous": true
            }]
        }"#;
        let ir = parse_artifact("AnonEvent", json).unwrap();
        assert_eq!(ir.events.len(), 1);
        assert!(ir.events[0].anonymous);
    }

    // ── state mutability variants ───────────────────────────────────────

    #[test]
    fn state_mutability_all_variants() {
        let json = r#"{
            "abi": [
                {"type": "function", "name": "pureF", "inputs": [], "outputs": [], "stateMutability": "pure"},
                {"type": "function", "name": "viewF", "inputs": [], "outputs": [], "stateMutability": "view"},
                {"type": "function", "name": "payF", "inputs": [], "outputs": [], "stateMutability": "payable"},
                {"type": "function", "name": "nonpayF", "inputs": [], "outputs": [], "stateMutability": "nonpayable"}
            ]
        }"#;
        let ir = parse_artifact("Mutability", json).unwrap();
        assert_eq!(ir.functions[0].state_mutability, StateMutability::Pure);
        assert_eq!(ir.functions[1].state_mutability, StateMutability::View);
        assert_eq!(ir.functions[2].state_mutability, StateMutability::Payable);
        assert_eq!(
            ir.functions[3].state_mutability,
            StateMutability::NonPayable
        );
    }

    #[test]
    fn missing_state_mutability_defaults_to_nonpayable() {
        let json = r#"{
            "abi": [{"type": "function", "name": "f", "inputs": [], "outputs": []}]
        }"#;
        let ir = parse_artifact("NoMut", json).unwrap();
        assert_eq!(
            ir.functions[0].state_mutability,
            StateMutability::NonPayable
        );
    }

    // ── error paths ────────────────────────────────────────────────────

    #[test]
    fn rejects_invalid_json() {
        assert!(parse_artifact("Bad", "not json").is_err());
    }

    #[test]
    fn rejects_json_without_abi() {
        assert!(parse_artifact("NoAbi", "{}").is_err());
    }

    #[test]
    fn empty_abi_produces_empty_ir() {
        let json = r#"{"abi": []}"#;
        let ir = parse_artifact("Empty", json).unwrap();
        assert!(ir.functions.is_empty());
        assert!(ir.events.is_empty());
        assert!(ir.errors.is_empty());
        assert!(ir.constructor.is_none());
    }

    #[test]
    fn tuple_with_no_components_errors() {
        assert!(parse_sol_type("tuple", &[]).is_err());
    }

    // ── bytesN edge cases ──────────────────────────────────────────────

    #[test]
    fn rejects_invalid_bytesn() {
        assert!(parse_sol_type("bytesfoo", &[]).is_err());
    }

    // ── canonical_type_str tuple array ──────────────────────────────────

    #[test]
    fn canonical_type_str_tuple_array() {
        let json = r#"{
            "abi": [{
                "type": "function",
                "name": "batchTransfer",
                "inputs": [{
                    "name": "transfers",
                    "type": "tuple[]",
                    "internalType": "struct Batch.Transfer[]",
                    "components": [
                        {"name": "to", "type": "address", "internalType": "address", "components": []},
                        {"name": "amount", "type": "uint256", "internalType": "uint256", "components": []}
                    ]
                }],
                "outputs": [],
                "stateMutability": "nonpayable"
            }],
            "metadata": {
                "output": {
                    "devdoc": {
                        "methods": {
                            "batchTransfer((address,uint256)[])": {
                                "params": {"transfers": "Array of transfers"}
                            }
                        }
                    },
                    "userdoc": {"methods": {}}
                }
            }
        }"#;
        let ir = parse_artifact("Batch", json).unwrap();
        let f = &ir.functions[0];
        let ns = f.natspec.as_ref().expect("should find natspec for tuple[]");
        assert_eq!(
            ns.params.get("transfers").map(|s| s.as_str()),
            Some("Array of transfers")
        );
    }

    // ── error NatSpec from devdoc ───────────────────────────────────────

    #[test]
    fn error_natspec_from_devdoc() {
        let json = r#"{
            "abi": [{
                "type": "error",
                "name": "InsufficientBalance",
                "inputs": [
                    {"name": "available", "type": "uint256", "internalType": "uint256", "components": []}
                ]
            }],
            "metadata": {
                "output": {
                    "devdoc": {
                        "errors": {
                            "InsufficientBalance(uint256)": {
                                "params": {"available": "The available balance"}
                            }
                        }
                    },
                    "userdoc": {"methods": {}}
                }
            }
        }"#;
        let ir = parse_artifact("ErrDoc", json).unwrap();
        let err = &ir.errors[0];
        let ns = err.natspec.as_ref().expect("should have error natspec");
        assert_eq!(
            ns.params.get("available").map(|s| s.as_str()),
            Some("The available balance")
        );
    }

    #[test]
    fn error_natspec_notice_from_userdoc() {
        let json = r#"{
            "abi": [{
                "type": "error",
                "name": "Unauthorized",
                "inputs": [
                    {"name": "account", "type": "address", "internalType": "address", "components": []}
                ]
            }],
            "metadata": {
                "output": {
                    "devdoc": {
                        "errors": {
                            "Unauthorized(address)": {
                                "details": "Raised when the caller is not allowed",
                                "params": {"account": "The unauthorized account"}
                            }
                        }
                    },
                    "userdoc": {
                        "errors": {
                            "Unauthorized(address)": {
                                "notice": "Caller is not authorized"
                            }
                        },
                        "methods": {}
                    }
                }
            }
        }"#;
        let ir = parse_artifact("ErrUserDoc", json).unwrap();
        let err = &ir.errors[0];
        let ns = err.natspec.as_ref().expect("should have error natspec");
        assert_eq!(ns.notice.as_deref(), Some("Caller is not authorized"));
        assert_eq!(
            ns.dev.as_deref(),
            Some("Raised when the caller is not allowed")
        );
        assert_eq!(
            ns.params.get("account").map(|s| s.as_str()),
            Some("The unauthorized account")
        );
    }

    // ── event NatSpec ───────────────────────────────────────────────────

    #[test]
    fn event_natspec_from_userdoc() {
        let json = r#"{
            "abi": [{
                "type": "event",
                "name": "Deposit",
                "inputs": [
                    {"name": "user", "type": "address", "indexed": true, "internalType": "address", "components": []}
                ],
                "anonymous": false
            }],
            "metadata": {
                "output": {
                    "devdoc": {"events": {}},
                    "userdoc": {
                        "events": {
                            "Deposit(address)": {"notice": "Emitted on deposit"}
                        },
                        "methods": {}
                    }
                }
            }
        }"#;
        let ir = parse_artifact("EvDoc", json).unwrap();
        let ev = &ir.events[0];
        let ns = ev.natspec.as_ref().expect("should have event natspec");
        assert_eq!(ns.notice.as_deref(), Some("Emitted on deposit"));
    }

    // ── parse_type_string public API ────────────────────────────────────

    #[test]
    fn parse_type_string_basic() {
        use crate::parser::parse_type_string;
        assert_eq!(parse_type_string("address").unwrap(), SolType::Address);
        assert_eq!(
            parse_type_string("uint256[]").unwrap(),
            SolType::Array(Box::new(SolType::Uint(256)))
        );
        assert!(parse_type_string("tuple").is_err()); // no components
    }

    // ── nested tuple ────────────────────────────────────────────────────

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn parses_nested_tuple() {
        let inner = vec![RawParam {
            name: "x".into(),
            ty: "uint256".into(),
            internal_type: None,
            components: vec![],
            indexed: false,
        }];
        let comps = vec![RawParam {
            name: "inner".into(),
            ty: "tuple".into(),
            internal_type: Some("struct Foo.Inner".into()),
            components: inner,
            indexed: false,
        }];
        let ty = parse_sol_type("tuple", &comps).unwrap();
        let SolType::Tuple(fields) = &ty else {
            unreachable!()
        };
        assert_eq!(fields.len(), 1);
        let SolType::Tuple(inner_fields) = &fields[0].ty else {
            unreachable!()
        };
        assert_eq!(inner_fields.len(), 1);
        assert_eq!(inner_fields[0].name, "x");
    }

    // ── unknown ABI item types ──────────────────────────────────────────

    #[test]
    fn unknown_abi_item_type_skipped_gracefully() {
        let json = r#"{
            "abi": [
                {"type": "function", "name": "ok", "inputs": [], "outputs": [], "stateMutability": "view"},
                {"type": "somethingNew", "name": "future"}
            ]
        }"#;
        let ir = parse_artifact("Skip", json).unwrap();
        // The function should be parsed, the unknown type skipped
        assert_eq!(ir.functions.len(), 1);
        assert_eq!(ir.functions[0].name, "ok");
    }

    // ── NatSpec edge: devdoc with no details/params ─────────────────────

    #[test]
    fn natspec_devdoc_only_returns_populated() {
        let json = r#"{
            "abi": [{
                "type": "function",
                "name": "foo",
                "inputs": [{"name": "x", "type": "uint256", "internalType": "uint256", "components": []}],
                "outputs": [],
                "stateMutability": "view"
            }],
            "metadata": {
                "output": {
                    "devdoc": {
                        "methods": {
                            "foo(uint256)": {
                                "details": "Internal helper",
                                "returns": {"_0": "nothing"}
                            }
                        }
                    },
                    "userdoc": {"methods": {}}
                }
            }
        }"#;
        let ir = parse_artifact("DevDoc", json).unwrap();
        let f = &ir.functions[0];
        let ns = f.natspec.as_ref().expect("should have devdoc");
        assert!(ns.notice.is_none());
        assert_eq!(ns.dev.as_deref(), Some("Internal helper"));
        assert_eq!(ns.returns.get("_0").map(|s| s.as_str()), Some("nothing"));
    }

    // ── NatSpec absent for a function ───────────────────────────────────

    #[test]
    fn natspec_absent_when_no_docs() {
        let json = r#"{
            "abi": [{
                "type": "function",
                "name": "undocumented",
                "inputs": [],
                "outputs": [],
                "stateMutability": "view"
            }],
            "metadata": {
                "output": {
                    "devdoc": {"methods": {}},
                    "userdoc": {"methods": {}}
                }
            }
        }"#;
        let ir = parse_artifact("NoDocs", json).unwrap();
        assert!(ir.functions[0].natspec.is_none());
    }
}
