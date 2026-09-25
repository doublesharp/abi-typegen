//! Kotlin output: one `object` per contract with its ABI, selectors, and
//! value types built on web3j.
//!
//! Tuples extend web3j's `StaticStruct` or `DynamicStruct`, so they encode
//! directly. Integers are `BigInteger`, `bytesN` and `bytes` use web3j's
//! value types, and nothing uses Kotlin's unsigned types, so every getter is
//! callable from Java.

use crate::naming::{Scope, exported, overload_indices, param_names, repeat_indices};
use crate::tuples::TupleRegistry;
use abi_typegen_core::types::{ContractIr, NatSpec, SolType};
use heck::ToShoutySnakeCase;
use std::collections::{BTreeSet, HashMap};

mod wrappers;

/// Kotlin hard keywords, which need backticks when used as identifiers.
const KOTLIN_KEYWORDS: &[&str] = &[
    "as",
    "break",
    "class",
    "continue",
    "do",
    "else",
    "false",
    "for",
    "fun",
    "if",
    "in",
    "interface",
    "is",
    "null",
    "object",
    "package",
    "return",
    "super",
    "this",
    "throw",
    "true",
    "try",
    "typealias",
    "typeof",
    "val",
    "var",
    "when",
    "while",
];

/// Properties that web3j's `Array` already defines as getters. A struct field
/// with one of these names would clash, so it gets a trailing underscore.
const STRUCT_RESERVED: &[&str] = &["value", "typeAsString", "componentType"];

/// Names the generated code refers to. A nested class with one of these
/// names would shadow the import inside the contract object.
const SDK_NAMES: &[&str] = &[
    "Address",
    "BigInteger",
    "Bool",
    "Boolean",
    "DynamicArray",
    "DynamicBytes",
    "DynamicStruct",
    "JvmField",
    "List",
    "Parameterized",
    "StaticArray",
    "StaticStruct",
    "String",
    "Utf8String",
];

/// Longest JSON chunk, in characters. At most three modified-UTF-8 bytes per
/// character keeps each chunk below the JVM's 65,535-byte constant limit.
const JSON_CHUNK_CHARS: usize = 16_384;

/// Largest fixed array web3j has a `StaticArrayN` class for.
const MAX_STATIC_ARRAY: usize = 32;

/// One property of a rendered class.
#[derive(Debug)]
struct Field {
    name: String,
    ty: String,
    /// web3j value for the struct super-constructor; empty outside structs.
    web3j: String,
}

/// Returns the contract object name, avoiding Kotlin and web3j type names.
pub fn namespace_name(contract_name: &str) -> String {
    let contract = exported(contract_name);
    if is_web3j_generated_name(&contract) {
        format!("{contract}Contract")
    } else {
        Scope::with_reserved(SDK_NAMES.iter().copied()).claim(&contract)
    }
}

/// Renders `<Name>.kt` for `ir` in package `package`.
pub fn render_kotlin_file(ir: &ContractIr, package: &str) -> String {
    render_kotlin_file_with_wrappers(ir, package, false)
}

