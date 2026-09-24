//! Identifier naming shared by the Go, Swift, and Kotlin renderers.
//!
//! These rules keep acronyms intact (`tokenURI` becomes `TokenURI`), name
//! unnamed parameters after their types, and number overloads the way the
//! target SDKs do.

use abi_typegen_core::types::{AbiFunction, SolType};
use std::collections::{HashMap, HashSet};

/// Converts an ABI name to an exported identifier.
///
/// Splits on `_`, uppercases the first character of each part, and keeps the
/// rest unchanged: `tokenURI` becomes `TokenURI`, `PREMIUM_PERIOD` becomes
/// `PREMIUMPERIOD`, and `_owner` becomes `Owner`. This matches abigen.
pub fn exported(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for part in name.split('_') {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.push_str(chars.as_str());
        }
    }
    out
}

/// Lowercases the first character of an identifier: `TokenURI` becomes `tokenURI`.
pub fn lower_first(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Returns the base name for an unnamed parameter of type `ty`.
///
/// Scalars use their Solidity spelling (`address`, `uint256`, `bytes32`).
/// Arrays append `Array`. Tuples use `tuple_name`, the name of the tuple at
/// the core of `ty`, in lower camel case.
pub fn unnamed_base(ty: &SolType, tuple_name: Option<&str>) -> String {
    match ty {
        SolType::Uint(bits) => format!("uint{bits}"),
        SolType::Int(bits) => format!("int{bits}"),
        SolType::Bool => "bool".to_string(),
        SolType::Address => "address".to_string(),
        SolType::Bytes => "bytes".to_string(),
        SolType::BytesN(size) => format!("bytes{size}"),
        SolType::StringType => "string".to_string(),
        SolType::Array(inner) | SolType::FixedArray(inner, _) => {
            format!("{}Array", unnamed_base(inner, tuple_name))
        }
        SolType::Tuple(_) => tuple_name
            .map(lower_first)
            .unwrap_or_else(|| "tuple".to_string()),
    }
}

/// Resolves parameter names, naming unnamed ones after their types.
///
/// Named parameters keep their ABI names. Unnamed parameters take
/// [`unnamed_base`], and repeats of a base get `2`, `3`, ... in order.
/// Any clash with another parameter gets the next free numeric suffix.
///
/// Each item is a parameter's ABI name, type, and the name of the tuple at
/// the core of its type, if any.
pub fn param_names<'a>(
    params: impl IntoIterator<Item = (&'a str, &'a SolType, Option<&'a str>)>,
) -> Vec<String> {
    let params: Vec<_> = params.into_iter().collect();
    let mut used: HashSet<String> = params
        .iter()
        .filter(|(name, _, _)| !name.is_empty())
        .map(|(name, _, _)| (*name).to_string())
        .collect();
    let mut seen: HashMap<String, usize> = HashMap::new();
    params
        .iter()
        .map(|(name, ty, tuple_name)| {
            if !name.is_empty() {
                return (*name).to_string();
            }
            let base = unnamed_base(ty, *tuple_name);
            let count = seen.entry(base.clone()).or_insert(0);
            *count += 1;
            let mut suffix = *count;
            let mut candidate = if suffix == 1 {
                base.clone()
            } else {
                format!("{base}{suffix}")
            };
            while used.contains(&candidate) {
                suffix += 1;
                candidate = format!("{base}{suffix}");
            }
            used.insert(candidate.clone());
            candidate
        })
        .collect()
}

/// Returns each function's position among overloads that share its name.
///
/// Functions with a unique name get `None`. Overloads get `Some(0)`,
/// `Some(1)`, ... in ABI declaration order.
pub fn overload_indices(functions: &[AbiFunction]) -> Vec<Option<usize>> {
    repeat_indices(functions.iter().map(|function| function.name.as_str()))
}

/// Numbers repeated names in order and leaves unique names unnumbered.
///
/// Used for overloaded functions, events, and errors alike.
pub fn repeat_indices<'a>(names: impl IntoIterator<Item = &'a str>) -> Vec<Option<usize>> {
    let names: Vec<&str> = names.into_iter().collect();
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for name in &names {
        *counts.entry(name).or_insert(0) += 1;
    }
    let mut next: HashMap<&str, usize> = HashMap::new();
    names
        .iter()
        .map(|name| {
            if counts[name] < 2 {
                return None;
            }
            let index = next.entry(name).or_insert(0);
            let current = *index;
            *index += 1;
            Some(current)
        })
        .collect()
}

