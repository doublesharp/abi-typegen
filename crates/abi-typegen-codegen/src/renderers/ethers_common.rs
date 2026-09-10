use crate::type_mapper::overload_suffix;
use abi_typegen_core::types::{AbiFunction, SolType};
use std::collections::{HashMap, HashSet};

/// Allocates overload aliases without hiding real methods or another alias.
pub(super) fn method_names(functions: &[AbiFunction]) -> Vec<String> {
    let mut counts = HashMap::new();
    let mut used: HashSet<String> = functions.iter().map(|f| f.name.clone()).collect();
    for function in functions {
        *counts.entry(function.name.as_str()).or_insert(0) += 1;
    }
    functions
        .iter()
        .map(|function| {
            if counts[function.name.as_str()] == 1 {
                return function.name.clone();
            }
            let base = format!("{}{}", function.name, overload_suffix(&function.inputs));
            let mut candidate = base.clone();
            let mut suffix = 2;
            while !used.insert(candidate.clone()) {
                candidate = format!("{base}_{suffix}");
                suffix += 1;
            }
            candidate
        })
        .collect()
}

/// Canonical ABI signature used to select the actual ethers runtime method.
pub(super) fn function_signature(function: &AbiFunction) -> String {
    format!(
        "{}({})",
        function.name,
        function
            .inputs
            .iter()
            .map(|input| canonical_type(&input.ty))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn canonical_type(ty: &SolType) -> String {
    match ty {
        SolType::Uint(bits) => format!("uint{bits}"),
        SolType::Int(bits) => format!("int{bits}"),
        SolType::Bool => "bool".into(),
        SolType::Address => "address".into(),
        SolType::Bytes => "bytes".into(),
        SolType::BytesN(size) => format!("bytes{size}"),
        SolType::StringType => "string".into(),
        SolType::Array(inner) => format!("{}[]", canonical_type(inner)),
        SolType::FixedArray(inner, size) => format!("{}[{size}]", canonical_type(inner)),
        SolType::Tuple(fields) => format!(
            "({})",
            fields
                .iter()
                .map(|field| canonical_type(&field.ty))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}
