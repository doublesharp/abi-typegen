use abi_typegen_core::types::{AbiParam, SolType};

/// TypeScript output target — determines which type mappings apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// viem/wagmi target — uses `bigint` for large integers, template literals for addresses and bytes.
    Viem,
    /// ethers target — uses `bigint` for large integers, `string` for addresses and bytes.
    Ethers,
    /// web3.js target — uses `string` for all numeric types.
    Web3,
}

/// Hard reserved words in JavaScript/TypeScript that cannot be used as parameter names.
///
/// Contextual keywords like `from`, `get`, `set`, `of`, `as`, `type` are intentionally
/// excluded — they are valid as function parameter names and object property names.
const RESERVED_WORDS: &[&str] = &[
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "new",
    "null",
    "return",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "yield",
    // Strict mode reserved
    "implements",
    "interface",
    "let",
    "package",
    "private",
    "protected",
    "public",
    "static",
];

/// Returns `true` if `name` is a JS/TS reserved word that needs quoting.
pub fn is_reserved_word(name: &str) -> bool {
    RESERVED_WORDS.contains(&name)
}

/// Sanitizes a parameter name for TypeScript output.
///
/// - Empty names get a positional fallback (`arg0`, `arg1`, ...).
/// - Reserved words are prefixed with an underscore.
pub fn safe_param_name(name: &str, index: usize) -> String {
    if name.is_empty() {
        return format!("arg{}", index);
    }
    if is_reserved_word(name) {
        return format!("_{}", name);
    }
    name.to_string()
}

/// Returns the TypeScript type string for a Solidity type.
pub fn sol_type_to_ts(ty: &SolType, target: Target) -> String {
    match ty {
        SolType::Bool => "boolean".into(),
        SolType::StringType => "string".into(),

        SolType::Uint(bits) | SolType::Int(bits) => match target {
            Target::Web3 => "string".into(),
            _ => {
                if *bits <= 48 {
                    "number".into()
                } else {
                    "bigint".into()
                }
            }
        },

        SolType::Address => match target {
            Target::Viem => "`0x${string}`".into(),
            Target::Ethers | Target::Web3 => "string".into(),
        },

        SolType::Bytes | SolType::BytesN(_) => match target {
            Target::Viem => "`0x${string}`".into(),
            Target::Ethers | Target::Web3 => "string".into(),
        },

        SolType::Array(inner) => {
            let inner_ts = sol_type_to_ts(inner, target);
            match target {
                Target::Viem => format!("readonly {}[]", inner_ts),
                Target::Ethers | Target::Web3 => format!("{}[]", inner_ts),
            }
        }

        SolType::FixedArray(inner, n) => {
            let inner_ts = sol_type_to_ts(inner, target);
            match target {
                Target::Viem => {
                    let items = std::iter::repeat_n(inner_ts.as_str(), *n)
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("readonly [{}]", items)
                }
                Target::Ethers | Target::Web3 => format!("{}[]", inner_ts),
            }
        }

        SolType::Tuple(components) => {
            let fields: Vec<String> = components
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    format!(
                        "{}: {}",
                        safe_param_name(&c.name, i),
                        sol_type_to_ts(&c.ty, target)
                    )
                })
                .collect();
            format!("{{ {} }}", fields.join("; "))
        }
    }
}

/// Returns a human-readable struct name derived from internalType, or None.
/// e.g. "struct Vault.Position" → Some("Position")
pub fn struct_name_from_internal_type(internal_type: &str) -> Option<&str> {
    let mut stripped = internal_type.strip_prefix("struct ")?;
    while stripped.ends_with(']') {
        let open = stripped.rfind('[')?;
        stripped = &stripped[..open];
    }
    // Take the part after the last dot, if any
    Some(stripped.rsplit('.').next().unwrap_or(stripped))
}

/// Short human-readable name for a Solidity type, used in overload disambiguation.
fn sol_type_short_name(ty: &SolType) -> String {
    match ty {
        SolType::Uint(bits) => format!("Uint{}", bits),
        SolType::Int(bits) => format!("Int{}", bits),
        SolType::Bool => "Bool".into(),
        SolType::Address => "Address".into(),
        SolType::Bytes => "Bytes".into(),
        SolType::BytesN(n) => format!("Bytes{}", n),
        SolType::StringType => "String".into(),
        SolType::Array(inner) => format!("{}Array", sol_type_short_name(inner)),
        SolType::FixedArray(inner, n) => format!("{}Array{}", sol_type_short_name(inner), n),
        SolType::Tuple(_) => "Tuple".into(),
    }
}

