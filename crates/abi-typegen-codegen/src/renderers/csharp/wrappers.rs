//! Nethereum-backed C# contract bindings.

use super::{csharp_property_names, function_stems, item_class_stems, item_stems, map_type};
use crate::tuples::TupleRegistry;
use abi_typegen_core::types::{ContractIr, StateMutability};

pub(super) fn render(ir: &ContractIr) -> String {
    let contract = &ir.name;
    let registry = TupleRegistry::new(ir);
    let binding = format!("{contract}Binding");
    let mut out = format!(
        "\n    /// <summary>Typed Nethereum binding for {contract}.</summary>\n    public class {binding}\n    {{\n        private readonly Contract contract;\n\n        public {binding}(Web3 web3, string address)\n        {{\n            if (web3 == null) throw new ArgumentNullException(nameof(web3));\n            contract = web3.Eth.GetContract({contract}AbiMetadata.ABI, address);\n        }}\n\n        /// <summary>The parsed Nethereum contract.</summary>\n        public Contract Contract => contract;\n"
    );
    if let Some(constructor) = &ir.constructor {
        let names =
            csharp_property_names(constructor.inputs.iter().map(|param| param.name.as_str()));
        let values = names
            .iter()
            .map(|name| format!("args.{name}"))
            .collect::<Vec<_>>()
            .join(", ");
        let inputs = if values.is_empty() {
            "Array.Empty<object>()".to_string()
        } else {
            format!("new object[] {{ {values} }}")
        };
        let params = format!("{contract}ConstructorParams");
        let value_guard = if matches!(constructor.state_mutability, StateMutability::NonPayable) {
            "            if (value != null && value.Value != 0) throw new ArgumentException(\"Constructor is not payable\", nameof(value));\n"
        } else {
            ""
        };
        out.push_str(&format!("\n        /// <summary>Encodes deployment bytecode and typed constructor arguments without RPC.</summary>\n        public static string EncodeDeployment(Web3 web3, string bytecode, {params} args)\n        {{\n            if (web3 == null) throw new ArgumentNullException(nameof(web3));\n            if (args == null) throw new ArgumentNullException(nameof(args));\n            return web3.Eth.DeployContract.GetData(bytecode, {contract}AbiMetadata.ABI, {inputs});\n        }}\n\n        /// <summary>Deploys caller-supplied bytecode with typed constructor arguments.</summary>\n        public static Task<string> DeployAsync(Web3 web3, string bytecode, string from, HexBigInteger gas, HexBigInteger value, {params} args)\n        {{\n            if (web3 == null) throw new ArgumentNullException(nameof(web3));\n            if (args == null) throw new ArgumentNullException(nameof(args));\n{value_guard}            return web3.Eth.DeployContract.SendRequestAsync({contract}AbiMetadata.ABI, bytecode, from, gas, value, {inputs});\n        }}\n"));
    }
    for (function, stem) in ir.functions.iter().zip(function_stems(ir)) {
        let params = format!("{contract}{stem}Params");
        let result = format!("{contract}{stem}Result");
        let signature = function.signature();
        let selector = format!(
            "0x{}",
            function
                .selector()
                .as_slice()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
        let field_names =
            csharp_property_names(function.inputs.iter().map(|param| param.name.as_str()));
        let values = field_names
            .iter()
            .map(|name| format!("args.{name}"))
            .collect::<Vec<_>>()
            .join(", ");
        let inputs = if values.is_empty() {
            "Array.Empty<object>()".to_string()
        } else {
            format!("new object[] {{ {values} }}")
        };
        out.push_str(&format!(
            "\n        /// <summary>Encodes calldata for {signature} without an RPC call.</summary>\n        public string Encode{stem}({params} args)\n        {{\n            if (args == null) throw new ArgumentNullException(nameof(args));\n            return contract.GetFunctionBySignature(\"{selector}\").GetData({inputs});\n        }}\n\n        /// <summary>Decodes the output of {signature}.</summary>\n        public {result} Decode{stem}Result(string data)\n        {{\n            return new FunctionCallDecoder().DecodeFunctionOutput(new {result}(), data);\n        }}\n"
        ));
        match function.state_mutability {
            StateMutability::Pure | StateMutability::View => out.push_str(&format!(
                "\n        /// <summary>Calls {signature} through the configured Nethereum client.</summary>\n        public Task<{result}> {stem}Async({params} args, BlockParameter block = null)\n        {{\n            if (args == null) throw new ArgumentNullException(nameof(args));\n            return contract.GetFunctionBySignature(\"{selector}\").CallDeserializingToObjectAsync<{result}>(block, {inputs});\n        }}\n"
            )),
            StateMutability::NonPayable | StateMutability::Payable => {
                let value_guard = if matches!(function.state_mutability, StateMutability::NonPayable) {
                    "            if (options.Value != null && options.Value.Value != 0) throw new ArgumentException(\"Function is not payable\", nameof(options));\n"
                } else { "" };
                out.push_str(&format!(
                    "\n        /// <summary>Sends {signature}; options supply sender, gas, nonce, and payable value.</summary>\n        public Task<string> {stem}Async({params} args, TransactionInput options)\n        {{\n            if (args == null) throw new ArgumentNullException(nameof(args));\n            if (options == null) throw new ArgumentNullException(nameof(options));\n{value_guard}            options.To = contract.Address;\n            return contract.GetFunctionBySignature(\"{selector}\").SendTransactionAsync(options, {inputs});\n        }}\n"
                ));
            }
        }
    }
    let event_methods = item_stems(ir.events.iter().map(|event| event.name.as_str()));
    let event_classes = item_class_stems(ir.events.iter().map(|event| event.name.as_str()));
    for ((event, stem), class_stem) in ir.events.iter().zip(event_methods).zip(event_classes) {
        let dto = format!("{contract}{class_stem}Event");
        let signature = event.signature();
        let topic = format!(
            "0x{}",
            event
                .topic0()
                .as_slice()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
        out.push_str(&format!(
            "\n        /// <summary>Creates a historical filter for {signature}.</summary>\n        public NewFilterInput Filter{stem}(BlockParameter from = null, BlockParameter to = null)\n        {{\n            return contract.GetEventBySignature(\"{topic}\").CreateFilterInput(from, to);\n        }}\n\n        /// <summary>Retrieves and decodes filtered {signature} logs.</summary>\n        public Task<List<EventLog<{dto}>>> Query{stem}Async(NewFilterInput filter)\n        {{\n            return contract.GetEventBySignature(\"{topic}\").GetAllChangesAsync<{dto}>(filter);\n        }}\n\n        /// <summary>Decodes a {signature} log.</summary>\n        public {dto} Decode{stem}Event(FilterLog log)\n        {{\n            var decoded = contract.GetEventBySignature(\"{topic}\").DecodeAllEventsForEvent<{dto}>(new[] {{ log }});\n            if (decoded.Count != 1) throw new ArgumentException(\"Log does not match event\", nameof(log));\n            return decoded[0].Event;\n        }}\n"
        ));
        let indexed = event
            .inputs
            .iter()
            .filter(|param| param.indexed)
            .collect::<Vec<_>>();
        if !indexed.is_empty() && indexed.len() <= 3 {
            let params = indexed
                .iter()
                .enumerate()
                .map(|(index, param)| {
                    format!(
                        "{}[] topic{index} = null",
                        map_type(
                            &param.ty,
                            param.internal_type.as_deref(),
                            &registry,
                            contract
                        )
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            let args = indexed
                .iter()
                .enumerate()
                .map(|(index, param)| {
                    let ty = map_type(
                        &param.ty,
                        param.internal_type.as_deref(),
                        &registry,
                        contract,
                    );
                    format!("topic{index} ?? Array.Empty<{ty}>()")
                })
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("\n        /// <summary>Filters {signature} by indexed argument values.</summary>\n        public NewFilterInput Filter{stem}ByTopics({params}, BlockParameter from = null, BlockParameter to = null)\n        {{\n            return contract.GetEventBySignature(\"{topic}\").CreateFilterInput({args}, from, to);\n        }}\n"));
        }
    }
    let error_methods = item_stems(ir.errors.iter().map(|error| error.name.as_str()));
    let error_classes = item_class_stems(ir.errors.iter().map(|error| error.name.as_str()));
    for ((error, stem), class_stem) in ir.errors.iter().zip(error_methods).zip(error_classes) {
        let dto = format!("{contract}{class_stem}Error");
        let selector = error
            .selector()
            .as_slice()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let length_guard = if error.inputs.is_empty() {
            "            if (data.Length != 10) throw new ArgumentException(\"Unexpected custom error payload\", nameof(data));\n"
        } else {
            ""
        };
        out.push_str(&format!(
            "\n        /// <summary>Decodes the {stem} custom error.</summary>\n        public {dto} Decode{stem}Error(string data)\n        {{\n            if (data == null || !data.StartsWith(\"0x{selector}\", StringComparison.OrdinalIgnoreCase)) throw new ArgumentException(\"Custom error selector mismatch\", nameof(data));\n{length_guard}            return new Nethereum.Contracts.Error(typeof({dto})).DecodeExceptionEncodedData<{dto}>(data);\n        }}\n"
        ));
    }
    out.push_str("    }\n");
    out
}