/// Renders Kotlin ABI types, optionally including SDK-backed contract wrappers.
pub fn render_kotlin_file_with_wrappers(ir: &ContractIr, package: &str, wrappers: bool) -> String {
    let registry = TupleRegistry::new(ir);
    let contract = namespace_name(&ir.name);
    let mut imports = BTreeSet::new();
    let mut scope = Scope::with_reserved(
        [contract.as_str(), "JSON"]
            .into_iter()
            .chain(SDK_NAMES.iter().copied())
            .chain(
                if wrappers {
                    &["Client", "TransactionOptions"][..]
                } else {
                    &[]
                }
                .iter()
                .copied(),
            ),
    );

    let tuple_names: HashMap<String, String> = registry
        .defs()
        .iter()
        .map(|def| {
            let name = if is_web3j_generated_name(&def.name) {
                format!("{}Tuple", def.name)
            } else {
                def.name.clone()
            };
            (def.name.clone(), scope.claim(&name))
        })
        .collect();
    let kotlin = Kotlin {
        registry: &registry,
        tuple_names: &tuple_names,
    };
    // Struct fields also get the web3j values their super-constructor takes.
    let fields_of = |params: Vec<(&str, &SolType, Option<&str>)>,
                     is_struct: bool,
                     imports: &mut BTreeSet<&'static str>| {
        let reserved: &[&str] = if is_struct { STRUCT_RESERVED } else { &[] };
        let names = param_names(params.iter().map(|(name, ty, internal_type)| {
            (*name, *ty, registry.name_of_type(ty, *internal_type))
        }));
        let mut field_scope = Scope::default();
        names
            .iter()
            .zip(&params)
            .map(|(name, (_, ty, internal_type))| {
                let name = if reserved.contains(&name.as_str()) {
                    format!("{name}_")
                } else {
                    name.clone()
                };
                let name = field_scope.claim(&name);
                let ty_name = kotlin.field_type(ty, *internal_type, imports);
                let web3j = if is_struct {
                    kotlin.to_web3j(&escape_keyword(&name), ty, *internal_type, imports)
                } else {
                    String::new()
                };
                Field {
                    name,
                    ty: ty_name,
                    web3j,
                }
            })
            .collect::<Vec<_>>()
    };

    let mut constants: Vec<(String, String, String)> = Vec::new();
    let mut classes = Vec::new();

    for def in registry.defs() {
        let fields = fields_of(
            def.components
                .iter()
                .map(|c| (c.name.as_str(), &c.ty, c.internal_type.as_deref()))
                .collect(),
            true,
            &mut imports,
        );
        let dynamic = def.components.iter().any(|c| is_dynamic(&c.ty));
        let base = if dynamic {
            imports.insert("org.web3j.abi.datatypes.DynamicStruct");
            "DynamicStruct"
        } else {
            imports.insert("org.web3j.abi.datatypes.StaticStruct");
            "StaticStruct"
        };
        let args = fields
            .iter()
            .map(|field| field.web3j.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let mut class = render_class(
            &tuple_names[&def.name],
            &format!("The `{}` tuple.", def.name),
            None,
            &fields,
            Some(&format!("{base}({args})")),
        );
        let decoder_types: Vec<_> = def
            .components
            .iter()
            .map(|c| kotlin.decoder_type(&c.ty, c.internal_type.as_deref(), &mut imports))
            .collect();
        if def.components.iter().any(|c| match c.ty {
            SolType::Bytes | SolType::BytesN(_) | SolType::Tuple(_) => false,
            SolType::Bool
            | SolType::StringType
            | SolType::Address
            | SolType::Uint(_)
            | SolType::Int(_)
            | SolType::Array(_)
            | SolType::FixedArray(_, _) => true,
        }) {
            let params = fields
                .iter()
                .zip(&decoder_types)
                .zip(&def.components)
                .map(|((f, ty), component)| {
                    let annotation = match &component.ty {
                        SolType::Array(inner) | SolType::FixedArray(inner, _) => {
                            imports.insert("org.web3j.abi.datatypes.reflection.Parameterized");
                            let element = kotlin.web3j_class(
                                inner,
                                component.internal_type.as_deref(),
                                &mut imports,
                            );
                            format!("@Parameterized(type = {element}::class) ")
                        }
                        _ => String::new(),
                    };
                    format!("{annotation}{}: {ty}", escape_keyword(&f.name))
                })
                .collect::<Vec<_>>()
                .join(", ");
            let values = fields
                .iter()
                .zip(&def.components)
                .map(|(f, c)| Kotlin::from_web3j(&escape_keyword(&f.name), &c.ty))
                .collect::<Vec<_>>()
                .join(", ");
            class.pop();
            class.push_str(&format!(
                " {{\n        constructor({params}) : this({values})\n    }}\n"
            ));
        }
        classes.push(class);
    }

    for (function, overload) in ir.functions.iter().zip(overload_indices(&ir.functions)) {
        let index = overload.map(|i| i.to_string()).unwrap_or_default();
        let constant = constant_base(&function.name, &index);
        let constant = scope.claim_family(&constant, &["_SIGNATURE", "_SELECTOR"]);
        let signature = function.signature();
        constants.push((
            format!("{constant}_SIGNATURE"),
            format!("Canonical signature of `{signature}`."),
            kotlin_string(&signature),
        ));
        constants.push((
            format!("{constant}_SELECTOR"),
            format!("Selector of `{signature}`."),
            kotlin_string(&function.selector().to_string()),
        ));
        if function.inputs.is_empty() {
            continue;
        }
        let fields = fields_of(
            function
                .inputs
                .iter()
                .map(|p| (p.name.as_str(), &p.ty, p.internal_type.as_deref()))
                .collect(),
            false,
            &mut imports,
        );
        classes.push(render_class(
            &format!(
                "{}Params",
                scope.claim_family(&format!("{}{index}", exported(&function.name)), &["Params"])
            ),
            &format!("Arguments of `{signature}`."),
            function.natspec.as_ref(),
            &fields,
            None,
        ));
    }

    for (event, index) in ir
        .events
        .iter()
        .zip(suffixes(ir.events.iter().map(|e| e.name.as_str())))
    {
        let constant = constant_base(&event.name, &index);
        let constant = scope.claim_family(&constant, &["_EVENT_SIGNATURE", "_EVENT_TOPIC"]);
        let signature = event.signature();
        constants.push((
            format!("{constant}_EVENT_SIGNATURE"),
            format!("Canonical signature of the `{signature}` event."),
            kotlin_string(&signature),
        ));
        if !event.anonymous {
            constants.push((
                format!("{constant}_EVENT_TOPIC"),
                format!("Topic 0 of the `{signature}` event."),
                kotlin_string(&event.topic0().to_string()),
            ));
        }
        let fields = fields_of(
            event
                .inputs
                .iter()
                .map(|p| (p.name.as_str(), &p.ty, p.internal_type.as_deref()))
                .collect(),
            false,
            &mut imports,
        );
        classes.push(render_class(
            &format!(
                "{}Event",
                scope.claim_family(&format!("{}{index}", exported(&event.name)), &["Event"])
            ),
            &format!("Fields of the `{signature}` event."),
            event.natspec.as_ref(),
            &fields,
            None,
        ));
    }

    for (error, index) in ir
        .errors
        .iter()
        .zip(suffixes(ir.errors.iter().map(|e| e.name.as_str())))
    {
        let constant = constant_base(&error.name, &index);
        let constant = scope.claim_family(&constant, &["_ERROR_SIGNATURE", "_ERROR_SELECTOR"]);
        let signature = error.signature();
        constants.push((
            format!("{constant}_ERROR_SIGNATURE"),
            format!("Canonical signature of the `{signature}` error."),
            kotlin_string(&signature),
        ));
        constants.push((
            format!("{constant}_ERROR_SELECTOR"),
            format!("Selector of the `{signature}` error."),
            kotlin_string(&error.selector().to_string()),
        ));
        let fields = fields_of(
            error
                .inputs
                .iter()
                .map(|p| (p.name.as_str(), &p.ty, p.internal_type.as_deref()))
                .collect(),
            false,
            &mut imports,
        );
        classes.push(render_class(
            &format!(
                "{}Error",
                scope.claim_family(&format!("{}{index}", exported(&error.name)), &["Error"])
            ),
            &format!("Arguments of the `{signature}` error."),
            error.natspec.as_ref(),
            &fields,
            None,
        ));
    }

    let wrapper_output = if wrappers {
        wrappers::render(ir, &kotlin, &mut imports)
    } else {
        String::new()
    };
    let mut out = String::from("// Generated by abi-typegen. Do not edit.\n\n");
    out.push_str(&format!("package {package}\n\n"));
    if !imports.is_empty() {
        for import in &imports {
            out.push_str(&format!("import {import}\n"));
        }
        out.push('\n');
    }
    out.push_str(&kdoc(
        "",
        &format!("Types and constants of the `{}` contract.", ir.name),
        ir.natspec.as_ref(),
    ));
    out.push_str(&format!("object {contract} {{\n"));
    out.push_str(&format!(
        "    /** JSON ABI of the `{}` contract. */\n    @JvmField\n    val JSON: String = {}\n",
        ir.name,
        json_expression(&serde_json::to_string(&ir.raw_abi).expect("JSON values serialize"))
    ));
    for (name, doc, value) in &constants {
        out.push_str(&format!(
            "\n    /** {doc} */\n    const val {name}: String = {value}\n"
        ));
    }
    for class in &classes {
        out.push('\n');
        out.push_str(class);
    }
    out.push_str(&wrapper_output);
    out.push_str("}\n");
    out
}