/// Allocates identifiers that are unique within one scope.
///
/// The first request for a name gets it unchanged. Later requests for a taken
/// name get the lowest free numeric suffix starting at `2`.
#[derive(Debug, Default)]
pub struct Scope {
    used: HashSet<String>,
}

impl Scope {
    /// Creates a scope with some names already taken.
    pub fn with_reserved<'a>(names: impl IntoIterator<Item = &'a str>) -> Self {
        Self {
            used: names.into_iter().map(str::to_string).collect(),
        }
    }

    /// Returns `name`, or `name` plus a numeric suffix when it is taken.
    pub fn claim(&mut self, name: &str) -> String {
        if self.used.insert(name.to_string()) {
            return name.to_string();
        }
        let mut suffix = 2;
        loop {
            let candidate = format!("{name}{suffix}");
            if self.used.insert(candidate.clone()) {
                return candidate;
            }
            suffix += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use abi_typegen_core::types::{AbiParam, StateMutability};

    #[test]
    fn exported_keeps_acronyms() {
        assert_eq!(exported("tokenURI"), "TokenURI");
        assert_eq!(exported("DOMAIN_SEPARATOR"), "DOMAINSEPARATOR");
        assert_eq!(exported("PREMIUM_PERIOD"), "PREMIUMPERIOD");
        assert_eq!(exported("premiumPeriod"), "PremiumPeriod");
        assert_eq!(exported("_owner"), "Owner");
        assert_eq!(exported("balance_of"), "BalanceOf");
        assert_eq!(exported("ERC20"), "ERC20");
    }

    #[test]
    fn lower_first_keeps_rest() {
        assert_eq!(lower_first("TokenURI"), "tokenURI");
        assert_eq!(lower_first(""), "");
    }

    #[test]
    fn unnamed_params_use_type_names_and_number_repeats() {
        let types = [
            SolType::Address,
            SolType::Address,
            SolType::Uint(256),
            SolType::Array(Box::new(SolType::BytesN(32))),
        ];
        let names = param_names(types.iter().map(|ty| ("", ty, None)));
        assert_eq!(names, ["address", "address2", "uint256", "bytes32Array"]);
    }

    #[test]
    fn unnamed_params_avoid_named_ones() {
        let params = [("address", SolType::Address), ("", SolType::Address)];
        let names = param_names(params.iter().map(|(n, t)| (*n, t, None)));
        assert_eq!(names, ["address", "address2"]);
    }

    #[test]
    fn unnamed_tuple_uses_struct_name() {
        let tuple = SolType::Array(Box::new(SolType::Tuple(vec![])));
        let names = param_names([("", &tuple, Some("VaultPosition"))]);
        assert_eq!(names, ["vaultPositionArray"]);
    }

    #[test]
    fn overloads_number_in_declaration_order() {
        let function = |name: &str, inputs: usize| AbiFunction {
            name: name.to_string(),
            inputs: (0..inputs)
                .map(|_| AbiParam {
                    name: String::new(),
                    ty: SolType::Address,
                    internal_type: None,
                })
                .collect(),
            outputs: vec![],
            state_mutability: StateMutability::NonPayable,
            natspec: None,
        };
        let functions = [
            function("safeTransferFrom", 3),
            function("approve", 2),
            function("safeTransferFrom", 4),
        ];
        assert_eq!(overload_indices(&functions), [Some(0), None, Some(1)]);
    }

    #[test]
    fn scope_suffixes_taken_names() {
        let mut scope = Scope::with_reserved(["Token"]);
        assert_eq!(scope.claim("Token"), "Token2");
        assert_eq!(scope.claim("Other"), "Other");
        assert_eq!(scope.claim("Other"), "Other2");
    }
}
