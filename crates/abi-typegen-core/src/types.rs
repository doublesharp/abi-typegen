//! Intermediate representation types for Solidity ABIs.

use std::collections::HashMap;

/// A Solidity type parsed from an ABI descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolType {
    /// `uintN` where N is the bit width (8–256).
    Uint(u16),
    /// `intN` where N is the bit width (8–256).
    Int(u16),
    /// `bool`.
    Bool,
    /// `address`.
    Address,
    /// Dynamic `bytes`.
    Bytes,
    /// Fixed `bytesN` (1–32).
    BytesN(u8),
    /// `string`.
    StringType,
    /// Dynamic array `T[]`.
    Array(Box<SolType>),
    /// Fixed-length array `T[N]`.
    FixedArray(Box<SolType>, usize),
    /// Tuple (struct) with named components.
    Tuple(Vec<TupleComponent>),
}

/// A single field within a [`SolType::Tuple`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TupleComponent {
    /// Field name (may be empty for anonymous tuples).
    pub name: String,
    /// Solidity type of this field.
    pub ty: SolType,
    /// Compiler-provided internal type (e.g. `"struct Vault.Position"`).
    pub internal_type: Option<String>,
}

/// A function or error input/output parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbiParam {
    /// Parameter name (may be empty for unnamed params).
    pub name: String,
    /// Solidity type.
    pub ty: SolType,
    /// Compiler-provided internal type string.
    pub internal_type: Option<String>,
}

/// An event parameter, which may be indexed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbiEventParam {
    /// Parameter name.
    pub name: String,
    /// Solidity type.
    pub ty: SolType,
    /// Whether this parameter is an indexed topic.
    pub indexed: bool,
    /// Compiler-provided internal type string.
    pub internal_type: Option<String>,
}

/// EVM function state mutability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateMutability {
    /// Does not read or write state.
    Pure,
    /// Reads but does not write state.
    View,
    /// May write state, does not accept ETH.
    NonPayable,
    /// May write state and accepts ETH.
    Payable,
}

/// Extracted NatSpec documentation for a contract item.
#[derive(Debug, Clone, Default)]
pub struct NatSpec {
    /// `@notice` — user-facing description.
    pub notice: Option<String>,
    /// `@dev` — developer-facing description.
    pub dev: Option<String>,
    /// `@param` entries keyed by parameter name.
    pub params: HashMap<String, String>,
    /// `@return` entries keyed by return name.
    pub returns: HashMap<String, String>,
}

/// A parsed ABI function.
#[derive(Debug, Clone)]
pub struct AbiFunction {
    /// Function name.
    pub name: String,
    /// Input parameters.
    pub inputs: Vec<AbiParam>,
    /// Output parameters.
    pub outputs: Vec<AbiParam>,
    /// State mutability.
    pub state_mutability: StateMutability,
    /// Extracted NatSpec documentation.
    pub natspec: Option<NatSpec>,
}

/// A parsed ABI event.
#[derive(Debug, Clone)]
pub struct AbiEvent {
    /// Event name.
    pub name: String,
    /// Event parameters (may be indexed).
    pub inputs: Vec<AbiEventParam>,
    /// Whether this event is anonymous.
    pub anonymous: bool,
    /// Extracted NatSpec documentation.
    pub natspec: Option<NatSpec>,
}

/// A parsed ABI custom error.
#[derive(Debug, Clone)]
pub struct AbiError {
    /// Error name.
    pub name: String,
    /// Error parameters.
    pub inputs: Vec<AbiParam>,
    /// Extracted NatSpec documentation.
    pub natspec: Option<NatSpec>,
}

/// A parsed ABI constructor.
#[derive(Debug, Clone)]
pub struct AbiConstructor {
    /// Constructor parameters.
    pub inputs: Vec<AbiParam>,
    /// State mutability (typically `NonPayable` or `Payable`).
    pub state_mutability: StateMutability,
}

/// Fully parsed intermediate representation of a single contract's ABI.
#[derive(Debug, Clone)]
pub struct ContractIr {
    /// Contract name (e.g. `"ERC20"`).
    pub name: String,
    /// Constructor, if present.
    pub constructor: Option<AbiConstructor>,
    /// All functions in the ABI.
    pub functions: Vec<AbiFunction>,
    /// All events in the ABI.
    pub events: Vec<AbiEvent>,
    /// All custom errors in the ABI.
    pub errors: Vec<AbiError>,
    /// Whether the contract has a `fallback` function.
    pub has_fallback: bool,
    /// Whether the contract has a `receive` function.
    pub has_receive: bool,
    /// Contract-level NatSpec documentation.
    pub natspec: Option<NatSpec>,
    /// Raw ABI JSON value, preserved for `as const` serialization.
    pub raw_abi: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sol_type_eq() {
        assert_eq!(SolType::Uint(256), SolType::Uint(256));
        assert_ne!(SolType::Uint(256), SolType::Uint(128));
        assert_eq!(SolType::Bool, SolType::Bool);
        assert_eq!(
            SolType::Array(Box::new(SolType::Address)),
            SolType::Array(Box::new(SolType::Address))
        );
        assert_eq!(
            SolType::FixedArray(Box::new(SolType::Uint(256)), 3),
            SolType::FixedArray(Box::new(SolType::Uint(256)), 3)
        );
        assert_ne!(
            SolType::FixedArray(Box::new(SolType::Uint(256)), 3),
            SolType::FixedArray(Box::new(SolType::Uint(256)), 4)
        );
    }

    #[test]
    fn tuple_component_eq() {
        let c1 = TupleComponent {
            name: "to".into(),
            ty: SolType::Address,
            internal_type: None,
        };
        let c2 = c1.clone();
        assert_eq!(c1, c2);
    }
}