/// Type mapping for one contract.
struct Kotlin<'a> {
    registry: &'a TupleRegistry,
    tuple_names: &'a HashMap<String, String>,
}

impl Kotlin<'_> {
    /// The Kotlin type of a field.
    fn field_type(
        &self,
        ty: &SolType,
        internal_type: Option<&str>,
        imports: &mut BTreeSet<&'static str>,
    ) -> String {
        match ty {
            SolType::Bool => "Boolean".to_string(),
            SolType::StringType | SolType::Address => "String".to_string(),
            SolType::Uint(_) | SolType::Int(_) => {
                imports.insert("java.math.BigInteger");
                "BigInteger".to_string()
            }
            SolType::Bytes => {
                imports.insert("org.web3j.abi.datatypes.DynamicBytes");
                "DynamicBytes".to_string()
            }
            SolType::BytesN(size) => {
                imports.insert(bytes_n_import(*size));
                format!("Bytes{size}")
            }
            SolType::Array(inner) | SolType::FixedArray(inner, _) => {
                format!("List<{}>", self.field_type(inner, internal_type, imports))
            }
            SolType::Tuple(components) => {
                self.tuple_names[self.registry.name(components, internal_type)].clone()
            }
        }
    }

    /// The web3j class of a type, for array element classes.
    fn web3j_class(
        &self,
        ty: &SolType,
        internal_type: Option<&str>,
        imports: &mut BTreeSet<&'static str>,
    ) -> String {
        match ty {
            SolType::Bool => {
                imports.insert("org.web3j.abi.datatypes.Bool");
                "Bool".to_string()
            }
            SolType::StringType => {
                imports.insert("org.web3j.abi.datatypes.Utf8String");
                "Utf8String".to_string()
            }
            SolType::Address => {
                imports.insert("org.web3j.abi.datatypes.Address");
                "Address".to_string()
            }
            SolType::Uint(bits) => {
                imports.insert(uint_import(*bits));
                format!("Uint{bits}")
            }
            SolType::Int(bits) => {
                imports.insert(int_import(*bits));
                format!("Int{bits}")
            }
            SolType::Array(_) => {
                imports.insert("org.web3j.abi.datatypes.DynamicArray");
                "DynamicArray".to_string()
            }
            SolType::FixedArray(_, size) => {
                let class = static_array_class(*size);
                imports.insert(static_array_import(*size));
                class
            }
            SolType::Bytes | SolType::BytesN(_) | SolType::Tuple(_) => {
                self.field_type(ty, internal_type, imports)
            }
        }
    }

    /// Complete web3j constructor parameter type, including array elements.
    fn decoder_type(
        &self,
        ty: &SolType,
        internal_type: Option<&str>,
        imports: &mut BTreeSet<&'static str>,
    ) -> String {
        let class = self.web3j_class(ty, internal_type, imports);
        match ty {
            SolType::Array(inner) | SolType::FixedArray(inner, _) => {
                format!(
                    "{class}<{}>",
                    self.decoder_type(inner, internal_type, imports)
                )
            }
            _ => class,
        }
    }

    /// Converts decoded SDK values back to the convenience property types.
    fn from_web3j(expr: &str, ty: &SolType) -> String {
        match ty {
            SolType::Bytes | SolType::BytesN(_) | SolType::Tuple(_) => expr.to_string(),
            SolType::Bool
            | SolType::StringType
            | SolType::Address
            | SolType::Uint(_)
            | SolType::Int(_) => format!("{expr}.value"),
            SolType::Array(inner) | SolType::FixedArray(inner, _) => {
                let item = Self::from_web3j("it", inner);
                if item == "it" {
                    format!("{expr}.value")
                } else {
                    format!("{expr}.value.map {{ {item} }}")
                }
            }
        }
    }

    /// Converts a Kotlin expression of type `ty` into a web3j value.
    fn to_web3j(
        &self,
        expr: &str,
        ty: &SolType,
        internal_type: Option<&str>,
        imports: &mut BTreeSet<&'static str>,
    ) -> String {
        match ty {
            // Fields of these types already hold web3j values.
            SolType::Bytes | SolType::BytesN(_) | SolType::Tuple(_) => expr.to_string(),
            SolType::Bool
            | SolType::StringType
            | SolType::Address
            | SolType::Uint(_)
            | SolType::Int(_) => {
                format!("{}({expr})", self.web3j_class(ty, internal_type, imports))
            }
            SolType::Array(inner) | SolType::FixedArray(inner, _) => {
                let array = self.web3j_class(ty, internal_type, imports);
                let element = self.web3j_class(inner, internal_type, imports);
                let item = self.to_web3j("it", inner, internal_type, imports);
                let items = if item == "it" {
                    expr.to_string()
                } else {
                    format!("{expr}.map {{ {item} }}")
                };
                format!("{array}({element}::class.java, {items})")
            }
        }
    }
}