/// Generates a disambiguating suffix for an overloaded function based on its parameter types.
///
/// Returns an UpperCamelCase suffix derived from the short names of each input type.
/// For example, `deposit(uint256)` produces `"Uint256"` and
/// `deposit(uint256, address)` produces `"Uint256Address"`.
///
/// When a function has no parameters the suffix is empty.
pub fn overload_suffix_from_sol_types<'a>(types: impl IntoIterator<Item = &'a SolType>) -> String {
    let mut suffix = String::new();
    for ty in types {
        suffix.push_str(&sol_type_short_name(ty));
    }
    suffix
}

/// Generates a disambiguating suffix for an overloaded function based on its parameter types.
///
/// Returns an UpperCamelCase suffix derived from the short names of each input type.
/// For example, `deposit(uint256)` produces `"Uint256"` and
/// `deposit(uint256, address)` produces `"Uint256Address"`.
///
/// When a function has no parameters the suffix is empty.
pub fn overload_suffix(inputs: &[AbiParam]) -> String {
    overload_suffix_from_sol_types(inputs.iter().map(|param| &param.ty))
}

#[cfg(test)]
mod tests {
    use super::*;
    use abi_typegen_core::types::{AbiParam, SolType, TupleComponent};

    #[test]
    fn bool_is_boolean() {
        assert_eq!(sol_type_to_ts(&SolType::Bool, Target::Viem), "boolean");
        assert_eq!(sol_type_to_ts(&SolType::Bool, Target::Ethers), "boolean");
    }

    #[test]
    fn small_uint_is_number() {
        assert_eq!(sol_type_to_ts(&SolType::Uint(8), Target::Viem), "number");
        assert_eq!(sol_type_to_ts(&SolType::Uint(48), Target::Viem), "number");
    }

    #[test]
    fn large_uint_is_bigint() {
        assert_eq!(sol_type_to_ts(&SolType::Uint(256), Target::Viem), "bigint");
        assert_eq!(sol_type_to_ts(&SolType::Uint(64), Target::Viem), "bigint");
        assert_eq!(sol_type_to_ts(&SolType::Int(128), Target::Ethers), "bigint");
    }

    #[test]
    fn address_differs_by_target() {
        assert_eq!(
            sol_type_to_ts(&SolType::Address, Target::Viem),
            "`0x${string}`"
        );
        assert_eq!(sol_type_to_ts(&SolType::Address, Target::Ethers), "string");
    }

    #[test]
    fn bytes_differs_by_target() {
        assert_eq!(
            sol_type_to_ts(&SolType::Bytes, Target::Viem),
            "`0x${string}`"
        );
        assert_eq!(
            sol_type_to_ts(&SolType::BytesN(32), Target::Viem),
            "`0x${string}`"
        );
        assert_eq!(sol_type_to_ts(&SolType::Bytes, Target::Ethers), "string");
    }

    #[test]
    fn dynamic_array_viem_readonly() {
        let ty = SolType::Array(Box::new(SolType::Address));
        assert_eq!(
            sol_type_to_ts(&ty, Target::Viem),
            "readonly `0x${string}`[]"
        );
        assert_eq!(sol_type_to_ts(&ty, Target::Ethers), "string[]");
    }

    #[test]
    fn fixed_array_viem_tuple_type() {
        let ty = SolType::FixedArray(Box::new(SolType::Uint(256)), 3);
        assert_eq!(
            sol_type_to_ts(&ty, Target::Viem),
            "readonly [bigint, bigint, bigint]"
        );
        assert_eq!(sol_type_to_ts(&ty, Target::Ethers), "bigint[]");
    }

    #[test]
    fn tuple_becomes_object_type() {
        let ty = SolType::Tuple(vec![
            TupleComponent {
                name: "shares".into(),
                ty: SolType::Uint(256),
                internal_type: None,
            },
            TupleComponent {
                name: "token".into(),
                ty: SolType::Address,
                internal_type: None,
            },
        ]);
        assert_eq!(
            sol_type_to_ts(&ty, Target::Viem),
            "{ shares: bigint; token: `0x${string}` }"
        );
    }

    #[test]
    fn struct_name_extraction() {
        assert_eq!(
            struct_name_from_internal_type("struct Vault.Position"),
            Some("Position")
        );
        assert_eq!(
            struct_name_from_internal_type("struct Position"),
            Some("Position")
        );
        assert_eq!(struct_name_from_internal_type("address"), None);
    }

    #[test]
    fn struct_name_extraction_strips_array_suffixes() {
        assert_eq!(
            struct_name_from_internal_type("struct Vault.Position[]"),
            Some("Position")
        );
        assert_eq!(
            struct_name_from_internal_type("struct Vault.Position[2][]"),
            Some("Position")
        );
    }

