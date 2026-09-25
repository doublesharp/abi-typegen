//! SDK-backed Kotlin contract wrapper generation.

use super::{Kotlin, escape_keyword, kotlin_string, suffixes};
use crate::naming::{Scope, exported, overload_indices, param_names};
use abi_typegen_core::types::{AbiParam, ContractIr, SolType, StateMutability};
use std::collections::BTreeSet;

mod layout;

pub(super) fn render(
    ir: &ContractIr,
    kotlin: &Kotlin<'_>,
    imports: &mut BTreeSet<&'static str>,
) -> String {
    let mut out = String::new();
    out.push_str(r#"
    /** Gas and payable value for a write transaction. */
    data class TransactionOptions(
        val gasPrice: java.math.BigInteger,
        val gasLimit: java.math.BigInteger,
        val value: java.math.BigInteger = java.math.BigInteger.ZERO,
    )

    /** SDK-backed access to a deployed contract. */
    class Client(
        val address: String,
        val web3j: org.web3j.protocol.Web3j,
        val transactionManager: org.web3j.tx.TransactionManager,
    ) {
        /** Submits encoded calldata through the configured transaction manager. */
        fun send(data: String, options: TransactionOptions): org.web3j.protocol.core.methods.response.EthSendTransaction {
            require(options.value.signum() >= 0) { "transaction value must be nonnegative" }
            val response = transactionManager.sendTransaction(
                options.gasPrice, options.gasLimit, address, data, options.value, false,
            )
            if (response.hasError()) throw IllegalStateException(response.error.message)
            return response
        }

        /** Calls the contract at the latest block and returns encoded ABI output. */
        fun call(data: String): String {
            val request = org.web3j.protocol.core.methods.request.Transaction.createEthCallTransaction(
                transactionManager.fromAddress, address, data,
            )
            val response = web3j.ethCall(request, org.web3j.protocol.core.DefaultBlockParameterName.LATEST).send()
            if (response.hasError()) throw IllegalStateException(response.error.message)
            if (response.isReverted) throw IllegalStateException("contract call reverted: ${response.revertReasonEncodedData}")
            return response.value
        }
    }
"#);
    out.push_str(layout::runtime());
    for def in kotlin.registry.defs() {
        let name = &kotlin.tuple_names[&def.name];
        let layouts = def
            .components
            .iter()
            .map(|component| {
                layout::expression(
                    &component.ty,
                    component.internal_type.as_deref(),
                    kotlin,
                    imports,
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let args = def
            .components
            .iter()
            .enumerate()
            .map(|(index, component)| {
                let ty =
                    kotlin.field_type(&component.ty, component.internal_type.as_deref(), imports);
                format!("values[{index}] as {ty}")
            })
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("\n    @Suppress(\"UNCHECKED_CAST\")\n    private fun layout{name}(): AbiLayout = AbiTuple(listOf({layouts})) {{ values -> {name}({args}) }}\n"));
        out.push_str(&format!("\n    /** Decodes a single ABI return value of tuple `{name}`. */\n    @Suppress(\"UNCHECKED_CAST\")\n    fun decode{name}Value(data: String): {name} =\n        (decodeLayout(data, AbiTuple(listOf(layout{name}())) {{ it }}) as List<Any>)[0] as {name}\n"));
        let children = def
            .components
            .iter()
            .map(|component| {
                type_ref(
                    &component.ty,
                    component.internal_type.as_deref(),
                    kotlin,
                    false,
                    imports,
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("\n    /** Type reference for decoding the `{name}` tuple. */\n    fun reference{name}(): org.web3j.abi.TypeReference<{name}> =\n        object : org.web3j.abi.TypeReference<{name}>(false, listOf({children})) {{}}\n"));
    }
    let mut method_scope = Scope::default();
    for (function, overload) in ir.functions.iter().zip(overload_indices(&ir.functions)) {
        let suffix = overload.map(|i| i.to_string()).unwrap_or_default();
        let stem = method_scope.claim(&format!("{}{}", exported(&function.name), suffix));
        let input_names = names(&function.inputs, kotlin);
        let args = function
            .inputs
            .iter()
            .zip(&input_names)
            .map(|(p, name)| {
                format!(
                    "{}: {}",
                    escape_keyword(name),
                    kotlin.field_type(&p.ty, p.internal_type.as_deref(), imports)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let arg_names = input_names
            .iter()
            .map(|name| escape_keyword(name))
            .collect::<Vec<_>>()
            .join(", ");
        let values = function
            .inputs
            .iter()
            .zip(&input_names)
            .map(|(p, name)| {
                kotlin.to_web3j(
                    &escape_keyword(name),
                    &p.ty,
                    p.internal_type.as_deref(),
                    imports,
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let output_refs = function
            .outputs
            .iter()
            .map(|p| type_ref(&p.ty, p.internal_type.as_deref(), kotlin, false, imports))
            .collect::<Vec<_>>()
            .join(", ");
        let input_values = if values.is_empty() {
            "emptyList()".to_string()
        } else {
            format!("listOf({values})")
        };
        let output_values = if output_refs.is_empty() {
            "emptyList()".to_string()
        } else {
            format!("listOf({output_refs})")
        };
        let fn_name = kotlin_string(&function.name);
        let return_type = match function.outputs.len() {
            0 => "Unit".to_string(),
            1 => kotlin.field_type(
                &function.outputs[0].ty,
                function.outputs[0].internal_type.as_deref(),
                imports,
            ),
            _ => format!("{stem}Result"),
        };
        if function
            .inputs
            .iter()
            .any(|p| exceeds_web3j_static_array(&p.ty))
        {
            out.push_str(&format!("\n    /** ABI function for `{}`. */\n    fun function{stem}({args}): org.web3j.abi.datatypes.Function =\n        throw UnsupportedOperationException(\"web3j supports static arrays with 1 to 32 elements\")\n", function.signature()));
        } else {
            out.push_str(&format!("\n    /** ABI function for `{}`. */\n    fun function{stem}({args}): org.web3j.abi.datatypes.Function =\n        org.web3j.abi.datatypes.Function({fn_name}, {input_values}, {output_values})\n", function.signature()));
        }
        out.push_str(&format!("\n    /** Encodes calldata for `{}` without an RPC call. */\n    fun encode{stem}({args}): String = org.web3j.abi.FunctionEncoder.encode(function{stem}({arg_names}))\n", function.signature()));
        if function.outputs.len() > 1 {
            out.push_str(&format!(
                "\n    /** Decoded result of `{}`. */\n    data class {stem}Result(\n",
                function.signature()
            ));
            let output_names = names(&function.outputs, kotlin);
            for (p, name) in function.outputs.iter().zip(&output_names) {
                let ty = kotlin.field_type(&p.ty, p.internal_type.as_deref(), imports);
                out.push_str(&format!("        val {}: {ty},\n", escape_keyword(name)));
            }
            out.push_str("    )\n");
        }
        out.push_str(&format!("\n    /** Decodes the ABI output of `{}`. */\n    @Suppress(\"UNCHECKED_CAST\")\n    fun decode{stem}Result(data: String): {return_type} {{\n", function.signature()));
        if function.outputs.is_empty() {
            out.push_str("        require(data == \"0x\" || data.isEmpty()) { \"unexpected output for void function\" }\n        return Unit\n");
        } else {
            let layouts = function
                .outputs
                .iter()
                .map(|p| layout::expression(&p.ty, p.internal_type.as_deref(), kotlin, imports))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("        val values = decodeLayout(data, AbiTuple(listOf({layouts})) {{ it }}) as List<Any>\n        require(values.size == {}) {{ \"incorrect result arity for {}\" }}\n", function.outputs.len(), function.signature()));
            let exprs = function
                .outputs
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let ty = kotlin.field_type(&p.ty, p.internal_type.as_deref(), imports);
                    format!("values[{i}] as {ty}")
                })
                .collect::<Vec<_>>();
            if exprs.len() == 1 {
                out.push_str(&format!("        return {}\n", exprs[0]));
            } else {
                out.push_str(&format!(
                    "        return {stem}Result({})\n",
                    exprs.join(", ")
                ));
            }
        }
        out.push_str("    }\n");
        match function.state_mutability {
            StateMutability::View | StateMutability::Pure => {
                out.push_str(&format!("\n    /** Executes a read call for `{}`. */\n    fun call{stem}(client: Client{}{}): {return_type} =\n        decode{stem}Result(client.call(encode{stem}({arg_names})))\n", function.signature(), if args.is_empty() { "" } else { ", " }, args));
            }
            StateMutability::NonPayable | StateMutability::Payable => {
                let params = if args.is_empty() {
                    "options: TransactionOptions".to_string()
                } else {
                    format!("{args}, options: TransactionOptions")
                };
                let value_check = if function.state_mutability == StateMutability::NonPayable {
                    "        require(options.value == java.math.BigInteger.ZERO) { \"nonpayable function cannot receive value\" }\n"
                } else {
                    ""
                };
                out.push_str(&format!("\n    /** Sends a write transaction for `{}`. */\n    fun send{stem}(client: Client, {params}): org.web3j.protocol.core.methods.response.EthSendTransaction {{\n{value_check}        return client.send(encode{stem}({arg_names}), options)\n    }}\n", function.signature()));
            }
        }
    }
    for (event, suffix) in ir
        .events
        .iter()
        .zip(suffixes(ir.events.iter().map(|e| e.name.as_str())))
    {
        let stem = format!("{}{}", exported(&event.name), suffix);
        let has_hashes = event
            .inputs
            .iter()
            .any(|p| p.indexed && indexed_hash(&p.ty));
        let result_class = if has_hashes {
            format!("Decoded{stem}Event")
        } else {
            format!("{stem}Event")
        };
        if has_hashes {
            let field_names = param_names(event.inputs.iter().map(|p| {
                (
                    p.name.as_str(),
                    &p.ty,
                    kotlin
                        .registry
                        .name_of_type(&p.ty, p.internal_type.as_deref()),
                )
            }));
            out.push_str(&format!("\n    /** Decoded `{}` log; indexed dynamic fields hold their topic hash. */\n    data class {result_class}(\n", event.signature()));
            for (p, name) in event.inputs.iter().zip(&field_names) {
                let ty = if p.indexed && indexed_hash(&p.ty) {
                    "String".to_string()
                } else {
                    kotlin.field_type(&p.ty, p.internal_type.as_deref(), imports)
                };
                out.push_str(&format!("        val {}: {ty},\n", escape_keyword(name)));
            }
            out.push_str("    )\n");
        }
        let topic_check = if event.anonymous {
            String::new()
        } else {
            format!(
                "        if (log.topics.firstOrNull()?.equals({}, ignoreCase = true) != true) return null\n",
                kotlin_string(&event.topic0().to_string())
            )
        };
        out.push_str(&format!("\n    /** Decodes a matching `{}` log, or returns null. */\n    @Suppress(\"UNCHECKED_CAST\")\n    fun decode{stem}Event(log: org.web3j.protocol.core.methods.response.Log): {result_class}? {{\n{topic_check}        val indexed = log.topics.drop({})\n        if (indexed.size != {}) return null\n", event.signature(), if event.anonymous {0} else {1}, event.inputs.iter().filter(|p| p.indexed).count()));
        let nonindexed = event
            .inputs
            .iter()
            .filter(|p| !p.indexed)
            .collect::<Vec<_>>();
        let data_layouts = nonindexed
            .iter()
            .map(|p| layout::expression(&p.ty, p.internal_type.as_deref(), kotlin, imports))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("        val values = decodeLayout(log.data, AbiTuple(listOf({data_layouts})) {{ it }}) as List<Any>\n        require(values.size == {}) {{ \"incorrect event data arity\" }}\n", nonindexed.len()));
        let mut indexed_i = 0;
        let mut data_i = 0;
        let mut exprs = Vec::new();
        for p in &event.inputs {
            if p.indexed {
                if indexed_hash(&p.ty) {
                    exprs.push(format!("indexed[{indexed_i}]"));
                    indexed_i += 1;
                    continue;
                }
                let topic_ref = type_ref(&p.ty, p.internal_type.as_deref(), kotlin, true, imports);
                let raw = format!(
                    "org.web3j.abi.FunctionReturnDecoder.decodeIndexedValue(indexed[{indexed_i}], {topic_ref})"
                );
                exprs.push(decoded(
                    &raw,
                    &p.ty,
                    p.internal_type.as_deref(),
                    kotlin,
                    imports,
                ));
                indexed_i += 1;
            } else {
                let ty = kotlin.field_type(&p.ty, p.internal_type.as_deref(), imports);
                exprs.push(format!("values[{data_i}] as {ty}"));
                data_i += 1;
            }
        }
        if !exprs.is_empty() {
            out.push_str(&format!(
                "        return {result_class}({})\n",
                exprs.join(", ")
            ));
        } else if event.inputs.is_empty() {
            out.push_str(&format!("        return {stem}Event\n"));
        }
        out.push_str("    }\n");
        let mut topic = if event.anonymous {
            String::new()
        } else {
            format!(
                "        filter.addSingleTopic({})\n",
                kotlin_string(&event.topic0().to_string())
            )
        };
        let indexed_params = event
            .inputs
            .iter()
            .filter(|p| p.indexed)
            .collect::<Vec<_>>();
        let indexed_names = param_names(indexed_params.iter().map(|p| {
            (
                p.name.as_str(),
                &p.ty,
                kotlin
                    .registry
                    .name_of_type(&p.ty, p.internal_type.as_deref()),
            )
        }));
        let mut filter_args = Vec::new();
        for (p, name) in indexed_params.iter().zip(&indexed_names) {
            let arg = escape_keyword(name);
            let ty = if indexed_hash(&p.ty) {
                "String".to_string()
            } else {
                kotlin.field_type(&p.ty, p.internal_type.as_deref(), imports)
            };
            filter_args.push(format!("{arg}: {ty}? = null"));
            let encoded = if indexed_hash(&p.ty) {
                format!(
                    "{arg}.also {{ require(Regex(\"0x[0-9a-fA-F]{{64}}\").matches(it)) {{ \"indexed hash must be 32 bytes\" }} }}"
                )
            } else {
                let value = kotlin.to_web3j(&arg, &p.ty, p.internal_type.as_deref(), imports);
                format!("\"0x\" + org.web3j.abi.TypeEncoder.encode({value})")
            };
            topic.push_str(&format!("        if ({arg} == null) filter.addNullTopic() else filter.addSingleTopic({encoded})\n"));
        }
        let filter_args = if filter_args.is_empty() {
            String::new()
        } else {
            format!(", {}", filter_args.join(", "))
        };
        out.push_str(&format!("\n    /** Creates an SDK filter for `{}` at one contract address. */\n    fun filter{stem}Event(address: String, fromBlock: org.web3j.protocol.core.DefaultBlockParameter, toBlock: org.web3j.protocol.core.DefaultBlockParameter{filter_args}): org.web3j.protocol.core.methods.request.EthFilter {{\n        val filter = org.web3j.protocol.core.methods.request.EthFilter(fromBlock, toBlock, address)\n{topic}        return filter\n    }}\n", event.signature()));
    }
    for (error, suffix) in ir
        .errors
        .iter()
        .zip(suffixes(ir.errors.iter().map(|e| e.name.as_str())))
    {
        let stem = format!("{}{}", exported(&error.name), suffix);
        let layouts = error
            .inputs
            .iter()
            .map(|p| layout::expression(&p.ty, p.internal_type.as_deref(), kotlin, imports))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("\n    /** Decodes the `{}` custom error, or returns null for another selector. */\n    @Suppress(\"UNCHECKED_CAST\")\n    fun decode{stem}Error(data: String): {stem}Error? {{\n        if (!data.startsWith({}, ignoreCase = true)) return null\n        val values = decodeLayout(\"0x\" + data.substring(10), AbiTuple(listOf({layouts})) {{ it }}) as List<Any>\n        require(values.size == {}) {{ \"incorrect custom error arity\" }}\n", error.signature(), kotlin_string(&error.selector().to_string()), error.inputs.len()));
        if error.inputs.is_empty() {
            out.push_str(&format!("        return {stem}Error\n"));
        } else {
            let exprs = error
                .inputs
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let ty = kotlin.field_type(&p.ty, p.internal_type.as_deref(), imports);
                    format!("values[{i}] as {ty}")
                })
                .collect::<Vec<_>>();
            out.push_str(&format!(
                "        return {stem}Error({})\n",
                exprs.join(", ")
            ));
        }
        out.push_str("    }\n");
    }
    out
}

fn names(params: &[AbiParam], kotlin: &Kotlin<'_>) -> Vec<String> {
    param_names(params.iter().map(|p| {
        (
            p.name.as_str(),
            &p.ty,
            kotlin
                .registry
                .name_of_type(&p.ty, p.internal_type.as_deref()),
        )
    }))
}

fn type_ref(
    ty: &SolType,
    internal: Option<&str>,
    kotlin: &Kotlin<'_>,
    indexed: bool,
    imports: &mut BTreeSet<&'static str>,
) -> String {
    let class = kotlin.decoder_type(ty, internal, imports);
    match ty {
        SolType::Tuple(components) => {
            let children = components
                .iter()
                .map(|component| {
                    type_ref(
                        &component.ty,
                        component.internal_type.as_deref(),
                        kotlin,
                        false,
                        imports,
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "object : org.web3j.abi.TypeReference<{class}>({indexed}, listOf({children})) {{}}"
            )
        }
        SolType::Array(inner) | SolType::FixedArray(inner, _) => {
            let child = type_ref(inner, internal, kotlin, false, imports);
            format!(
                "object : org.web3j.abi.TypeReference<{class}>({indexed}, listOf({child})) {{}}"
            )
        }
        SolType::Uint(_)
        | SolType::Int(_)
        | SolType::Bool
        | SolType::Address
        | SolType::Bytes
        | SolType::BytesN(_)
        | SolType::StringType => {
            if indexed {
                format!("object : org.web3j.abi.TypeReference<{class}>(true) {{}}")
            } else {
                format!("object : org.web3j.abi.TypeReference<{class}>() {{}}")
            }
        }
    }
}

fn decoded(
    expr: &str,
    ty: &SolType,
    internal: Option<&str>,
    kotlin: &Kotlin<'_>,
    imports: &mut BTreeSet<&'static str>,
) -> String {
    let cast = kotlin.decoder_type(ty, internal, imports);
    Kotlin::from_web3j(&format!("({expr} as {cast})"), ty)
}

fn indexed_hash(ty: &SolType) -> bool {
    matches!(
        ty,
        SolType::StringType
            | SolType::Bytes
            | SolType::Array(_)
            | SolType::FixedArray(_, _)
            | SolType::Tuple(_)
    )
}

fn exceeds_web3j_static_array(ty: &SolType) -> bool {
    match ty {
        SolType::FixedArray(inner, size) => {
            *size == 0 || *size > super::MAX_STATIC_ARRAY || exceeds_web3j_static_array(inner)
        }
        SolType::Array(inner) => exceeds_web3j_static_array(inner),
        SolType::Tuple(components) => components
            .iter()
            .any(|component| exceeds_web3j_static_array(&component.ty)),
        SolType::Uint(_)
        | SolType::Int(_)
        | SolType::Bool
        | SolType::Address
        | SolType::Bytes
        | SolType::BytesN(_)
        | SolType::StringType => false,
    }
}
