//! SDK-backed callable Go bindings layered on top of the metadata renderer.

use super::{
    Imports, Scope, TupleRegistry, go_string, render_struct, sol_type_to_go, struct_fields,
};
use abi_typegen_core::types::{ContractIr, StateMutability};
use std::collections::HashMap;

pub(super) struct Context<'a> {
    pub ir: &'a ContractIr,
    pub registry: &'a TupleRegistry,
    pub tuple_names: &'a HashMap<String, String>,
    pub scope: &'a mut Scope,
    pub imports: &'a mut Imports,
    pub function_names: &'a [String],
    pub function_params: &'a [Option<String>],
    pub event_names: &'a [String],
    pub event_types: &'a [String],
    pub error_names: &'a [String],
    pub error_types: &'a [String],
    pub constructor_params: Option<&'a str>,
}

pub(super) fn render(context: Context<'_>) -> String {
    let Context {
        ir,
        registry,
        tuple_names,
        scope,
        imports,
        function_names,
        function_params,
        event_names,
        event_types,
        error_names,
        error_types,
        constructor_params,
    } = context;
    let contract = crate::naming::exported(&ir.name);
    let binding = scope.claim(&format!("{contract}Binding"));
    let mut out = format!(
        "// {binding} provides typed calls, transactions, logs, and offline ABI encoding.\n\
         type {binding} struct {{\n\tabi     abi.ABI\n\tbound   *bind.BoundContract\n\tbackend bind.ContractBackend\n\taddress common.Address\n}}\n\n\
         // New{binding} parses the ABI and binds an optional backend. Pass nil for offline encoding and decoding.\n\
         func New{binding}(address common.Address, backend bind.ContractBackend) (*{binding}, error) {{\n\
         \tparsed, err := abi.JSON(strings.NewReader({contract}ABI))\n\
         \tif err != nil {{ return nil, err }}\n\
         \tvar bound *bind.BoundContract\n\
         \tif backend != nil {{ bound = bind.NewBoundContract(address, parsed, backend, backend, backend) }}\n\
         \treturn &{binding}{{abi: parsed, bound: bound, backend: backend, address: address}}, nil\n}}\n\n\
         // ABI returns the parsed contract ABI.\n\
         func (c *{binding}) ABI() abi.ABI {{ return c.abi }}\n\n\
         func (c *{binding}) method(signature string) (abi.Method, error) {{\n\
         \tfor _, method := range c.abi.Methods {{ if method.Sig == signature {{ return method, nil }} }}\n\
         \treturn abi.Method{{}}, fmt.Errorf(\"unknown method %s\", signature)\n}}\n\n\
         func (c *{binding}) event(signature string) (abi.Event, error) {{\n\
         \tfor _, event := range c.abi.Events {{ if event.Sig == signature {{ return event, nil }} }}\n\
         \treturn abi.Event{{}}, fmt.Errorf(\"unknown event %s\", signature)\n}}\n\n\
         func (c *{binding}) customError(signature string) (abi.Error, error) {{\n\
         \tfor _, definition := range c.abi.Errors {{ if definition.Sig == signature {{ return definition, nil }} }}\n\
         \treturn abi.Error{{}}, fmt.Errorf(\"unknown error %s\", signature)\n}}\n\n"
    );

    let constructor_fields = ir
        .constructor
        .as_ref()
        .map(|constructor| {
            struct_fields(
                constructor
                    .inputs
                    .iter()
                    .map(|p| (p.name.as_str(), &p.ty, p.internal_type.as_deref())),
                registry,
                &mut |_, ty, internal| sol_type_to_go(ty, internal, registry, tuple_names, imports),
            )
        })
        .unwrap_or_default();
    let constructor_decl = constructor_params
        .map(|name| format!("params {name}"))
        .unwrap_or_default();
    let constructor_suffix = if constructor_decl.is_empty() {
        String::new()
    } else {
        format!(", {constructor_decl}")
    };
    let constructor_values = constructor_fields
        .iter()
        .map(|field| format!("params.{}", field.name))
        .collect::<Vec<_>>();
    let constructor_args = if constructor_values.is_empty() {
        String::new()
    } else {
        format!(", {}", constructor_values.join(", "))
    };
    let constructor_payable = ir
        .constructor
        .as_ref()
        .is_some_and(|c| matches!(c.state_mutability, StateMutability::Payable));
    let constructor_value_guard = if constructor_payable {
        String::new()
    } else {
        "\tif opts.Value != nil && opts.Value.Sign() != 0 { return common.Address{}, nil, nil, fmt.Errorf(\"constructor is not payable\") }\n".to_string()
    };
    out.push_str(&format!(
        "// EncodeConstructor encodes the constructor arguments without bytecode.\n\
         func (c *{binding}) EncodeConstructor({constructor_decl}) ([]byte, error) {{\n\
         \treturn c.abi.Pack(\"\"{constructor_args})\n}}\n\n\
         // Deploy{binding} deploys caller-supplied bytecode with typed constructor arguments.\n\
         func Deploy{binding}(opts *bind.TransactOpts, backend bind.ContractBackend, bytecode []byte{constructor_suffix}) (common.Address, *types.Transaction, *{binding}, error) {{\n\
         \tif opts == nil || backend == nil {{ return common.Address{{}}, nil, nil, fmt.Errorf(\"transaction options and backend are required\") }}\n\
         \tif len(bytecode) == 0 {{ return common.Address{{}}, nil, nil, fmt.Errorf(\"deployment bytecode is empty\") }}\n{constructor_value_guard}\
         \tparsed, err := abi.JSON(strings.NewReader({contract}ABI))\n\
         \tif err != nil {{ return common.Address{{}}, nil, nil, err }}\n\
         \taddress, tx, _, err := bind.DeployContract(opts, parsed, bytecode, backend{constructor_args})\n\
         \tif err != nil {{ return common.Address{{}}, nil, nil, err }}\n\
         \tbinding, err := New{binding}(address, backend)\n\
         \tif err != nil {{ return common.Address{{}}, nil, nil, err }}\n\
         \treturn address, tx, binding, nil\n}}\n\n"
    ));

    let mut callable_names =
        Scope::with_reserved(["ABI", "DecodeError", "method", "event", "customError"]);
    for ((function, base), params_type) in
        ir.functions.iter().zip(function_names).zip(function_params)
    {
        let base = callable_names.claim(base);
        let signature = go_string(&function.signature());
        let params_decl = params_type
            .as_ref()
            .map(|name| format!("params {name}"))
            .unwrap_or_default();
        let params_suffix = if params_decl.is_empty() {
            String::new()
        } else {
            format!(", {params_decl}")
        };
        let params_arg = if params_type.is_some() { "params" } else { "" };
        let fields = params_type
            .as_ref()
            .map(|_| {
                struct_fields(
                    function
                        .inputs
                        .iter()
                        .map(|p| (p.name.as_str(), &p.ty, p.internal_type.as_deref())),
                    registry,
                    &mut |_, ty, internal| {
                        sol_type_to_go(ty, internal, registry, tuple_names, imports)
                    },
                )
            })
            .unwrap_or_default();
        let input_args = fields
            .iter()
            .map(|field| format!("params.{}", field.name))
            .collect::<Vec<_>>();
        let args = if input_args.is_empty() {
            String::new()
        } else {
            format!(", {}", input_args.join(", "))
        };
        out.push_str(&format!(
            "// Encode{base} encodes calldata for {signature}.\n\
             func (c *{binding}) Encode{base}({params_decl}) ([]byte, error) {{\n\
             \tmethod, err := c.method({signature})\n\
             \tif err != nil {{ return nil, err }}\n\
             \treturn c.abi.Pack(method.Name{args})\n}}\n\n"
        ));
        let mut result_assignments = String::new();
        let result_type = if function.outputs.is_empty() {
            None
        } else {
            let name = scope.claim(&format!("{contract}{base}Result"));
            let fields = struct_fields(
                function
                    .outputs
                    .iter()
                    .map(|p| (p.name.as_str(), &p.ty, p.internal_type.as_deref())),
                registry,
                &mut |_, ty, internal| sol_type_to_go(ty, internal, registry, tuple_names, imports),
            );
            for (index, field) in fields.iter().enumerate() {
                result_assignments.push_str(&format!(
                    "\tout.{name} = *abi.ConvertType(values[{index}], new({ty})).(*{ty})\n",
                    name = field.name,
                    ty = field.ty
                ));
            }
            out.push_str(&format!("// {name} holds the outputs of {signature}.\n"));
            out.push_str(&render_struct(&name, &fields));
            Some(name)
        };
        match &result_type {
            Some(result) => out.push_str(&format!(
                "// Decode{base}Result decodes the return data of {signature}.\n\
                 func (c *{binding}) Decode{base}Result(data []byte) (out {result}, err error) {{\n\
                 \tdefer func() {{\n\
                 \t\tif failure := recover(); failure != nil {{\n\
                 \t\t\terr = fmt.Errorf(\"invalid ABI result: %v\", failure)\n\
                 \t\t}}\n\
                 \t}}()\n\
                 \tmethod, err := c.method({signature})\n\
                 \tif err != nil {{ return out, err }}\n\
                 \tvalues, err := method.Outputs.Unpack(data)\n\
                 \tif err != nil {{ return out, err }}\n\
                 \tif len(values) != {output_count} {{ return out, fmt.Errorf(\"result value count mismatch\") }}\n{result_assignments}\
                 \treturn out, nil\n}}\n\n",
                output_count=function.outputs.len(),
            )),
            None => out.push_str(&format!(
                "// Decode{base}Result validates the empty return data of {signature}.\n\
                 func (c *{binding}) Decode{base}Result(data []byte) error {{\n\
                 \tif len(data) != 0 {{ return fmt.Errorf(\"unexpected return data for %s\", {signature}) }}\n\
                 \treturn nil\n}}\n\n"
            )),
        }
        match function.state_mutability {
            StateMutability::Pure | StateMutability::View => {
                if let Some(result) = &result_type {
                    out.push_str(&format!(
                        "// {base} calls {signature} on the bound backend.\n\
                         func (c *{binding}) {base}(opts *bind.CallOpts{params_suffix}) ({result}, error) {{\n\
                         \tvar zero {result}\n\
                         \tif c.bound == nil {{ return zero, fmt.Errorf(\"{binding} has no backend\") }}\n\
                         \tinput, err := c.Encode{base}({params_arg})\n\
                         \tif err != nil {{ return zero, err }}\n\
                         \toutput, err := c.bound.CallRaw(opts, input)\n\
                         \tif err != nil {{ return zero, err }}\n\
                         \treturn c.Decode{base}Result(output)\n}}\n\n"
                    ));
                } else {
                    out.push_str(&format!(
                        "// {base} calls {signature} on the bound backend.\n\
                         func (c *{binding}) {base}(opts *bind.CallOpts{params_suffix}) error {{\n\
                         \tif c.bound == nil {{ return fmt.Errorf(\"{binding} has no backend\") }}\n\
                         \tinput, err := c.Encode{base}({params_arg})\n\
                         \tif err != nil {{ return err }}\n\
                         \toutput, err := c.bound.CallRaw(opts, input)\n\
                         \tif err != nil {{ return err }}\n\
                         \treturn c.Decode{base}Result(output)\n}}\n\n"
                    ));
                }
            }
            StateMutability::NonPayable | StateMutability::Payable => {
                let value_guard = if matches!(
                    function.state_mutability,
                    StateMutability::NonPayable
                ) {
                    format!(
                        "\tif opts.Value != nil && opts.Value.Sign() != 0 {{ return nil, fmt.Errorf(\"%s is not payable\", {signature}) }}\n"
                    )
                } else {
                    String::new()
                };
                out.push_str(&format!(
                "// {base} submits {signature}; opts supplies signer, gas, and optional value.\n\
                 func (c *{binding}) {base}(opts *bind.TransactOpts{params_suffix}) (*types.Transaction, error) {{\n\
                 \tif c.bound == nil {{ return nil, fmt.Errorf(\"{binding} has no backend\") }}\n\
                 \tif opts == nil {{ return nil, fmt.Errorf(\"transaction options are required\") }}\n{value_guard}\
                 \tmethod, err := c.method({signature})\n\
                 \tif err != nil {{ return nil, err }}\n\
                 \treturn c.bound.Transact(opts, method.Name{args})\n}}\n\n"
                ));
            }
        }
    }

    let mut event_methods = Scope::default();
    for ((event, base), event_type) in ir.events.iter().zip(event_names).zip(event_types) {
        let base = event_methods.claim(base);
        let signature = go_string(&event.signature());
        let fields = struct_fields(
            event
                .inputs
                .iter()
                .map(|p| (p.name.as_str(), &p.ty, p.internal_type.as_deref())),
            registry,
            &mut |index, ty, internal| {
                if event.inputs[index].indexed && super::indexed_topic_is_hash(ty) {
                    "common.Hash".to_string()
                } else {
                    sol_type_to_go(ty, internal, registry, tuple_names, imports)
                }
            },
        );
        let mut decode_fields = String::new();
        let mut value_index = 0;
        for (param, field) in event.inputs.iter().zip(&fields) {
            if !param.indexed {
                decode_fields.push_str(&format!(
                    "\t\tout.{name} = *abi.ConvertType(values[{value_index}], new({ty})).(*{ty})\n",
                    name = field.name,
                    ty = field.ty
                ));
                value_index += 1;
            }
        }
        let topic_offset = if event.anonymous { 0 } else { 1 };
        let indexed_count = event.inputs.iter().filter(|param| param.indexed).count();
        let mut topic_fields = String::new();
        let mut topic_index = topic_offset;
        for (input_index, (param, field)) in event.inputs.iter().zip(&fields).enumerate() {
            if !param.indexed {
                continue;
            }
            if matches!(param.ty, abi_typegen_core::types::SolType::Tuple(_)) {
                topic_fields.push_str(&format!(
                    "\tout.{} = log.Topics[{topic_index}]\n",
                    field.name
                ));
            } else {
                topic_fields.push_str(&format!("\tindexed = append(indexed, definition.Inputs[{input_index}])\n\tindexedTopics = append(indexedTopics, log.Topics[{topic_index}])\n"));
            }
            topic_index += 1;
        }
        let topic_check = if event.anonymous {
            String::new()
        } else {
            format!(
                "\tif len(log.Topics) == 0 || log.Topics[0] != definition.ID {{ return out, fmt.Errorf(\"event signature mismatch for %s\", {signature}) }}\n"
            )
        };
        out.push_str(&format!(
            "// Decode{base}Event decodes a {signature} log.\n\
             func (c *{binding}) Decode{base}Event(log types.Log) (out {event_type}, err error) {{\n\
             \tdefer func() {{\n\
             \t\tif failure := recover(); failure != nil {{\n\
             \t\t\terr = fmt.Errorf(\"invalid ABI event: %v\", failure)\n\
             \t\t}}\n\
             \t}}()\n\
             \tdefinition, err := c.event({signature})\n\
             \tif err != nil {{ return out, err }}\n{topic_check}\
             \tif len(log.Topics) != {topic_count} {{ return out, fmt.Errorf(\"event topic count mismatch for %s\", {signature}) }}\n\
             \tif len(definition.Inputs.NonIndexed()) > 0 {{\n\
             \t\tvalues, err := definition.Inputs.NonIndexed().Unpack(log.Data)\n\
             \t\tif err != nil {{ return out, err }}\n\
             \t\tif len(values) != {value_index} {{ return out, fmt.Errorf(\"event value count mismatch\") }}\n{decode_fields}\
             \t}} else if len(log.Data) != 0 {{ return out, fmt.Errorf(\"unexpected event data\") }}\n\
             \tvar indexed abi.Arguments\n\
             \tvar indexedTopics []common.Hash\n{topic_fields}\
             \tif err := abi.ParseTopics(&out, indexed, indexedTopics); err != nil {{ return out, err }}\n\
             \treturn out, nil\n}}\n\n", topic_count=indexed_count + topic_offset
        ));
        let sig_topic = if event.anonymous {
            String::new()
        } else {
            "\tqueries = append([][]any{{definition.ID}}, queries...)\n".to_string()
        };
        out.push_str(&format!(
            "// Filter{base} retrieves and decodes {signature} logs. Indexed filters follow ABI order.\n\
             func (c *{binding}) Filter{base}(ctx context.Context, from, to *big.Int, indexed ...[]any) ([]{event_type}, error) {{\n\
             \tif c.backend == nil {{ return nil, fmt.Errorf(\"{binding} has no backend\") }}\n\
             \tdefinition, err := c.event({signature})\n\
             \tif err != nil {{ return nil, err }}\n\
             \tcount := 0\n\
             \tfor _, arg := range definition.Inputs {{ if arg.Indexed {{ count++ }} }}\n\
             \tif len(indexed) > count {{ return nil, fmt.Errorf(\"too many indexed filters for %s\", {signature}) }}\n\
             \tqueries := indexed\n{sig_topic}\
             \ttopics, err := abi.MakeTopics(queries...)\n\
             \tif err != nil {{ return nil, err }}\n\
             \tlogs, err := c.backend.FilterLogs(ctx, ethereum.FilterQuery{{Addresses: []common.Address{{c.address}}, Topics: topics, FromBlock: from, ToBlock: to}})\n\
             \tif err != nil {{ return nil, err }}\n\
             \tout := make([]{event_type}, 0, len(logs))\n\
             \tfor _, log := range logs {{\n\
             \t\tdecoded, err := c.Decode{base}Event(log)\n\
             \t\tif err != nil {{ return nil, err }}\n\
             \t\tout = append(out, decoded)\n\
             \t}}\n\
             \treturn out, nil\n}}\n\n"
        ));
        out.push_str(&format!(
            "// Watch{base} subscribes to new {signature} logs and decodes them. Call Unsubscribe on the returned subscription when done.\n\
             func (c *{binding}) Watch{base}(ctx context.Context, indexed ...[]any) (<-chan {event_type}, event.Subscription, error) {{\n\
             \tif c.backend == nil {{ return nil, nil, fmt.Errorf(\"{binding} has no backend\") }}\n\
             \tdefinition, err := c.event({signature})\n\
             \tif err != nil {{ return nil, nil, err }}\n\
             \tcount := 0\n\
             \tfor _, arg := range definition.Inputs {{ if arg.Indexed {{ count++ }} }}\n\
             \tif len(indexed) > count {{ return nil, nil, fmt.Errorf(\"too many indexed filters for %s\", {signature}) }}\n\
             \tqueries := indexed\n{sig_topic}\
             \ttopics, err := abi.MakeTopics(queries...)\n\
             \tif err != nil {{ return nil, nil, err }}\n\
             \tlogs := make(chan types.Log)\n\
             \tupstream, err := c.backend.SubscribeFilterLogs(ctx, ethereum.FilterQuery{{Addresses: []common.Address{{c.address}}, Topics: topics}}, logs)\n\
             \tif err != nil {{ return nil, nil, err }}\n\
             \tevents := make(chan {event_type})\n\
             \tsubscription := event.NewSubscription(func(quit <-chan struct{{}}) error {{\n\
             \t\tdefer upstream.Unsubscribe()\n\
             \t\tdefer close(events)\n\
             \t\tfor {{\n\
             \t\t\tselect {{\n\
             \t\t\tcase <-quit:\n\
             \t\t\t\treturn nil\n\
             \t\t\tcase <-ctx.Done():\n\
             \t\t\t\treturn ctx.Err()\n\
             \t\t\tcase err, ok := <-upstream.Err():\n\
             \t\t\t\tif !ok {{ return nil }}\n\
             \t\t\t\treturn err\n\
             \t\t\tcase log, ok := <-logs:\n\
             \t\t\t\tif !ok {{ return nil }}\n\
             \t\t\t\tdecoded, err := c.Decode{base}Event(log)\n\
             \t\t\t\tif err != nil {{ return err }}\n\
             \t\t\t\tselect {{\n\
             \t\t\t\tcase events <- decoded:\n\
             \t\t\t\tcase <-quit:\n\
             \t\t\t\t\treturn nil\n\
             \t\t\t\tcase <-ctx.Done():\n\
             \t\t\t\t\treturn ctx.Err()\n\
             \t\t\t\t}}\n\
             \t\t\t}}\n\
             \t\t}}\n\
             \t}})\n\
             \treturn events, subscription, nil\n}}\n\n"
        ));
    }

    let mut error_methods = Scope::default();
    let mut decoded_error_names = Vec::new();
    for ((error, base), error_type) in ir.errors.iter().zip(error_names).zip(error_types) {
        let base = error_methods.claim(base);
        decoded_error_names.push(base.clone());
        let signature = go_string(&error.signature());
        let fields = struct_fields(
            error
                .inputs
                .iter()
                .map(|p| (p.name.as_str(), &p.ty, p.internal_type.as_deref())),
            registry,
            &mut |_, ty, internal| sol_type_to_go(ty, internal, registry, tuple_names, imports),
        );
        let mut assignments = String::new();
        for (index, field) in fields.iter().enumerate() {
            assignments.push_str(&format!(
                "\tout.{name} = *abi.ConvertType(values[{index}], new({ty})).(*{ty})\n",
                name = field.name,
                ty = field.ty
            ));
        }
        out.push_str(&format!(
            "// Decode{base}Error decodes a {signature} revert payload, including its selector.\n\
             func (c *{binding}) Decode{base}Error(raw []byte) (out {error_type}, err error) {{\n\
             \tdefer func() {{\n\
             \t\tif failure := recover(); failure != nil {{\n\
             \t\t\terr = fmt.Errorf(\"invalid ABI error: %v\", failure)\n\
             \t\t}}\n\
             \t}}()\n\
             \tdefinition, err := c.customError({signature})\n\
             \tif err != nil {{ return out, err }}\n\
             \tif len(raw) < 4 || !bytes.Equal(raw[:4], definition.ID[:4]) {{ return out, fmt.Errorf(\"error selector mismatch for %s\", {signature}) }}\n\
             \tvalues, err := definition.Inputs.Unpack(raw[4:])\n\
             \tif err != nil {{ return out, err }}\n\
             \tif len(values) != {input_count} {{ return out, fmt.Errorf(\"error value count mismatch\") }}\n{assignments}\
             \treturn out, nil\n}}\n\n"
            , input_count=error.inputs.len()
        ));
    }
    out.push_str(&format!(
        "// DecodeError identifies and decodes a declared custom error.\n\
         func (c *{binding}) DecodeError(raw []byte) (any, error) {{\n\
         \tif len(raw) < 4 {{ return nil, fmt.Errorf(\"revert payload is shorter than a selector\") }}\n"
    ));
    for (error, base) in ir.errors.iter().zip(&decoded_error_names) {
        let signature = go_string(&error.signature());
        out.push_str(&format!(
            "\tif definition, err := c.customError({signature}); err == nil && bytes.Equal(raw[:4], definition.ID[:4]) {{ return c.Decode{base}Error(raw) }}\n"
        ));
    }
    out.push_str("\treturn nil, fmt.Errorf(\"unknown custom error selector %x\", raw[:4])\n}\n\n");
    expand_inline_controls(&out)
}

