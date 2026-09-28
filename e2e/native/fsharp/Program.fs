module Consumer
open System
open System.Numerics
open Nethereum.Web3
open Contracts.Token

let check condition message = if not condition then failwith message
let contract = bind (Web3("http://127.0.0.1:8545")) "0x0000000000000000000000000000000000000001"
let args: ApproveParams = { Spender = "0x0000000000000000000000000000000000000002"; Amount = 42I }
let data = encodeApprove contract args
check (data.StartsWith("0x095ea7b3") && data.Length = 138) "approve encoding"
check ((decodeBalanceOfResult ("0x" + (42I).ToString("x").PadLeft(64, '0'))).Arg0 = 42I) "result decoding"
decodeInvalidRecipientError "0x9c8d2cd2" |> ignore
printfn "F# generated Nethereum consumer passed"
let word (value: BigInteger) = value.ToString("x").PadLeft(64, '0')
let edge = Contracts.EdgeCases.bind (Web3("http://127.0.0.1:8545")) "0x0000000000000000000000000000000000000001"
let tuple: Contracts.EdgeCases.TupleComplexStruct = { Id = 42I; Owner = args.Spender; Hash = Array.zeroCreate 32; Active = true; Name = "tuple"; Timestamp = 7I }
check ((Contracts.EdgeCases.encodeProcessComplex edge { Input = tuple }).StartsWith("0x")) "tuple encoding"
let nested = Contracts.EdgeCases.decodeNestedArrayResult ("0x" + word 32I + word 1I + word 32I + word 2I + word 1I + word 2I)
check (Seq.toArray nested.Arg0.[0] = [| 1I; 2I |]) "nested array decoding"
let signed = Contracts.EdgeCases.decodeSignedIntsResult ("0x" + String.replicate 192 "f")
check (signed.A = -1I && signed.B = -1I && signed.C = -1I) "signed integer decoding"
check ((encodeDeployment (Web3()) "0x60006000" { Name = "Token"; Symbol = "TKN"; Decimals = 18I }).StartsWith("0x60006000")) "constructor encoding"
let malformedRejected =
    try decodeInvalidRecipientError "0x9c8d2cd200" |> ignore; false
    with :? ArgumentException -> true
check malformedRejected "malformed custom error accepted"
let nonpayableRejected =
    try sendApprove contract (Nethereum.RPC.Eth.DTOs.TransactionInput(Value = Nethereum.Hex.HexTypes.HexBigInteger(1I))) args |> ignore; false
    with :? ArgumentException -> true
check nonpayableRejected "nonpayable value accepted"

let rpcUrl = Environment.GetEnvironmentVariable("ATG_RPC_URL")
if not (isNull rpcUrl) then
    let account = Nethereum.Web3.Accounts.Account(Environment.GetEnvironmentVariable("ATG_PRIVATE_KEY"), BigInteger.Parse(Environment.GetEnvironmentVariable("ATG_CHAIN_ID")))
    let web3 = Web3(account, rpcUrl)
    let token = bind web3 (Environment.GetEnvironmentVariable("ATG_TOKEN_ADDRESS"))
    let receipt hash =
        let timer = System.Diagnostics.Stopwatch.StartNew()
        let mutable result = web3.Eth.Transactions.GetTransactionReceipt.SendRequestAsync(hash).GetAwaiter().GetResult()
        while isNull result && timer.Elapsed.TotalSeconds < 30.0 do
            System.Threading.Thread.Sleep(100)
            result <- web3.Eth.Transactions.GetTransactionReceipt.SendRequestAsync(hash).GetAwaiter().GetResult()
        if isNull result then failwith "receipt timeout"
        result
    let options () = Nethereum.RPC.Eth.DTOs.TransactionInput(From = account.Address, Gas = Nethereum.Hex.HexTypes.HexBigInteger(500000I))
    let amount = BigInteger.One <<< 128
    let hash = (sendMint token (options()) { To = account.Address; Amount = amount }).GetAwaiter().GetResult()
    let minted = receipt hash
    check (minted.Status.Value = 1I) "mint transaction"
    let balance = (callBalanceOf token null { Arg0 = account.Address }).GetAwaiter().GetResult()
    check (balance.Arg0 = amount) "read exact large integer"
    let transfer = decodeTransferEvent token minted.Logs.[0]
    check (transfer.Amount = amount) "event decoding"
    let block = Nethereum.RPC.Eth.DTOs.BlockParameter(minted.BlockNumber)
    let logs = (queryTransfer token (filterTransfer token block block)).GetAwaiter().GetResult()
    check (logs.Count = 1) "historical event query"
    let approval = (sendApprove token (options()) { Spender = account.Address; Amount = 17I }).GetAwaiter().GetResult() |> receipt
    check (approval.Status.Value = 1I) "approve transaction"
    let allowance = (callAllowance token null { Arg0 = account.Address; Arg1 = account.Address }).GetAwaiter().GetResult()
    check (allowance.Arg0 = 17I) "allowance read"
    use artifact = System.Text.Json.JsonDocument.Parse(System.IO.File.ReadAllText("../../foundry-sample/out/Token.sol/Token.json"))
    let bytecode = artifact.RootElement.GetProperty("bytecode").GetProperty("object").GetString()
    let deployment = (deploy web3 bytecode account.Address (Nethereum.Hex.HexTypes.HexBigInteger(4000000I)) (Nethereum.Hex.HexTypes.HexBigInteger(0I)) { Name = "FSharp deploy"; Symbol = "FSD"; Decimals = 9I }).GetAwaiter().GetResult() |> receipt
    check (deployment.Status.Value = 1I) "deployment"
    let deployed = bind web3 deployment.ContractAddress
    check ((callName deployed null (NameParams())).GetAwaiter().GetResult().Arg0 = "FSharp deploy") "deployed contract read"
    printfn "F# Anvil signed writes, exact reads, deployment and event queries passed"
