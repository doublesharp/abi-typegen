//! Named types for the tuples in a contract ABI.
//!
//! Go, Swift, and Kotlin render every tuple as a named type. The registry
//! walks a contract once, assigns each distinct tuple a name, and orders the
//! definitions so nested tuples come before the tuples that contain them.

use crate::naming::exported;
use abi_typegen_core::types::{ContractIr, SolType, TupleComponent};

/// One named tuple type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TupleDef {
    /// Exported type name, without any contract prefix.
    pub name: String,
    /// Tuple fields in ABI order.
    pub components: Vec<TupleComponent>,
    source: Option<String>,
}

/// All named tuple types in one contract, in dependency order.
#[derive(Debug, Clone, Default)]
pub struct TupleRegistry {
    defs: Vec<TupleDef>,
}

/// Returns the struct name encoded in an `internalType`, with dots removed.
///
/// `struct Vault.Position[]` becomes `VaultPosition`. Returns `None` when the
/// internal type does not name a struct.
pub fn internal_struct_name(internal_type: &str) -> Option<String> {
    let mut stripped = internal_type.strip_prefix("struct ")?;
    while stripped.ends_with(']') {
        let open = stripped.rfind('[')?;
        stripped = &stripped[..open];
    }
    let name: String = stripped.split('.').map(exported).collect();
    (!name.is_empty()).then_some(name)
}

impl TupleRegistry {
    /// Collects every tuple in the contract's functions, events, errors, and constructor.
    pub fn new(ir: &ContractIr) -> Self {
        let mut registry = Self::default();
        if let Some(constructor) = &ir.constructor {
            for (index, input) in constructor.inputs.iter().enumerate() {
                registry.visit(
                    &input.ty,
                    input.internal_type.as_deref(),
                    &context("Constructor", &input.name, index),
                );
            }
        }
        for function in &ir.functions {
            let owner = exported(&function.name);
            for (index, input) in function.inputs.iter().enumerate() {
                registry.visit(
                    &input.ty,
                    input.internal_type.as_deref(),
                    &context(&owner, &input.name, index),
                );
            }
            for (index, output) in function.outputs.iter().enumerate() {
                registry.visit(
                    &output.ty,
                    output.internal_type.as_deref(),
                    &context(&format!("{owner}Return"), &output.name, index),
                );
            }
        }
        for event in &ir.events {
            let owner = format!("{}Event", exported(&event.name));
            for (index, input) in event.inputs.iter().enumerate() {
                registry.visit(
                    &input.ty,
                    input.internal_type.as_deref(),
                    &context(&owner, &input.name, index),
                );
            }
        }
        for error in &ir.errors {
            let owner = format!("{}Error", exported(&error.name));
            for (index, input) in error.inputs.iter().enumerate() {
                registry.visit(
                    &input.ty,
                    input.internal_type.as_deref(),
                    &context(&owner, &input.name, index),
                );
            }
        }
        registry
    }

    /// Returns the tuple definitions, nested tuples first.
    pub fn defs(&self) -> &[TupleDef] {
        &self.defs
    }

    /// Returns the type name for a tuple occurrence.
    ///
    /// `internal_type` is the occurrence's own internal type (possibly with
    /// array suffixes).
    ///
    /// # Panics
    ///
    /// Panics if the tuple was not part of the contract the registry was built
    /// from. Renderers only look up tuples from that same contract.
    pub fn name(&self, components: &[TupleComponent], internal_type: Option<&str>) -> &str {
        let source = internal_type.and_then(internal_struct_name);
        &self
            .defs
            .iter()
            .find(|def| def.source == source && def.components == components)
            .expect("tuple registered from the same contract")
            .name
    }

    /// Returns the name of the tuple at the core of `ty`, looking through arrays.
    pub fn name_of_type(&self, ty: &SolType, internal_type: Option<&str>) -> Option<&str> {
        match ty {
            SolType::Tuple(components) => Some(self.name(components, internal_type)),
            SolType::Array(inner) | SolType::FixedArray(inner, _) => {
                self.name_of_type(inner, internal_type)
            }
            SolType::Uint(_)
            | SolType::Int(_)
            | SolType::Bool
            | SolType::Address
            | SolType::Bytes
            | SolType::BytesN(_)
            | SolType::StringType => None,
        }
    }

