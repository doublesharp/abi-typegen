//! Canonical ABI signatures, function selectors, and event topics.

use crate::types::{AbiError, AbiEvent, AbiFunction, SolType};
use alloy_primitives::{B256, FixedBytes, keccak256};

impl SolType {
    /// Returns the canonical ABI type string used in signatures.
    ///
    /// Tuples expand to their component types, e.g. `(uint256,address)[]`.
    pub fn canonical(&self) -> String {
        match self {
            SolType::Uint(bits) => format!("uint{bits}"),
            SolType::Int(bits) => format!("int{bits}"),
            SolType::Bool => "bool".to_string(),
            SolType::Address => "address".to_string(),
            SolType::Bytes => "bytes".to_string(),
            SolType::BytesN(size) => format!("bytes{size}"),
            SolType::StringType => "string".to_string(),
            SolType::Array(inner) => format!("{}[]", inner.canonical()),
            SolType::FixedArray(inner, size) => format!("{}[{size}]", inner.canonical()),
            SolType::Tuple(components) => format!(
                "({})",
                components
                    .iter()
                    .map(|component| component.ty.canonical())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }
}

fn signature<'a>(name: &str, types: impl IntoIterator<Item = &'a SolType>) -> String {
    let types = types
        .into_iter()
        .map(SolType::canonical)
        .collect::<Vec<_>>()
        .join(",");
    format!("{name}({types})")
}

fn selector(signature: &str) -> FixedBytes<4> {
    let hash = keccak256(signature.as_bytes());
    FixedBytes::from_slice(&hash[..4])
}

impl AbiFunction {
    /// Returns the canonical signature, e.g. `transfer(address,uint256)`.
    pub fn signature(&self) -> String {
        signature(&self.name, self.inputs.iter().map(|param| &param.ty))
    }

    /// Returns the 4-byte function selector.
    pub fn selector(&self) -> FixedBytes<4> {
        selector(&self.signature())
    }
}

impl AbiEvent {
    /// Returns the canonical signature, e.g. `Transfer(address,address,uint256)`.
    pub fn signature(&self) -> String {
        signature(&self.name, self.inputs.iter().map(|param| &param.ty))
    }

    /// Returns the keccak-256 hash of the signature.
    ///
    /// Non-anonymous events log this as topic 0. Anonymous events do not log
    /// it, so callers should not match logs against it.
    pub fn topic0(&self) -> B256 {
        keccak256(self.signature().as_bytes())
    }
}

impl AbiError {
    /// Returns the canonical signature, e.g. `Unauthorized(address)`.
    pub fn signature(&self) -> String {
        signature(&self.name, self.inputs.iter().map(|param| &param.ty))
    }

    /// Returns the 4-byte error selector.
    pub fn selector(&self) -> FixedBytes<4> {
        selector(&self.signature())
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{AbiEventParam, AbiParam, StateMutability, TupleComponent};

    use super::*;

    fn param(ty: SolType) -> AbiParam {
        AbiParam {
            name: String::new(),
            ty,
            internal_type: None,
        }
    }

    fn function(name: &str, inputs: Vec<SolType>) -> AbiFunction {
        AbiFunction {
            name: name.to_string(),
            inputs: inputs.into_iter().map(param).collect(),
            outputs: vec![],
            state_mutability: StateMutability::NonPayable,
            natspec: None,
        }
    }

    #[test]
    fn function_selector_matches_erc20_transfer() {
        let transfer = function("transfer", vec![SolType::Address, SolType::Uint(256)]);
        assert_eq!(transfer.signature(), "transfer(address,uint256)");
        assert_eq!(transfer.selector().to_string(), "0xa9059cbb");
    }

    #[test]
    fn event_topic_matches_erc20_transfer() {
        let event = AbiEvent {
            name: "Transfer".to_string(),
            inputs: [SolType::Address, SolType::Address, SolType::Uint(256)]
                .into_iter()
                .map(|ty| AbiEventParam {
                    name: String::new(),
                    ty,
                    indexed: false,
                    internal_type: None,
                })
                .collect(),
            anonymous: false,
            natspec: None,
        };
        assert_eq!(
            event.topic0().to_string(),
            "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
        );
    }

    #[test]
    fn error_selector_matches_solidity_error_string() {
        let error = AbiError {
            name: "Error".to_string(),
            inputs: vec![param(SolType::StringType)],
            natspec: None,
        };
        assert_eq!(error.selector().to_string(), "0x08c379a0");
    }

    #[test]
    fn tuple_arrays_expand_components() {
        let tuple = SolType::Tuple(vec![
            TupleComponent {
                name: "shares".to_string(),
                ty: SolType::Uint(256),
                internal_type: None,
            },
            TupleComponent {
                name: "tags".to_string(),
                ty: SolType::FixedArray(Box::new(SolType::BytesN(32)), 2),
                internal_type: None,
            },
        ]);
        let f = function(
            "deposit",
            vec![SolType::Array(Box::new(tuple)), SolType::Bool],
        );
        assert_eq!(f.signature(), "deposit((uint256,bytes32[2])[],bool)");
    }
}