/// Whether `name` is a web3j generated class such as `Uint256`, `Bytes32`, or
/// `StaticArray2`, which a nested class of that name would shadow.
fn is_web3j_generated_name(name: &str) -> bool {
    ["Uint", "Int", "Bytes", "StaticArray"]
        .iter()
        .any(|prefix| {
            name.strip_prefix(prefix)
                .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
        })
}

/// Whether a type is dynamic in the ABI encoding.
fn is_dynamic(ty: &SolType) -> bool {
    match ty {
        SolType::Bytes | SolType::StringType | SolType::Array(_) => true,
        SolType::FixedArray(inner, _) => is_dynamic(inner),
        SolType::Tuple(components) => components.iter().any(|c| is_dynamic(&c.ty)),
        SolType::Uint(_)
        | SolType::Int(_)
        | SolType::Bool
        | SolType::Address
        | SolType::BytesN(_) => false,
    }
}

fn static_array_class(size: usize) -> String {
    if (1..=MAX_STATIC_ARRAY).contains(&size) {
        format!("StaticArray{size}")
    } else {
        // web3j rejects longer static arrays at run time.
        "StaticArray".to_string()
    }
}

fn static_array_import(size: usize) -> &'static str {
    const IMPORTS: [&str; MAX_STATIC_ARRAY] = [
        "org.web3j.abi.datatypes.generated.StaticArray1",
        "org.web3j.abi.datatypes.generated.StaticArray2",
        "org.web3j.abi.datatypes.generated.StaticArray3",
        "org.web3j.abi.datatypes.generated.StaticArray4",
        "org.web3j.abi.datatypes.generated.StaticArray5",
        "org.web3j.abi.datatypes.generated.StaticArray6",
        "org.web3j.abi.datatypes.generated.StaticArray7",
        "org.web3j.abi.datatypes.generated.StaticArray8",
        "org.web3j.abi.datatypes.generated.StaticArray9",
        "org.web3j.abi.datatypes.generated.StaticArray10",
        "org.web3j.abi.datatypes.generated.StaticArray11",
        "org.web3j.abi.datatypes.generated.StaticArray12",
        "org.web3j.abi.datatypes.generated.StaticArray13",
        "org.web3j.abi.datatypes.generated.StaticArray14",
        "org.web3j.abi.datatypes.generated.StaticArray15",
        "org.web3j.abi.datatypes.generated.StaticArray16",
        "org.web3j.abi.datatypes.generated.StaticArray17",
        "org.web3j.abi.datatypes.generated.StaticArray18",
        "org.web3j.abi.datatypes.generated.StaticArray19",
        "org.web3j.abi.datatypes.generated.StaticArray20",
        "org.web3j.abi.datatypes.generated.StaticArray21",
        "org.web3j.abi.datatypes.generated.StaticArray22",
        "org.web3j.abi.datatypes.generated.StaticArray23",
        "org.web3j.abi.datatypes.generated.StaticArray24",
        "org.web3j.abi.datatypes.generated.StaticArray25",
        "org.web3j.abi.datatypes.generated.StaticArray26",
        "org.web3j.abi.datatypes.generated.StaticArray27",
        "org.web3j.abi.datatypes.generated.StaticArray28",
        "org.web3j.abi.datatypes.generated.StaticArray29",
        "org.web3j.abi.datatypes.generated.StaticArray30",
        "org.web3j.abi.datatypes.generated.StaticArray31",
        "org.web3j.abi.datatypes.generated.StaticArray32",
    ];
    match size {
        1..=MAX_STATIC_ARRAY => IMPORTS[size - 1],
        _ => "org.web3j.abi.datatypes.StaticArray",
    }
}