/// Expands compact generated control lines so the output is already gofmt-clean.
fn expand_inline_controls(source: &str) -> String {
    let mut out = String::new();
    for line in source.lines() {
        expand_control_line(line, &mut out);
    }
    out
}

fn expand_control_line(line: &str, out: &mut String) {
    let trimmed = line.trim_start_matches('\t');
    let indent = &line[..line.len() - trimmed.len()];
    if (trimmed.starts_with("if ")
        || trimmed.starts_with("for ")
        || trimmed.starts_with("} else if "))
        && let Some(open) = trimmed.find(" { ")
        && trimmed.ends_with(" }")
    {
        out.push_str(indent);
        out.push_str(&trimmed[..open + 2]);
        out.push('\n');
        let inner = &trimmed[open + 3..trimmed.len() - 2];
        expand_control_line(&format!("{indent}\t{inner}"), out);
        out.push_str(indent);
        out.push_str("}\n");
    } else {
        out.push_str(line);
        out.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use abi_typegen_core::parser::parse_artifact;

    #[test]
    fn indexed_tuple_topics_are_copied_without_sdk_tuple_reconstruction() {
        let ir = parse_artifact(
            "IndexedTuple",
            r#"{"abi":[{"type":"event","name":"Recorded","anonymous":false,"inputs":[{"name":"item","type":"tuple","indexed":true,"components":[{"name":"id","type":"uint256"},{"name":"label","type":"string"}]},{"name":"count","type":"uint256","indexed":false}]}]}"#,
        )
        .expect("valid indexed tuple ABI");
        let generated = super::super::render_go_file_with_wrappers(&ir, "tupleindexed", true);
        assert!(
            generated.contains("out.Item = log.Topics[1]"),
            "{generated}"
        );
        assert!(
            generated.contains("abi.ParseTopics(&out, indexed, indexedTopics)"),
            "{generated}"
        );
    }
}