    fn visit(&mut self, ty: &SolType, internal_type: Option<&str>, fallback: &str) {
        match ty {
            SolType::Tuple(components) => {
                let source = internal_type.and_then(internal_struct_name);
                if self
                    .defs
                    .iter()
                    .any(|def| def.source == source && def.components == *components)
                {
                    return;
                }
                let base = source.clone().unwrap_or_else(|| fallback.to_string());
                let mut name = base.clone();
                let mut suffix = 2;
                while self.defs.iter().any(|def| def.name == name) {
                    name = format!("{base}{suffix}");
                    suffix += 1;
                }
                for (index, component) in components.iter().enumerate() {
                    self.visit(
                        &component.ty,
                        component.internal_type.as_deref(),
                        &context(&name, &component.name, index),
                    );
                }
                // Nested tuples can claim `name` while being visited.
                while self.defs.iter().any(|def| def.name == name) {
                    name = format!("{base}{suffix}");
                    suffix += 1;
                }
                self.defs.push(TupleDef {
                    name,
                    components: components.clone(),
                    source,
                });
            }
            SolType::Array(inner) | SolType::FixedArray(inner, _) => {
                self.visit(inner, internal_type, fallback);
            }
            SolType::Uint(_)
            | SolType::Int(_)
            | SolType::Bool
            | SolType::Address
            | SolType::Bytes
            | SolType::BytesN(_)
            | SolType::StringType => {}
        }
    }
}

fn context(owner: &str, name: &str, index: usize) -> String {
    if name.is_empty() {
        format!("{owner}Param{index}")
    } else {
        format!("{owner}{}", exported(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use abi_typegen_core::parser::parse_artifact;

    fn registry(abi: &str) -> TupleRegistry {
        let ir = parse_artifact("Test", &format!(r#"{{"abi":{abi}}}"#)).expect("valid abi");
        TupleRegistry::new(&ir)
    }

    fn names(registry: &TupleRegistry) -> Vec<&str> {
        registry
            .defs()
            .iter()
            .map(|def| def.name.as_str())
            .collect()
    }

    #[test]
    fn internal_type_names_drop_dots_and_arrays() {
        assert_eq!(
            internal_struct_name("struct Vault.Position[][2]").as_deref(),
            Some("VaultPosition")
        );
        assert_eq!(
            internal_struct_name("struct URIData").as_deref(),
            Some("URIData")
        );
        assert_eq!(internal_struct_name("address"), None);
    }

    #[test]
    fn same_struct_name_in_different_libraries_stays_distinct() {
        let reg = registry(
            r#"[
            {"type":"function","name":"deposit","inputs":[{"name":"p","type":"tuple","internalType":"struct A.Position","components":[{"name":"amount","type":"uint256"}]}],"outputs":[],"stateMutability":"nonpayable"},
            {"type":"function","name":"deposit","inputs":[{"name":"p","type":"tuple","internalType":"struct B.Position","components":[{"name":"account","type":"address"}]}],"outputs":[],"stateMutability":"nonpayable"}
        ]"#,
        );
        assert_eq!(names(&reg), ["APosition", "BPosition"]);
    }

    #[test]
    fn identical_anonymous_tuples_share_one_type() {
        let reg = registry(
            r#"[
            {"type":"function","name":"a","inputs":[{"name":"p","type":"tuple","components":[{"name":"x","type":"uint256"}]}],"outputs":[],"stateMutability":"nonpayable"},
            {"type":"function","name":"b","inputs":[{"name":"q","type":"tuple[]","components":[{"name":"x","type":"uint256"}]}],"outputs":[],"stateMutability":"nonpayable"}
        ]"#,
        );
        assert_eq!(names(&reg), ["AP"]);
    }

    #[test]
    fn same_name_different_shape_gets_suffix() {
        let reg = registry(
            r#"[
            {"type":"function","name":"a","inputs":[{"name":"p","type":"tuple","internalType":"struct Pos","components":[{"name":"x","type":"uint256"}]}],"outputs":[],"stateMutability":"nonpayable"},
            {"type":"function","name":"b","inputs":[{"name":"p","type":"tuple","internalType":"struct Pos","components":[{"name":"y","type":"address"}]}],"outputs":[],"stateMutability":"nonpayable"}
        ]"#,
        );
        assert_eq!(names(&reg), ["Pos", "Pos2"]);
    }

    #[test]
    fn nested_tuples_come_first_and_resolve() {
        let reg = registry(
            r#"[
            {"type":"function","name":"f","inputs":[{"name":"outer","type":"tuple","internalType":"struct V.Outer","components":[
                {"name":"inner","type":"tuple[]","internalType":"struct V.Inner[]","components":[{"name":"x","type":"uint8"}]}
            ]}],"outputs":[],"stateMutability":"nonpayable"}
        ]"#,
        );
        assert_eq!(names(&reg), ["VInner", "VOuter"]);
        let inner = &reg.defs()[1].components[0];
        assert_eq!(
            reg.name_of_type(&inner.ty, inner.internal_type.as_deref()),
            Some("VInner")
        );
    }

    #[test]
    fn unnamed_output_tuple_uses_positional_context() {
        let reg = registry(
            r#"[
            {"type":"function","name":"get","inputs":[],"outputs":[{"name":"","type":"tuple","components":[{"name":"x","type":"bool"}]}],"stateMutability":"view"}
        ]"#,
        );
        assert_eq!(names(&reg), ["GetReturnParam0"]);
    }
}