/// Import path of web3j's `UintN` class.
fn uint_import(bits: u16) -> &'static str {
    web3j_int_import("Uint", bits)
}

/// Import path of web3j's `IntN` class.
fn int_import(bits: u16) -> &'static str {
    web3j_int_import("Int", bits)
}

fn web3j_int_import(prefix: &str, bits: u16) -> &'static str {
    const WIDTHS: [u16; 32] = [
        8, 16, 24, 32, 40, 48, 56, 64, 72, 80, 88, 96, 104, 112, 120, 128, 136, 144, 152, 160, 168,
        176, 184, 192, 200, 208, 216, 224, 232, 240, 248, 256,
    ];
    macro_rules! table {
        ($prefix:literal) => {
            [
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "8"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "16"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "24"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "32"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "40"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "48"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "56"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "64"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "72"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "80"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "88"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "96"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "104"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "112"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "120"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "128"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "136"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "144"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "152"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "160"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "168"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "176"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "184"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "192"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "200"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "208"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "216"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "224"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "232"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "240"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "248"),
                concat!("org.web3j.abi.datatypes.generated.", $prefix, "256"),
            ]
        };
    }
    const UINTS: [&str; 32] = table!("Uint");
    const INTS: [&str; 32] = table!("Int");
    let table = if prefix == "Uint" { &UINTS } else { &INTS };
    let index = WIDTHS
        .iter()
        .position(|width| *width == bits)
        .expect("the parser only accepts multiples of 8 from 8 to 256");
    table[index]
}

