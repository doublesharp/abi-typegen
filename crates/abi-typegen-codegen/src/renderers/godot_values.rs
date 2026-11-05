//! Named GDScript values and lossless conversions around the existing dictionary codecs.

use super::{gd_string, names};
use crate::naming::{Scope, exported};
use crate::tuples::TupleRegistry;
use abi_typegen_core::types::{AbiParam, ContractIr, SolType};
use std::collections::HashMap;

fn class_stem(name: &str) -> String {
    let clean: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    let stem = exported(&clean);
    if stem.is_empty() {
        "Value".into()
    } else {
        stem
    }
}

// Object and RefCounted members from the pinned Godot 4.7 extension API.
const MEMBERS: &[&str] = &[
    "add_user_signal",
    "can_translate_messages",
    "cancel_free",
    "get_incoming_connections",
    "get_meta_list",
    "get_method_argument_count",
    "get_signal_connection_list",
    "get_translation_domain",
    "has_connections",
    "has_user_signal",
    "is_blocking_signals",
    "is_queued_for_deletion",
    "notify_property_list_changed",
    "property_can_revert",
    "property_get_revert",
    "property_list_changed",
    "remove_meta",
    "remove_user_signal",
    "script_changed",
    "set_block_signals",
    "set_message_translation",
    "set_translation_domain",
    "tr",
    "tr_n",
    "bool",
    "int",
    "float",
    "from_dictionary",
    "from_abi_value",
    "to_dictionary",
    "new",
    "get",
    "set",
    "free",
    "get_class",
    "is_class",
    "get_instance_id",
    "get_script",
    "set_script",
    "get_meta",
    "set_meta",
    "has_meta",
    "call",
    "callv",
    "call_deferred",
    "connect",
    "disconnect",
    "emit_signal",
    "reference",
    "unreference",
    "get_reference_count",
    "init_ref",
    "_init",
    "notification",
    "property_list_changed_notify",
    "get_property_list",
    "get_method_list",
    "get_signal_list",
    "has_method",
    "has_signal",
    "is_connected",
    "set_deferred",
    "get_indexed",
    "set_indexed",
    "to_string",
    "_to_string",
    "_get",
    "_set",
    "_get_property_list",
    "script",
];

/// Names assigned once so colliding tuple and operation types remain distinct.
pub(super) struct ValueTypes {
    tuples: HashMap<String, String>,
    /// One named return class for each function in ABI order.
    pub(super) results: Vec<String>,
    /// One named payload class for each event in ABI order.
    pub(super) events: Vec<String>,
    /// One named payload class for each custom error in ABI order.
    pub(super) errors: Vec<String>,
}

impl ValueTypes {
    /// Emit standalone values even when runtime wrappers are disabled.
    pub(super) fn render(out: &mut String, ir: &ContractIr, registry: &TupleRegistry) -> Self {
        let namespace = super::namespace_name(&ir.name);
        let mut scope = Scope::with_reserved([namespace.as_str()]);
        let tuples = registry
            .defs()
            .iter()
            .map(|def| {
                (
                    def.name.clone(),
                    scope.claim(&format!("Atg{}", class_stem(&def.name))),
                )
            })
            .collect();
        let mut types = Self {
            tuples,
            results: Vec::new(),
            events: Vec::new(),
            errors: Vec::new(),
        };
        for function in &ir.functions {
            types
                .results
                .push(scope.claim(&format!("Atg{}Result", class_stem(&function.name))));
        }
        for event in &ir.events {
            types
                .events
                .push(scope.claim(&format!("Atg{}Event", class_stem(&event.name))));
        }
        for error in &ir.errors {
            types
                .errors
                .push(scope.claim(&format!("Atg{}Error", class_stem(&error.name))));
        }
        for def in registry.defs() {
            let params: Vec<_> = def
                .components
                .iter()
                .map(|p| AbiParam {
                    name: p.name.clone(),
                    ty: p.ty.clone(),
                    internal_type: p.internal_type.clone(),
                })
                .collect();
            types.render_class(out, &types.tuples[&def.name], &params, registry);
        }
        for (function, name) in ir.functions.iter().zip(&types.results) {
            types.render_class(out, name, &function.outputs, registry);
        }
        for (event, name) in ir.events.iter().zip(&types.events) {
            let params: Vec<_> = event
                .inputs
                .iter()
                .map(|p| AbiParam {
                    name: p.name.clone(),
                    ty: if p.indexed && super::indexed_hash(&p.ty) {
                        SolType::BytesN(32)
                    } else {
                        p.ty.clone()
                    },
                    internal_type: p.internal_type.clone(),
                })
                .collect();
            types.render_class(out, name, &params, registry);
        }
        for (error, name) in ir.errors.iter().zip(&types.errors) {
            types.render_class(out, name, &error.inputs, registry);
        }
        types
    }