    #[test]
    fn nested_tuple_in_array() {
        let ty = SolType::Array(Box::new(SolType::Tuple(vec![TupleComponent {
            name: "x".into(),
            ty: SolType::Uint(256),
            internal_type: None,
        }])));
        let result = sol_type_to_ts(&ty, Target::Viem);
        assert_eq!(result, "readonly { x: bigint }[]");
        let result_ethers = sol_type_to_ts(&ty, Target::Ethers);
        assert_eq!(result_ethers, "{ x: bigint }[]");
    }

    #[test]
    fn empty_tuple() {
        let ty = SolType::Tuple(vec![]);
        assert_eq!(sol_type_to_ts(&ty, Target::Viem), "{  }");
    }

    // ── safe_param_name tests ───────────────────────────────────────────

    #[test]
    fn empty_name_gets_positional_fallback() {
        assert_eq!(safe_param_name("", 0), "arg0");
        assert_eq!(safe_param_name("", 3), "arg3");
    }

    #[test]
    fn normal_name_unchanged() {
        assert_eq!(safe_param_name("amount", 0), "amount");
        assert_eq!(safe_param_name("to", 1), "to");
    }

    #[test]
    fn reserved_word_gets_underscore_prefix() {
        assert_eq!(safe_param_name("class", 0), "_class");
        assert_eq!(safe_param_name("delete", 1), "_delete");
        assert_eq!(safe_param_name("function", 2), "_function");
        assert_eq!(safe_param_name("interface", 0), "_interface");
        assert_eq!(safe_param_name("return", 0), "_return");
    }

    #[test]
    fn contextual_keywords_not_reserved() {
        // These are valid TypeScript parameter names
        assert_eq!(safe_param_name("from", 0), "from");
        assert_eq!(safe_param_name("type", 0), "type");
        assert_eq!(safe_param_name("get", 0), "get");
        assert_eq!(safe_param_name("set", 0), "set");
    }

    #[test]
    fn is_reserved_word_check() {
        assert!(is_reserved_word("class"));
        assert!(is_reserved_word("return"));
        assert!(!is_reserved_word("amount"));
        assert!(!is_reserved_word("balance"));
    }

    #[test]
    fn tuple_unnamed_fields_get_positional_names() {
        let ty = SolType::Tuple(vec![
            TupleComponent {
                name: "".into(),
                ty: SolType::Uint(256),
                internal_type: None,
            },
            TupleComponent {
                name: "".into(),
                ty: SolType::Address,
                internal_type: None,
            },
        ]);
        let result = sol_type_to_ts(&ty, Target::Viem);
        assert_eq!(result, "{ arg0: bigint; arg1: `0x${string}` }");
    }

    // ── Additional coverage ────────────────────────────────────────────

    #[test]
    fn small_int_is_number_both_targets() {
        assert_eq!(sol_type_to_ts(&SolType::Int(8), Target::Viem), "number");
        assert_eq!(sol_type_to_ts(&SolType::Int(8), Target::Ethers), "number");
    }

    #[test]
    fn bytesn_ethers_is_string() {
        assert_eq!(
            sol_type_to_ts(&SolType::BytesN(32), Target::Ethers),
            "string"
        );
        assert_eq!(
            sol_type_to_ts(&SolType::BytesN(1), Target::Ethers),
            "string"
        );
    }

    #[test]
    fn struct_name_deeply_nested_dots() {
        assert_eq!(struct_name_from_internal_type("struct A.B.C"), Some("C"));
    }

    #[test]
    fn fixed_array_size_one_viem() {
        let ty = SolType::FixedArray(Box::new(SolType::Bool), 1);
        assert_eq!(sol_type_to_ts(&ty, Target::Viem), "readonly [boolean]");
    }

    #[test]
    fn fixed_array_size_zero_viem() {
        let ty = SolType::FixedArray(Box::new(SolType::Bool), 0);
        assert_eq!(sol_type_to_ts(&ty, Target::Viem), "readonly []");
    }

    #[test]
    fn string_type_both_targets() {
        assert_eq!(sol_type_to_ts(&SolType::StringType, Target::Viem), "string");
        assert_eq!(
            sol_type_to_ts(&SolType::StringType, Target::Ethers),
            "string"
        );
    }