fn bytes_n_import(size: u8) -> &'static str {
    macro_rules! bytes {
        ($($n:literal),*) => {
            [$(concat!("org.web3j.abi.datatypes.generated.Bytes", $n)),*]
        };
    }
    const BYTES: [&str; 32] = bytes!(
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30, 31, 32
    );
    BYTES[usize::from(size) - 1]
}

/// `safeTransferFrom` with overload `0` becomes `SAFE_TRANSFER_FROM_0`.
fn constant_base(name: &str, index: &str) -> String {
    let base = name.to_shouty_snake_case();
    if index.is_empty() {
        base
    } else {
        format!("{base}_{index}")
    }
}

fn render_class(
    name: &str,
    summary: &str,
    natspec: Option<&NatSpec>,
    fields: &[Field],
    superclass: Option<&str>,
) -> String {
    let mut out = kdoc("    ", summary, natspec);
    if fields.is_empty() {
        out.push_str(&format!("    data object {name}\n"));
        return out;
    }
    out.push_str(&format!("    data class {name}(\n"));
    for field in fields {
        out.push_str(&format!(
            "        val {}: {},\n",
            escape_keyword(&field.name),
            field.ty
        ));
    }
    match superclass {
        Some(superclass) => out.push_str(&format!("    ) : {superclass}\n")),
        None => out.push_str("    )\n"),
    }
    out
}

fn escape_keyword(name: &str) -> String {
    if KOTLIN_KEYWORDS.contains(&name) || (!name.is_empty() && name.chars().all(|c| c == '_')) {
        format!("`{name}`")
    } else {
        name.to_string()
    }
}

fn kdoc(indent: &str, summary: &str, natspec: Option<&NatSpec>) -> String {
    let lines: Vec<String> = natspec
        .into_iter()
        .flat_map(|natspec| natspec.notice.iter().chain(natspec.dev.iter()))
        .flat_map(|text| text.lines())
        .map(|line| line.trim().replace("*/", "*&#47;"))
        .collect();
    if lines.is_empty() {
        return format!("{indent}/** {summary} */\n");
    }
    let mut out = format!("{indent}/**\n{indent} * {summary}\n{indent} *\n");
    for line in lines {
        if line.is_empty() {
            out.push_str(&format!("{indent} *\n"));
        } else {
            out.push_str(&format!("{indent} * {line}\n"));
        }
    }
    out.push_str(&format!("{indent} */\n"));
    out
}

