//! Java web3j wrapper methods and explicit ABI layout decoding.

use super::{Java, is_dynamic, java_ident, java_string};
use crate::naming::{Scope, exported, overload_indices, param_names, repeat_indices};
use abi_typegen_core::types::{ContractIr, SolType, StateMutability};

pub(super) fn render(ir: &ContractIr, java: &Java<'_>) -> String {
    let mut out = String::from(
        r#"
    /** Gas settings and payable amount for a write transaction. */
    public record TransactionOptions(java.math.BigInteger gasPrice, java.math.BigInteger gasLimit, java.math.BigInteger value) {
        public TransactionOptions(java.math.BigInteger gasPrice, java.math.BigInteger gasLimit) {
            this(gasPrice, gasLimit, java.math.BigInteger.ZERO);
        }
    }

    /** SDK-backed handle for a deployed contract. */
    public static final class Client {
        public final String address;
        public final org.web3j.protocol.Web3j web3j;
        public final org.web3j.tx.TransactionManager transactionManager;
        public Client(String address, org.web3j.protocol.Web3j web3j, org.web3j.tx.TransactionManager transactionManager) {
            this.address = address;
            this.web3j = web3j;
            this.transactionManager = transactionManager;
        }
        public String call(String data) throws java.io.IOException {
            var request = org.web3j.protocol.core.methods.request.Transaction.createEthCallTransaction(transactionManager.getFromAddress(), address, data);
            var response = web3j.ethCall(request, org.web3j.protocol.core.DefaultBlockParameterName.LATEST).send();
            if (response.hasError()) throw new IllegalStateException(response.getError().getMessage());
            if (response.isReverted()) throw new IllegalStateException("contract call reverted: " + response.getRevertReasonEncodedData());
            return response.getValue();
        }
        public org.web3j.protocol.core.methods.response.EthSendTransaction send(String data, TransactionOptions options) throws java.io.IOException {
            if (options.value().signum() < 0) throw new IllegalArgumentException("transaction value must be nonnegative");
            var response = transactionManager.sendTransaction(options.gasPrice(), options.gasLimit(), address, data, options.value(), false);
            if (response.hasError()) throw new IllegalStateException(response.getError().getMessage());
            return response;
        }
    }

    private interface Layout {
        boolean dynamic();
        int headBytes();
        Object read(String hex, int at);
    }
    private record Atom(boolean dynamic, java.util.function.BiFunction<String, Integer, Object> decoder) implements Layout {
        public int headBytes() { return 32; }
        public Object read(String hex, int at) { return decoder.apply(hex, at); }
    }
    private record ArrayLayout(Layout element, Integer fixedSize) implements Layout {
        public boolean dynamic() { return fixedSize == null || element.dynamic(); }
        public int headBytes() { return dynamic() ? 32 : Math.multiplyExact(fixedSize, element.headBytes()); }
        public Object read(String hex, int at) {
            int count = fixedSize == null ? word(hex, at).intValueExact() : fixedSize;
            if (count < 0 || count > hex.length() / 64) throw new IllegalArgumentException("array length exceeds ABI data");
            int base = fixedSize == null ? Math.addExact(at, 32) : at;
            int cursor = base;
            var values = new java.util.ArrayList<Object>(count);
            for (int i = 0; i < count; i++) {
                int target = element.dynamic() ? Math.addExact(base, word(hex, cursor).intValueExact()) : cursor;
                values.add(element.read(hex, target));
                cursor = Math.addExact(cursor, element.headBytes());
            }
            return java.util.List.copyOf(values);
        }
    }
    private record TupleLayout(java.util.List<Layout> fields, java.util.function.Function<java.util.List<Object>, Object> construct) implements Layout {
        public boolean dynamic() { return fields.stream().anyMatch(Layout::dynamic); }
        public int headBytes() {
            if (dynamic()) return 32;
            int total = 0;
            for (var field : fields) total = Math.addExact(total, field.headBytes());
            return total;
        }
        public Object read(String hex, int at) {
            int cursor = at;
            var values = new java.util.ArrayList<Object>(fields.size());
            for (var field : fields) {
                int target = field.dynamic() ? Math.addExact(at, word(hex, cursor).intValueExact()) : cursor;
                values.add(field.read(hex, target));
                cursor = Math.addExact(cursor, field.headBytes());
            }
            return construct.apply(java.util.List.copyOf(values));
        }
    }
    private static java.math.BigInteger word(String hex, int at) {
        int start = Math.multiplyExact(at, 2);
        if (start < 0 || start > hex.length() - 64) throw new IllegalArgumentException("truncated ABI word");
        return new java.math.BigInteger(hex.substring(start, start + 64), 16);
    }
    private static Object decodeLayout(String data, Layout layout) {
        if (!data.startsWith("0x") || data.length() % 2 != 0) throw new IllegalArgumentException("invalid ABI hex");
        return layout.read(data.substring(2), 0);
    }
"#,
    );
    for def in java.registry.defs() {
        let name = &java.tuple_names[&def.name];
        let layouts = def
            .components
            .iter()
            .map(|component| layout_expr(&component.ty, component.internal_type.as_deref(), java))
            .collect::<Vec<_>>()
            .join(", ");
        let args = def
            .components
            .iter()
            .enumerate()
            .map(|(i, c)| {
                format!(
                    "({}) values.get({i})",
                    java.field_type(&c.ty, c.internal_type.as_deref())
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("\n    @SuppressWarnings(\"unchecked\")\n    private static Layout layout{name}() {{ return new TupleLayout(java.util.List.of({layouts}), values -> new {name}({args})); }}\n"));
        out.push_str(&format!("    /** Decodes one `{name}` ABI return value. */\n    public static {name} decode{name}Value(String data) {{\n        var values = (java.util.List<?>) decodeLayout(data, new TupleLayout(java.util.List.of(layout{name}()), v -> v));\n        return ({name}) values.get(0);\n    }}\n"));
    }
    let mut method_scope = Scope::default();
    for (function, index) in ir.functions.iter().zip(overload_indices(&ir.functions)) {
        let stem = method_scope.claim(&format!(
            "{}{}",
            exported(&function.name),
            index.map(|i| i.to_string()).unwrap_or_default()
        ));
        let fields = java.fields(&function.inputs);
        let args = fields
            .iter()
            .map(|f| format!("{} {}", f.ty, f.name))
            .collect::<Vec<_>>()
            .join(", ");
        let names = fields
            .iter()
            .map(|f| f.name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        let sdk_values = function
            .inputs
            .iter()
            .zip(&fields)
            .map(|(p, f)| java.to_sdk(&f.name, &p.ty, p.internal_type.as_deref(), 0))
            .collect::<Vec<_>>()
            .join(", ");
        let refs = function
            .outputs
            .iter()
            .map(|p| type_ref(&p.ty, p.internal_type.as_deref(), java))
            .collect::<Vec<_>>()
            .join(", ");
        let values = if sdk_values.is_empty() {
            "java.util.List.of()".to_string()
        } else {
            format!("java.util.List.of({sdk_values})")
        };
        let output_refs = if refs.is_empty() {
            "java.util.List.of()".to_string()
        } else {
            format!("java.util.List.of({refs})")
        };
        let unsupported_input = function.inputs.iter().any(|p| unsupported_static(&p.ty));
        let constructor = if unsupported_input {
            "throw new UnsupportedOperationException(\"web3j supports static arrays with 1 to 32 elements\");".to_string()
        } else {
            format!(
                "return new org.web3j.abi.datatypes.Function({}, {values}, {output_refs});",
                java_string(&function.name)
            )
        };
        out.push_str(&format!("\n    /** Typed web3j function for `{}`. */\n    public static org.web3j.abi.datatypes.Function function{stem}({args}) {{ {constructor} }}\n",function.signature()));
        out.push_str(&format!("    /** Encodes `{}` calldata without RPC. */\n    public static String encode{stem}({args}) {{ return org.web3j.abi.FunctionEncoder.encode(function{stem}({names})); }}\n",function.signature()));
        let result_ty = match function.outputs.len() {
            0 => "void".to_string(),
            1 => java.field_type(
                &function.outputs[0].ty,
                function.outputs[0].internal_type.as_deref(),
            ),
            _ => format!("{stem}Result"),
        };
        if function.outputs.len() > 1 {
            out.push_str(&java.record(&format!("{stem}Result"), &function.outputs));
        }
        out.push_str(&format!("    /** Decodes `{}` ABI output. */\n    @SuppressWarnings(\"unchecked\")\n    public static {result_ty} decode{stem}Result(String data) {{\n",function.signature()));
        if function.outputs.is_empty() {
            out.push_str("        if (!data.isEmpty() && !data.equals(\"0x\")) throw new IllegalArgumentException(\"unexpected void result\");\n        return;\n");
        } else {
            let layouts = function
                .outputs
                .iter()
                .map(|p| layout_expr(&p.ty, p.internal_type.as_deref(), java))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("        var decoded = (java.util.List<?>) decodeLayout(data, new TupleLayout(java.util.List.of({layouts}), v -> v));\n        if (decoded.size() != {}) throw new IllegalArgumentException(\"incorrect result arity\");\n",function.outputs.len()));
            if function.outputs.len() == 1 {
                out.push_str(&format!("        return ({result_ty}) decoded.get(0);\n"));
            } else {
                let args = function
                    .outputs
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        format!(
                            "({}) decoded.get({i})",
                            java.field_type(&p.ty, p.internal_type.as_deref())
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!("        return new {stem}Result({args});\n"));
            }
        }
        out.push_str("    }\n");
        match function.state_mutability {
            StateMutability::View | StateMutability::Pure => {
                let full_args = if args.is_empty() {
                    "Client client".to_string()
                } else {
                    format!("Client client, {args}")
                };
                let call = if result_ty == "void" {
                    format!("decode{stem}Result(client.call(encode{stem}({names})));")
                } else {
                    format!("return decode{stem}Result(client.call(encode{stem}({names})));")
                };
                out.push_str(&format!("    /** Performs a typed read call. */\n    public static {result_ty} call{stem}({full_args}) throws java.io.IOException {{ {call} }}\n"));
            }
            StateMutability::NonPayable | StateMutability::Payable => {
                let full_args = if args.is_empty() {
                    "Client client, TransactionOptions options".to_string()
                } else {
                    format!("Client client, {args}, TransactionOptions options")
                };
                let check = if function.state_mutability == StateMutability::NonPayable {
                    "if (options.value().signum() != 0) throw new IllegalArgumentException(\"nonpayable function cannot receive value\"); "
                } else {
                    ""
                };
                out.push_str(&format!("    /** Signs and submits a typed write transaction. */\n    public static org.web3j.protocol.core.methods.response.EthSendTransaction send{stem}({full_args}) throws java.io.IOException {{ {check}return client.send(encode{stem}({names}), options); }}\n"));
            }
        }
    }
    for (event, index) in ir
        .events
        .iter()
        .zip(repeat_indices(ir.events.iter().map(|e| e.name.as_str())))
    {
        let stem = format!(
            "{}{}",
            exported(&event.name),
            index.map(|i| i.to_string()).unwrap_or_default()
        );
        let hashes = event
            .inputs
            .iter()
            .any(|p| p.indexed && indexed_hash(&p.ty));
        let result = if hashes {
            format!("Decoded{stem}Event")
        } else {
            format!("{stem}Event")
        };
        if hashes {
            let names = param_names(event.inputs.iter().map(|p| {
                (
                    p.name.as_str(),
                    &p.ty,
                    java.registry
                        .name_of_type(&p.ty, p.internal_type.as_deref()),
                )
            }));
            let fields = event
                .inputs
                .iter()
                .zip(names)
                .map(|(p, name)| {
                    format!(
                        "{} {}",
                        if p.indexed && indexed_hash(&p.ty) {
                            "String".to_string()
                        } else {
                            java.field_type(&p.ty, p.internal_type.as_deref())
                        },
                        java_ident(&name)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("    /** Decoded event; indexed dynamic fields contain their topic hash. */\n    public record {result}({fields}) {{}}\n"));
        }
        out.push_str(&format!("    /** Decodes one `{}` event log, or null for another topic. */\n    @SuppressWarnings(\"unchecked\")\n    public static {result} decode{stem}Event(org.web3j.protocol.core.methods.response.Log log) {{\n",event.signature()));
        if !event.anonymous {
            out.push_str(&format!("        if (log.getTopics().isEmpty() || !log.getTopics().get(0).equalsIgnoreCase({})) return null;\n",java_string(&event.topic0().to_string())));
        }
        let topic_start = if event.anonymous { 0 } else { 1 };
        let indexed_count = event.inputs.iter().filter(|p| p.indexed).count();
        out.push_str(&format!(
            "        if (log.getTopics().size() != {}) return null;\n",
            topic_start + indexed_count
        ));
        let nonindexed = event
            .inputs
            .iter()
            .filter(|p| !p.indexed)
            .collect::<Vec<_>>();
        let layouts = nonindexed
            .iter()
            .map(|p| layout_expr(&p.ty, p.internal_type.as_deref(), java))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("        var values = (java.util.List<?>) decodeLayout(log.getData(), new TupleLayout(java.util.List.of({layouts}), v -> v));\n"));
        let mut topic_i = topic_start;
        let mut data_i = 0;
        let mut args = Vec::new();
        for p in &event.inputs {
            if p.indexed {
                if indexed_hash(&p.ty) {
                    args.push(format!("log.getTopics().get({topic_i})"));
                } else {
                    let sdk = java.sdk_type(&p.ty, p.internal_type.as_deref());
                    let decoded = format!(
                        "org.web3j.abi.FunctionReturnDecoder.decodeIndexedValue(log.getTopics().get({topic_i}), new org.web3j.abi.TypeReference<{sdk}>(true) {{}})"
                    );
                    let native = match p.ty {
                        SolType::BytesN(_) => format!("({sdk}) {decoded}"),
                        _ => format!("(({sdk}) {decoded}).getValue()"),
                    };
                    args.push(native);
                }
                topic_i += 1;
            } else {
                args.push(format!(
                    "({}) values.get({data_i})",
                    java.field_type(&p.ty, p.internal_type.as_deref())
                ));
                data_i += 1;
            }
        }
        out.push_str(&format!(
            "        return new {result}({});\n    }}\n",
            args.join(", ")
        ));
        let mut filter_args = vec![
            "String address".to_string(),
            "org.web3j.protocol.core.DefaultBlockParameter fromBlock".to_string(),
            "org.web3j.protocol.core.DefaultBlockParameter toBlock".to_string(),
        ];
        let indexed: Vec<_> = event.inputs.iter().filter(|p| p.indexed).collect();
        let names = param_names(indexed.iter().map(|p| {
            (
                p.name.as_str(),
                &p.ty,
                java.registry
                    .name_of_type(&p.ty, p.internal_type.as_deref()),
            )
        }));
        let mut filters = String::new();
        if !event.anonymous {
            filters.push_str(&format!(
                "        filter.addSingleTopic({});\n",
                java_string(&event.topic0().to_string())
            ));
        }
        for (p, name) in indexed.iter().zip(names) {
            let name = java_ident(&name);
            let ty = if indexed_hash(&p.ty) {
                "String".to_string()
            } else {
                java.field_type(&p.ty, p.internal_type.as_deref())
            };
            filter_args.push(format!("{ty} {name}"));
            let encoded = if indexed_hash(&p.ty) {
                name.to_string()
            } else {
                format!(
                    "\"0x\" + org.web3j.abi.TypeEncoder.encode({})",
                    java.to_sdk(&name, &p.ty, p.internal_type.as_deref(), 0)
                )
            };
            filters.push_str(&format!("        if ({name} == null) filter.addNullTopic(); else filter.addSingleTopic({encoded});\n"));
        }
        out.push_str(&format!("    /** Builds a positional SDK event filter. */\n    public static org.web3j.protocol.core.methods.request.EthFilter filter{stem}Event({}) {{\n        var filter = new org.web3j.protocol.core.methods.request.EthFilter(fromBlock, toBlock, address);\n{filters}        return filter;\n    }}\n",filter_args.join(", ")));
    }
    for (error, index) in ir
        .errors
        .iter()
        .zip(repeat_indices(ir.errors.iter().map(|e| e.name.as_str())))
    {
        let stem = format!(
            "{}{}",
            exported(&error.name),
            index.map(|i| i.to_string()).unwrap_or_default()
        );
        let layouts = error
            .inputs
            .iter()
            .map(|p| layout_expr(&p.ty, p.internal_type.as_deref(), java))
            .collect::<Vec<_>>()
            .join(", ");
        let args = error
            .inputs
            .iter()
            .enumerate()
            .map(|(i, p)| {
                format!(
                    "({}) values.get({i})",
                    java.field_type(&p.ty, p.internal_type.as_deref())
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("    /** Decodes `{}` custom error, or null for another selector. */\n    @SuppressWarnings(\"unchecked\")\n    public static {stem}Error decode{stem}Error(String data) {{\n        if (!data.regionMatches(true, 0, {}, 0, 10)) return null;\n        var values = (java.util.List<?>) decodeLayout(\"0x\" + data.substring(10), new TupleLayout(java.util.List.of({layouts}), v -> v));\n        return new {stem}Error({args});\n    }}\n",error.signature(),java_string(&error.selector().to_string())));
    }
    out
}

fn type_ref(ty: &SolType, internal: Option<&str>, java: &Java<'_>) -> String {
    format!(
        "new org.web3j.abi.TypeReference<{}>() {{}}",
        java.sdk_type(ty, internal)
    )
}

fn layout_expr(ty: &SolType, internal: Option<&str>, java: &Java<'_>) -> String {
    match ty {
        SolType::Bool
        | SolType::Address
        | SolType::StringType
        | SolType::Uint(_)
        | SolType::Int(_)
        | SolType::Bytes
        | SolType::BytesN(_) => {
            let sdk = java.sdk_type(ty, internal);
            let value = match ty {
                SolType::Bytes | SolType::BytesN(_) => "value".to_string(),
                _ => "value.getValue()".to_string(),
            };
            format!(
                "new Atom({}, (hex, at) -> {{ var value = org.web3j.abi.TypeDecoder.decode(hex, Math.multiplyExact(at, 2), {sdk}.class); return {value}; }})",
                is_dynamic(ty)
            )
        }
        SolType::Array(inner) => format!(
            "new ArrayLayout({}, null)",
            layout_expr(inner, internal, java)
        ),
        SolType::FixedArray(inner, size) => format!(
            "new ArrayLayout({}, {size})",
            layout_expr(inner, internal, java)
        ),
        SolType::Tuple(components) => format!(
            "layout{}()",
            java.tuple_names[java.registry.name(components, internal)]
        ),
    }
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
fn unsupported_static(ty: &SolType) -> bool {
    match ty {
        SolType::FixedArray(inner, size) => *size == 0 || *size > 32 || unsupported_static(inner),
        SolType::Array(inner) => unsupported_static(inner),
        SolType::Tuple(parts) => parts.iter().any(|p| unsupported_static(&p.ty)),
        SolType::Bool
        | SolType::Address
        | SolType::StringType
        | SolType::Uint(_)
        | SolType::Int(_)
        | SolType::Bytes
        | SolType::BytesN(_) => false,
    }
}