    fn type_name(&self, ty: &SolType, internal: Option<&str>, registry: &TupleRegistry) -> String {
        match ty {
            SolType::Bool => "bool".into(),
            SolType::Address | SolType::StringType => "String".into(),
            SolType::Uint(_) | SolType::Int(_) | SolType::Bytes | SolType::BytesN(_) => {
                "PackedByteArray".into()
            }
            SolType::Array(_) | SolType::FixedArray(_, _) => "Array".into(),
            SolType::Tuple(parts) => self.tuples[registry.name(parts, internal)].clone(),
        }
    }

    fn convert(
        &self,
        ty: &SolType,
        internal: Option<&str>,
        expression: &str,
        registry: &TupleRegistry,
        to_dictionary: bool,
        depth: usize,
    ) -> String {
        match ty {
            SolType::Tuple(parts) => {
                if to_dictionary {
                    format!("{expression}.to_dictionary()")
                } else {
                    format!(
                        "{}.from_dictionary({expression})",
                        self.tuples[registry.name(parts, internal)]
                    )
                }
            }
            SolType::Array(inner) | SolType::FixedArray(inner, _) => {
                let item = format!("_item{depth}");
                let converted =
                    self.convert(inner, internal, &item, registry, to_dictionary, depth + 1);
                format!("{expression}.map(func({item}): return {converted})")
            }
            SolType::Bool
            | SolType::Address
            | SolType::StringType
            | SolType::Uint(_)
            | SolType::Int(_)
            | SolType::Bytes
            | SolType::BytesN(_) => expression.into(),
        }
    }

    fn render_class(
        &self,
        out: &mut String,
        name: &str,
        params: &[AbiParam],
        registry: &TupleRegistry,
    ) {
        let raw_names = names(params, registry);
        let reserved: Vec<_> = MEMBERS
            .iter()
            .copied()
            .chain(raw_names.iter().map(String::as_str))
            .collect();
        let mut field_scope = Scope::with_reserved(reserved);
        let fields: Vec<_> = raw_names
            .iter()
            .map(|raw| {
                if MEMBERS.contains(&raw.as_str()) {
                    field_scope.claim(raw)
                } else {
                    raw.clone()
                }
            })
            .collect();
        out.push_str(&format!("\n## Named ABI value. Integer fields retain their exact 32-byte ABI words.\nclass {name}:\n    extends RefCounted\n"));
        for (param, field) in params.iter().zip(&fields) {
            let default = match &param.ty {
                SolType::Bool => "false".into(),
                SolType::Address | SolType::StringType => "\"\"".into(),
                SolType::Array(_) | SolType::FixedArray(_, _) => "[]".into(),
                SolType::Tuple(_) => format!(
                    "{}.new()",
                    self.type_name(&param.ty, param.internal_type.as_deref(), registry)
                ),
                SolType::Uint(_) | SolType::Int(_) => {
                    format!("PackedByteArray([{}])", vec!["0"; 32].join(","))
                }
                SolType::BytesN(n) => format!(
                    "PackedByteArray([{}])",
                    vec!["0"; usize::from(*n)].join(",")
                ),
                SolType::Bytes => "PackedByteArray()".into(),
            };
            out.push_str(&format!(
                "    var {field}: {} = {default}\n",
                self.type_name(&param.ty, param.internal_type.as_deref(), registry)
            ));
        }
        out.push_str(&format!("\n    ## Convert a dictionary returned by the validated ABI codec.\n    static func from_dictionary(data: Dictionary) -> {name}:\n        var value := {name}.new()\n"));
        if params.is_empty() {
            out.push_str("        # No ABI fields.\n");
        }
        for ((param, field), raw) in params.iter().zip(&fields).zip(&raw_names) {
            out.push_str(&format!(
                "        value.{field} = {}\n",
                self.convert(
                    &param.ty,
                    param.internal_type.as_deref(),
                    &format!("data[{}]", gd_string(raw)),
                    registry,
                    false,
                    0
                )
            ));
        }
        out.push_str("        return value\n\n    ## Convert this value back to the existing ABI dictionary representation.\n    func to_dictionary() -> Dictionary:\n        return {\n");
        for ((param, field), raw) in params.iter().zip(&fields).zip(&raw_names) {
            out.push_str(&format!(
                "            {}: {},\n",
                gd_string(raw),
                self.convert(
                    &param.ty,
                    param.internal_type.as_deref(),
                    field,
                    registry,
                    true,
                    0
                )
            ));
        }
        out.push_str("        }\n");
        out.push_str(&format!("\n    ## Convert the raw codec payload, which collapses zero or one ABI fields.\n    static func from_abi_value(data: Variant) -> {name}:\n"));
        match raw_names.as_slice() {
            [] => out.push_str(&format!("        return {name}.new()\n")),
            [raw] => out.push_str(&format!(
                "        return from_dictionary({{{}: data}})\n",
                gd_string(raw)
            )),
            _ => out.push_str("        return from_dictionary(data)\n"),
        }
    }
}