printfn "F# tuples, integers, constructors, errors and optional live transactions passed"
let native = Contracts.NativeCases.bind (Web3()) "0x0000000000000000000000000000000000000001"
let hashTopic = "0x" + String.replicate 64 "a"
let indexedLog = Nethereum.RPC.Eth.DTOs.FilterLog(Topics = [| box "0xbe2c441cb63799e5afca59c017722c418d1279daa00c6d3770e4ce7efac17b3e"; box hashTopic; box hashTopic; box hashTopic |], Data = "0x")
let indexed = Contracts.NativeCases.decodeIndexedReferencesEvent native indexedLog
check (indexed.Label = hashTopic && indexed.Payload = indexed.Label && indexed.Values = indexed.Label) "indexed dynamic references preserve topic hash"
let anonymousLog = Nethereum.RPC.Eth.DTOs.FilterLog(Topics = [||], Data = "0x" + word 32I + word 1I + "aa" + String.replicate 62 "0")
let anonymous = Contracts.NativeCases.decodeLoggedEvent native anonymousLog
check (anonymous.Data = [| 0xaauy |]) "anonymous event decode"
printfn "F# indexed hashes and anonymous events passed"
let collisions = Contracts.Collisions.bind (Web3()) "0x0000000000000000000000000000000000000001"
let tupleTopic = "0x" + collisions.GetEvent("StaticTuple").EventABI.Sha3Signature
let tupleLog = Nethereum.RPC.Eth.DTOs.FilterLog(Topics = [| box tupleTopic; box hashTopic |], Data = "0x")
check ((Contracts.Collisions.decodeStaticTupleEvent collisions tupleLog).Pair = hashTopic) "indexed static tuple preserves topic hash"
let anonymousFilter = Contracts.NativeCases.filterLogged native null null
check (isNull anonymousFilter.Topics || anonymousFilter.Topics.Length = 0) "anonymous filter excludes signature topic"
let fixedLog = Nethereum.RPC.Eth.DTOs.FilterLog(Topics = [| box "0x7bc22f66d25cea4277c53524704c189ba0197ff86d1883af04a37ec66fe5aa24"; box hashTopic |], Data = "0x")
let fixedIndexed = Contracts.NativeCases.decodeIndexedFixedEvent native fixedLog
check (fixedIndexed.Pair = hashTopic) "indexed fixed array preserves topic hash"
if not (isNull rpcUrl) then
    let account = Nethereum.Web3.Accounts.Account(Environment.GetEnvironmentVariable("ATG_PRIVATE_KEY"), BigInteger.Parse(Environment.GetEnvironmentVariable("ATG_CHAIN_ID")))
    let web3 = Web3(account, rpcUrl)
    // Constructor-only EVM fixture emits fixed-array and static-tuple hashes,
    // then an anonymous bytes event. The runtime is empty.
    let emitHash topic = "7f" + hashTopic.Substring(2) + "7f" + topic + "60006000a2"
    let bytecode = "0x" + emitHash "7bc22f66d25cea4277c53524704c189ba0197ff86d1883af04a37ec66fe5aa24" + emitHash (tupleTopic.Substring(2)) + "602060005260016020527f" + "aa" + String.replicate 62 "0" + "60405260606000a060006000f3"
    let transaction = (Contracts.Collisions.deploy web3 bytecode account.Address (Nethereum.Hex.HexTypes.HexBigInteger(500000I)) (Nethereum.Hex.HexTypes.HexBigInteger(0I)) (Contracts.Collisions.ConstructorParams())).GetAwaiter().GetResult()
    let timer = System.Diagnostics.Stopwatch.StartNew()
    let mutable receipt = web3.Eth.Transactions.GetTransactionReceipt.SendRequestAsync(transaction).GetAwaiter().GetResult()
    while isNull receipt && timer.Elapsed.TotalSeconds < 30.0 do
        System.Threading.Thread.Sleep(100)
        receipt <- web3.Eth.Transactions.GetTransactionReceipt.SendRequestAsync(transaction).GetAwaiter().GetResult()
    check (not (isNull receipt) && receipt.Status.Value = 1I) "event fixture deployment"
    let source = Contracts.NativeCases.bind web3 receipt.ContractAddress
    let tupleSource = Contracts.Collisions.bind web3 receipt.ContractAddress
    let block = Nethereum.RPC.Eth.DTOs.BlockParameter(receipt.BlockNumber)
    let fixedEvents = (Contracts.NativeCases.queryIndexedFixed source (Contracts.NativeCases.filterIndexedFixed source block block)).GetAwaiter().GetResult()
    check (fixedEvents.Count = 1 && fixedEvents.[0].Event.Pair = hashTopic) "live fixed-array query preserves canonical signature"
    let tupleEvents = (Contracts.Collisions.queryStaticTuple tupleSource (Contracts.Collisions.filterStaticTuple tupleSource block block)).GetAwaiter().GetResult()
    check (tupleEvents.Count = 1 && tupleEvents.[0].Event.Pair = hashTopic) "live static-tuple query preserves canonical signature"
    let anonymousEvents = (Contracts.NativeCases.queryLogged source (Contracts.NativeCases.filterLogged source block block)).GetAwaiter().GetResult()
    check (anonymousEvents.Count = 1 && anonymousEvents.[0].Event.Data = [|0xaauy|]) "live anonymous query excludes signature topic"
    printfn "F# static indexed hashes and anonymous queries passed on Anvil"