fn kotlin_string(value: &str) -> String {
    let mut out = String::from("\"");
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '$' => out.push_str("\\$"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Builds the JSON at run time from chunks, so kotlinc cannot fold them into
/// one constant larger than the JVM allows.
fn json_expression(json: &str) -> String {
    let chars: Vec<char> = json.chars().collect();
    let chunks: Vec<String> = chars
        .chunks(JSON_CHUNK_CHARS)
        .map(|chunk| kotlin_string(&chunk.iter().collect::<String>()))
        .collect();
    if chunks.is_empty() {
        return "\"\"".to_string();
    }
    let items = chunks
        .iter()
        .map(|chunk| format!("        {chunk},\n"))
        .collect::<String>();
    format!("arrayOf(\n{items}    ).joinToString(\"\")")
}

/// Overload suffixes: `"0"`, `"1"`, ... for repeated names, `""` otherwise.
fn suffixes<'a>(names: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    repeat_indices(names)
        .into_iter()
        .map(|index| index.map(|i| i.to_string()).unwrap_or_default())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use abi_typegen_core::parser::parse_artifact;

    fn render(abi: &str) -> String {
        let ir = parse_artifact("Token", &format!(r#"{{"abi":{abi}}}"#)).expect("valid abi");
        render_kotlin_file(&ir, "com.example.contracts")
    }

    #[test]
    fn wrappers_include_typed_calls_events_and_errors() {
        let ir = parse_artifact("Token", r#"{"abi":[
            {"type":"function","name":"balanceOf","inputs":[{"name":"owner","type":"address"}],"outputs":[{"name":"balance","type":"uint256"}],"stateMutability":"view"},
            {"type":"function","name":"transfer","inputs":[{"name":"to","type":"address"},{"name":"value","type":"uint256"}],"outputs":[{"name":"ok","type":"bool"}],"stateMutability":"nonpayable"},
            {"type":"event","name":"Transfer","inputs":[{"name":"from","type":"address","indexed":true},{"name":"to","type":"address","indexed":true},{"name":"value","type":"uint256","indexed":false}]},
            {"type":"error","name":"Unauthorized","inputs":[{"name":"owner","type":"address"}]}
        ]}"#).expect("valid abi");
        let out = render_kotlin_file_with_wrappers(&ir, "contracts", true);
        assert!(out.contains("fun encodeBalanceOf("), "{out}");
        assert!(out.contains("fun decodeBalanceOfResult("), "{out}");
        assert!(out.contains("fun callBalanceOf("), "{out}");
        assert!(out.contains("fun sendTransfer("), "{out}");
        assert!(out.contains("fun decodeTransferEvent("), "{out}");
        assert!(out.contains("fun decodeUnauthorizedError("), "{out}");
        assert!(!render_kotlin_file(&ir, "contracts").contains("fun encodeBalanceOf("));
    }

    #[test]
    fn uses_configured_package_and_contract_object() {
        let out = render("[]");
        assert!(
            out.starts_with(
                "// Generated by abi-typegen. Do not edit.\n\npackage com.example.contracts\n"
            ),
            "{out}"
        );
        assert!(out.contains("object Token {"), "{out}");
        assert!(
            out.contains("@JvmField\n    val JSON: String = arrayOf(\n"),
            "{out}"
        );
    }

    #[test]
    fn embeds_signatures_selectors_and_topics() {
        let out = render(
            r#"[{"type":"function","name":"transfer","inputs":[{"name":"to","type":"address"},{"name":"amount","type":"uint256"}],"outputs":[],"stateMutability":"nonpayable"},
               {"type":"event","name":"Transfer","inputs":[],"anonymous":false}]"#,
        );
        assert!(
            out.contains("const val TRANSFER_SIGNATURE: String = \"transfer(address,uint256)\""),
            "{out}"
        );
        assert!(
            out.contains("const val TRANSFER_SELECTOR: String = \"0xa9059cbb\""),
            "{out}"
        );
        assert!(
            out.contains("const val TRANSFER_EVENT_TOPIC: String = \"0x"),
            "{out}"
        );
    }

    #[test]
    fn tuples_extend_web3j_structs() {
        let out = render(
            r#"[{"type":"function","name":"deposit","inputs":[{"name":"p","type":"tuple","internalType":"struct Token.Position","components":[
                {"name":"shares","type":"uint256"},{"name":"token","type":"address"},{"name":"memo","type":"bytes32"},{"name":"flags","type":"bool[2]"}
            ]}],"outputs":[],"stateMutability":"nonpayable"}]"#,
        );
        assert!(
            out.contains(
                "    data class Position(\n        val shares: BigInteger,\n        val token: String,\n        val memo: Bytes32,\n        val flags: List<Boolean>,\n    ) : StaticStruct(Uint256(shares), Address(token), memo, StaticArray2(Bool::class.java, flags.map { Bool(it) })) {\n"
            ),
            "{out}"
        );
        assert!(out.contains("val p: Position,"), "{out}");
        assert!(
            out.contains("import org.web3j.abi.datatypes.generated.StaticArray2\n"),
            "{out}"
        );
        assert!(!out.contains("Map<String, Any>"), "{out}");
    }

    #[test]
    fn dynamic_members_make_dynamic_structs() {
        let out = render(
            r#"[{"type":"function","name":"f","inputs":[{"name":"p","type":"tuple","internalType":"struct Token.Batch","components":[
                {"name":"data","type":"bytes"},{"name":"rows","type":"uint256[][]"}
            ]}],"outputs":[],"stateMutability":"nonpayable"}]"#,
        );
        assert!(
            out.contains(") : DynamicStruct(data, DynamicArray(DynamicArray::class.java, rows.map { DynamicArray(Uint256::class.java, it.map { Uint256(it) }) }))"),
            "{out}"
        );
    }

    #[test]
    fn inherited_getter_names_are_renamed_in_structs() {
        let out = render(
            r#"[{"type":"function","name":"f","inputs":[{"name":"call","type":"tuple","internalType":"struct Token.Call3Value","components":[
                {"name":"target","type":"address"},{"name":"value","type":"uint256"}
            ]},{"name":"value","type":"uint256"}],"outputs":[],"stateMutability":"payable"}]"#,
        );
        assert!(out.contains("        val value_: BigInteger,\n    ) : StaticStruct(Address(target), Uint256(value_))"), "{out}");
        // Plain classes keep the ABI name.
        assert!(
            out.contains("        val value: BigInteger,\n    )\n"),
            "{out}"
        );
    }

    #[test]
    fn casing_is_consistent_and_overloads_are_numbered() {
        let out = render(
            r#"[
            {"type":"function","name":"tokenURI","inputs":[{"name":"id","type":"uint256"}],"outputs":[],"stateMutability":"view"},
            {"type":"function","name":"safeTransferFrom","inputs":[{"name":"from","type":"address"}],"outputs":[],"stateMutability":"nonpayable"},
            {"type":"function","name":"safeTransferFrom","inputs":[{"name":"from","type":"address"},{"name":"data","type":"bytes"}],"outputs":[],"stateMutability":"nonpayable"},
            {"type":"event","name":"transferred","inputs":[{"name":"x","type":"uint8"}],"anonymous":false},
            {"type":"error","name":"notOwner","inputs":[{"name":"x","type":"uint8"}]}
        ]"#,
        );
        assert!(out.contains("data class TokenURIParams("), "{out}");
        assert!(out.contains("data class SafeTransferFrom0Params("), "{out}");
        assert!(out.contains("data class SafeTransferFrom1Params("), "{out}");
        assert!(out.contains("SAFE_TRANSFER_FROM_1_SELECTOR"), "{out}");
        assert!(out.contains("data class TransferredEvent("), "{out}");
        assert!(out.contains("data class NotOwnerError("), "{out}");
    }

    #[test]
    fn no_unsigned_or_byte_array_types() {
        let out = render(
            r#"[{"type":"function","name":"f","inputs":[{"name":"a","type":"uint8"},{"name":"b","type":"uint64"},{"name":"c","type":"bytes"},{"name":"d","type":"bytes4"}],"outputs":[],"stateMutability":"nonpayable"}]"#,
        );
        for bad in ["UInt", "ULong", "ByteArray", "Long", ": Int"] {
            assert!(!out.contains(bad), "{bad} in:\n{out}");
        }
        assert!(out.contains("val c: DynamicBytes,"), "{out}");
        assert!(out.contains("val d: Bytes4,"), "{out}");
    }

    #[test]
    fn empty_items_are_data_objects() {
        let out = render(r#"[{"type":"error","name":"ZeroAmount","inputs":[]}]"#);
        assert!(out.contains("    data object ZeroAmountError\n"), "{out}");
    }

    #[test]
    fn keywords_are_escaped_and_unnamed_params_use_types() {
        let out = render(
            r#"[{"type":"function","name":"f","inputs":[{"name":"in","type":"bool"},{"name":"","type":"address"}],"outputs":[],"stateMutability":"nonpayable"}]"#,
        );
        assert!(out.contains("val `in`: Boolean,"), "{out}");
        assert!(out.contains("val address: String,"), "{out}");
    }

    #[test]
    fn tuples_named_like_sdk_types_do_not_shadow_them() {
        let out = render(
            r#"[{"type":"function","name":"f","inputs":[
                {"name":"a","type":"tuple","internalType":"struct Token.Address","components":[{"name":"target","type":"address"}]},
                {"name":"b","type":"tuple","internalType":"struct Token.Uint256","components":[{"name":"x","type":"uint256"}]}
            ],"outputs":[],"stateMutability":"nonpayable"}]"#,
        );
        assert!(out.contains("data class Address2("), "{out}");
        assert!(out.contains(") : StaticStruct(Address(target))"), "{out}");
        assert!(out.contains("data class Uint256Tuple("), "{out}");
    }

    #[test]
    fn strings_escape_dollar_signs() {
        assert_eq!(kotlin_string("a$b\"c"), "\"a\\$b\\\"c\"");
    }

    #[test]
    fn large_json_is_split_into_chunks() {
        let json = "x".repeat(JSON_CHUNK_CHARS * 2 + 1);
        let expression = json_expression(&json);
        assert_eq!(expression.matches("        \"").count(), 3, "{expression}");
        assert!(expression.ends_with(").joinToString(\"\")"));
    }
}
