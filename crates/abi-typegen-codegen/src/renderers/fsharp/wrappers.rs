//! Typed functions over caller-owned Nethereum clients.
use super::{fields, indexed_hash, quoted};
use abi_typegen_core::types::{AbiParam, ContractIr, StateMutability};

fn values(params: &[AbiParam]) -> String {
    format!(
        "[| {} |]",
        fields(params.iter().map(|p| p.name.as_str()))
            .iter()
            .map(|n| format!("box args.{n}"))
            .collect::<Vec<_>>()
            .join("; ")
    )
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub(super) fn render(
    ir: &ContractIr,
    functions: &[String],
    events: &[String],
    errors: &[String],
) -> String {
    let mut out = String::from(
        "/// Creates a contract using the caller's client and signing configuration.\nlet bind (web3: Web3) (address: string) = web3.Eth.GetContract(abi, address)\n\n",
    );
    if let Some(c) = &ir.constructor {
        let args = values(&c.inputs);
        out.push_str(&format!("/// Encodes supplied deployment bytecode and constructor arguments.\nlet encodeDeployment (web3: Web3) (bytecode: string) (args: ConstructorParams) =\n    web3.Eth.DeployContract.GetData(bytecode, abi, {args})\n\nlet deploy (web3: Web3) (bytecode: string) (sender: string) (gas: HexBigInteger) (value: HexBigInteger) (args: ConstructorParams) =\n"));
        if c.state_mutability == StateMutability::NonPayable {
            out.push_str("    if not (isNull value) && value.Value <> BigInteger.Zero then invalidArg \"value\" \"Constructor is not payable\"\n");
        }
        out.push_str(&format!("    web3.Eth.DeployContract.SendRequestAsync(abi, bytecode, sender, gas, value, {args})\n\n"));
    }
    for (f, n) in ir.functions.iter().zip(functions) {
        let selector = hex(f.selector().as_slice());
        let args = values(&f.inputs);
        out.push_str(&format!("/// Encodes {} without RPC.\nlet encode{n} (contract: Contract) (args: {n}Params) =\n    contract.GetFunctionBySignature(\"0x{selector}\").GetData({args})\n\nlet decode{n}Result (data: string) : {n}Result =\n    Nethereum.ABI.FunctionEncoding.FunctionCallDecoder().DecodeFunctionOutput(Activator.CreateInstance<{n}Result>(), data)\n\n", f.signature()));
        match f.state_mutability {
            StateMutability::Pure | StateMutability::View => out.push_str(&format!("let call{n} (contract: Contract) (block: BlockParameter) (args: {n}Params) =\n    contract.GetFunctionBySignature(\"0x{selector}\").CallDeserializingToObjectAsync<{n}Result>(block, {args})\n\n")),
            StateMutability::Payable | StateMutability::NonPayable => {
                out.push_str(&format!("/// Sends through the client's configured transaction manager.\nlet send{n} (contract: Contract) (options: TransactionInput) (args: {n}Params) =\n    if isNull options then nullArg \"options\"\n"));
                if f.state_mutability == StateMutability::NonPayable { out.push_str("    if not (isNull options.Value) && options.Value.Value <> BigInteger.Zero then invalidArg \"options\" \"Function is not payable\"\n"); }
                out.push_str(&format!("    options.To <- contract.Address\n    contract.GetFunctionBySignature(\"0x{selector}\").SendTransactionAsync(options, {args})\n\n"));
            }
        }
    }
    for (e, n) in ir.events.iter().zip(events) {
        let topic = hex(e.topic0().as_slice());
        let transport = e.inputs.iter().any(|p| p.indexed && indexed_hash(&p.ty));
        let dto = if transport {
            format!("{n}TopicDecoder")
        } else {
            format!("{n}Event")
        };
        if transport {
            let values = fields(e.inputs.iter().map(|p| p.name.as_str()))
                .iter()
                .map(|field| format!("{field} = value.{field}"))
                .collect::<Vec<_>>()
                .join("; ");
            out.push_str(&format!(
                "let private convert{n} (value: {dto}) : {n}Event =\n    {{ {values} }}\n\n"
            ));
        }
        out.push_str(&format!("let filter{n} (contract: Contract) (fromBlock: BlockParameter) (toBlock: BlockParameter) =\n    let filter = contract.GetEventBySignature(\"0x{topic}\").CreateFilterInput(fromBlock, toBlock)\n"));
        if e.anonymous {
            out.push_str("    filter.Topics <- [||]\n");
        }
        out.push_str(&format!(
            "    filter\n\nlet decode{n}Event (contract: Contract) (log: FilterLog) : {n}Event =\n"
        ));
        if transport {
            let initial = fields(e.inputs.iter().map(|p| p.name.as_str()))
                .iter()
                .map(|f| format!("{f} = Unchecked.defaultof<_>"))
                .collect::<Vec<_>>()
                .join("; ");
            out.push_str(&format!("    if not (contract.GetEventBySignature(\"0x{topic}\").IsLogForEvent(log)) then invalidArg \"log\" \"Log does not match event\"\n    let initial: {dto} = {{ {initial} }}\n    let decoded = Nethereum.ABI.FunctionEncoding.EventTopicDecoder({}).DecodeTopics(initial, log.Topics, log.Data)\n    convert{n} decoded\n\n",e.anonymous));
        } else {
            out.push_str(&format!("    let decoded = contract.GetEventBySignature(\"0x{topic}\").DecodeAllEventsForEvent<{dto}>([| log |])\n    if decoded.Count <> 1 then invalidArg \"log\" \"Log does not match event\"\n    decoded.[0].Event\n\n"));
        }
        let log_match = if e.anonymous {
            format!(
                "event.IsLogForEvent(log) && not (isNull log.Topics) && log.Topics.Length = {}",
                e.inputs.iter().filter(|p| p.indexed).count()
            )
        } else {
            "event.IsLogForEvent(log)".into()
        };
        // GetAllChangesAsync<T> derives signatures from DTO properties and also
        // rejects anonymous filters with no topics. Fetch with the caller's SDK
        // and decode against the canonical ABI instead.
        out.push_str(&format!("let query{n} (contract: Contract) (filter: NewFilterInput) =\n    if isNull filter || isNull filter.Address || not (filter.Address |> Array.exists (fun address -> String.Equals(address, contract.Address, StringComparison.OrdinalIgnoreCase))) then invalidArg \"filter\" \"Filter must include this contract address\"\n    task {{\n        let! logs = contract.Eth.Filters.GetLogs.SendRequestAsync(filter)\n        let result = ResizeArray<EventLog<{n}Event>>()\n        let event = contract.GetEventBySignature(\"0x{topic}\")\n        for log in logs do\n            if String.Equals(log.Address, contract.Address, StringComparison.OrdinalIgnoreCase) && {log_match} then\n                result.Add(EventLog<{n}Event>(decode{n}Event contract log, log))\n        return result\n    }}\n\n"));
    }
    for (e, n) in ir.errors.iter().zip(errors) {
        let selector = quoted(&format!("0x{}", hex(e.selector().as_slice())));
        out.push_str(&format!("let decode{n}Error (data: string) : {n}Error =\n    if isNull data || not (data.StartsWith({selector}, StringComparison.OrdinalIgnoreCase)) then invalidArg \"data\" \"Custom error selector mismatch\"\n"));
        if e.inputs.is_empty() {
            out.push_str("    if data.Length <> 10 then invalidArg \"data\" \"Unexpected custom error payload\"\n");
        }
        out.push_str(&format!("    Nethereum.Contracts.Error(typeof<{n}Error>).DecodeExceptionEncodedData<{n}Error>(data)\n\n"));
    }
    out
}