    #[test]
    fn int_48_bit_boundary() {
        // 48-bit should be "number"
        assert_eq!(sol_type_to_ts(&SolType::Int(48), Target::Viem), "number");
        assert_eq!(sol_type_to_ts(&SolType::Int(48), Target::Ethers), "number");
        // 49-bit should cross into "bigint"
        assert_eq!(sol_type_to_ts(&SolType::Int(49), Target::Viem), "bigint");
        assert_eq!(sol_type_to_ts(&SolType::Int(49), Target::Ethers), "bigint");
    }

    // ── overload_suffix tests ──────────────────────────────────────────

    #[test]
    fn overload_suffix_empty_inputs() {
        assert_eq!(overload_suffix(&[]), "");
    }

    #[test]
    fn overload_suffix_single_uint() {
        let inputs = vec![AbiParam {
            name: "amount".into(),
            ty: SolType::Uint(256),
            internal_type: None,
        }];
        assert_eq!(overload_suffix(&inputs), "Uint256");
    }

    #[test]
    fn overload_suffix_uint_and_address() {
        let inputs = vec![
            AbiParam {
                name: "amount".into(),
                ty: SolType::Uint(256),
                internal_type: None,
            },
            AbiParam {
                name: "recipient".into(),
                ty: SolType::Address,
                internal_type: None,
            },
        ];
        assert_eq!(overload_suffix(&inputs), "Uint256Address");
    }

    #[test]
    fn overload_suffix_bool_string_bytes() {
        let inputs = vec![
            AbiParam {
                name: "flag".into(),
                ty: SolType::Bool,
                internal_type: None,
            },
            AbiParam {
                name: "label".into(),
                ty: SolType::StringType,
                internal_type: None,
            },
            AbiParam {
                name: "data".into(),
                ty: SolType::Bytes,
                internal_type: None,
            },
        ];
        assert_eq!(overload_suffix(&inputs), "BoolStringBytes");
    }

    #[test]
    fn overload_suffix_bytesn() {
        let inputs = vec![AbiParam {
            name: "hash".into(),
            ty: SolType::BytesN(32),
            internal_type: None,
        }];
        assert_eq!(overload_suffix(&inputs), "Bytes32");
    }

    #[test]
    fn overload_suffix_int() {
        let inputs = vec![AbiParam {
            name: "delta".into(),
            ty: SolType::Int(128),
            internal_type: None,
        }];
        assert_eq!(overload_suffix(&inputs), "Int128");
    }

    #[test]
    fn overload_suffix_array() {
        let inputs = vec![AbiParam {
            name: "values".into(),
            ty: SolType::Array(Box::new(SolType::Uint(256))),
            internal_type: None,
        }];
        assert_eq!(overload_suffix(&inputs), "Uint256Array");
    }

    #[test]
    fn overload_suffix_fixed_array() {
        let inputs = vec![AbiParam {
            name: "pair".into(),
            ty: SolType::FixedArray(Box::new(SolType::Address), 2),
            internal_type: None,
        }];
        assert_eq!(overload_suffix(&inputs), "AddressArray2");
    }

    #[test]
    fn overload_suffix_tuple() {
        let inputs = vec![AbiParam {
            name: "pos".into(),
            ty: SolType::Tuple(vec![TupleComponent {
                name: "x".into(),
                ty: SolType::Uint(256),
                internal_type: None,
            }]),
            internal_type: None,
        }];
        assert_eq!(overload_suffix(&inputs), "Tuple");
    }

    #[test]
    fn sol_type_short_name_all_variants() {
        assert_eq!(sol_type_short_name(&SolType::Uint(256)), "Uint256");
        assert_eq!(sol_type_short_name(&SolType::Int(128)), "Int128");
        assert_eq!(sol_type_short_name(&SolType::Bool), "Bool");
        assert_eq!(sol_type_short_name(&SolType::Address), "Address");
        assert_eq!(sol_type_short_name(&SolType::Bytes), "Bytes");
        assert_eq!(sol_type_short_name(&SolType::BytesN(4)), "Bytes4");
        assert_eq!(sol_type_short_name(&SolType::StringType), "String");
        assert_eq!(
            sol_type_short_name(&SolType::Array(Box::new(SolType::Bool))),
            "BoolArray"
        );
        assert_eq!(
            sol_type_short_name(&SolType::FixedArray(Box::new(SolType::Bool), 3)),
            "BoolArray3"
        );
        assert_eq!(sol_type_short_name(&SolType::Tuple(vec![])), "Tuple");
    }

    #[test]
    fn overload_suffix_from_types_reuses_same_naming_rules() {
        let types = [SolType::Uint(256), SolType::Address];
        assert_eq!(
            overload_suffix_from_sol_types(types.iter()),
            "Uint256Address"
        );
        assert_eq!(overload_suffix_from_sol_types([].iter()), "");
    }
}
