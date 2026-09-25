using System.Numerics;
using System.Text.Json;
using Contracts;
using Nethereum.Hex.HexTypes;
using Nethereum.RPC.Eth.DTOs;
using Nethereum.Web3;
using Nethereum.Web3.Accounts;

static void Check(bool condition, string message)
{
    if (!condition) throw new Exception(message);
}

var offline = new TokenBinding(new Web3("http://127.0.0.1:8545"), "0x0000000000000000000000000000000000000001");
var approve = new TokenApproveParams { Spender = "0x0000000000000000000000000000000000000002", Amount = 42 };
var calldata = offline.EncodeApprove(approve);
Check(calldata.StartsWith("0x095ea7b3", StringComparison.OrdinalIgnoreCase) && calldata.Length == 138, "typed approve calldata");
var balanceOutput = "0x" + new BigInteger(42).ToString("x").PadLeft(64, '0');
Check(offline.DecodeBalanceOfResult(balanceOutput).Arg0 == 42, "typed uint256 output decode");
Check(offline.DecodeInvalidRecipientError("0x9c8d2cd2") != null, "zero-argument custom error decode");
try
{
    offline.DecodeInvalidRecipientError("0x9c8d2cd200");
    throw new Exception("malformed custom error accepted");
}
catch (ArgumentException) { }
Check(TokenBinding.EncodeDeployment(new Web3("http://127.0.0.1:8545"), "0x60006000", new TokenConstructorParams { Name = "Token", Symbol = "TKN", Decimals = 18 }).StartsWith("0x60006000"), "typed constructor encoding");
Check(offline.FilterTransferByTopics(new[] { approve.Spender }).Topics.Length == 3, "typed indexed event filter");
try
{
    _ = offline.ApproveAsync(approve, new TransactionInput { Value = new HexBigInteger(1) });
    throw new Exception("nonpayable value accepted");
}
catch (ArgumentException) { }

var edge = new EdgeCasesBinding(new Web3("http://127.0.0.1:8545"), "0x0000000000000000000000000000000000000001");
Check(edge.EncodeProcessComplex(new EdgeCasesProcessComplexParams { Input = new EdgeCasesComplexStruct {
    Id = 42, Owner = approve.Spender, Hash = new byte[32], Active = true, Name = "tuple", Timestamp = 7
} }).StartsWith("0x"), "tuple calldata encoding");
Check(edge.DecodeNestedArrayResult("0x" + Word(32) + Word(1) + Word(32) + Word(2) + Word(1) + Word(2)).Arg0[0].SequenceEqual(new BigInteger[] { 1, 2 }), "nested array result decoding");

var rpcUrl = Environment.GetEnvironmentVariable("ATG_RPC_URL");
if (rpcUrl != null)
{
    var privateKey = Environment.GetEnvironmentVariable("ATG_PRIVATE_KEY") ?? throw new Exception("missing private key");
    var chainId = BigInteger.Parse(Environment.GetEnvironmentVariable("ATG_CHAIN_ID") ?? throw new Exception("missing chain ID"));
    var tokenAddress = Environment.GetEnvironmentVariable("ATG_TOKEN_ADDRESS") ?? throw new Exception("missing token address");
    var account = new Account(privateKey, chainId);
    var web3 = new Web3(account, rpcUrl);
    var token = new TokenBinding(web3, tokenAddress);
    var amount = BigInteger.One << 128;
    var mintHash = await token.MintAsync(new TokenMintParams { To = account.Address, Amount = amount }, new TransactionInput { From = account.Address, Gas = new HexBigInteger(500000) });
    var mintReceipt = await Receipt(web3, mintHash);
    Check(mintReceipt.Status.Value == 1, "mint transaction reverted");
    var balance = await token.BalanceOfAsync(new TokenBalanceOfParams { Arg0 = account.Address });
    Check(balance.Arg0 == amount, "typed RPC balance");
    Check(mintReceipt.Logs.Length == 1, "missing mint event");
    var transfer = token.DecodeTransferEvent((FilterLog)mintReceipt.Logs[0]);
    Check(transfer.To.Equals(account.Address, StringComparison.OrdinalIgnoreCase) && transfer.Amount == amount, "typed transfer event decode");
    var filtered = await token.QueryTransferAsync(token.FilterTransfer(new BlockParameter(mintReceipt.BlockNumber), new BlockParameter(mintReceipt.BlockNumber)));
    Check(filtered.Count == 1, "historical event filter");
    var approveHash = await token.ApproveAsync(new TokenApproveParams { Spender = account.Address, Amount = 17 }, new TransactionInput { From = account.Address, Gas = new HexBigInteger(500000) });
    var approveReceipt = await Receipt(web3, approveHash);
    Check(approveReceipt.Status.Value == 1, "approve transaction reverted");
    var allowance = await token.AllowanceAsync(new TokenAllowanceParams { Arg0 = account.Address, Arg1 = account.Address });
    Check(allowance.Arg0 == 17, "typed RPC allowance");
    Check(token.DecodeApprovalEvent((FilterLog)approveReceipt.Logs[0]).Amount == 17, "typed approval event decode");
    using var artifact = JsonDocument.Parse(File.ReadAllText("../../foundry-sample/out/Token.sol/Token.json"));
    var bytecode = artifact.RootElement.GetProperty("bytecode").GetProperty("object").GetString() ?? throw new Exception("missing bytecode");
    var deployHash = await TokenBinding.DeployAsync(web3, bytecode, account.Address, new HexBigInteger(4_000_000), new HexBigInteger(0), new TokenConstructorParams { Name = "CSharp deploy", Symbol = "CSD", Decimals = 9 });
    var deployReceipt = await Receipt(web3, deployHash);
    Check(deployReceipt.Status.Value == 1 && !string.IsNullOrEmpty(deployReceipt.ContractAddress), "typed deployment reverted");
    var deployed = new TokenBinding(web3, deployReceipt.ContractAddress);
    Check((await deployed.NameAsync(new TokenNameParams())).Arg0 == "CSharp deploy", "typed deployed contract read");
}

Console.WriteLine("C# generated Nethereum consumer passed");

static async Task<TransactionReceipt> Receipt(Web3 web3, string hash)
{
    using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(20));
    while (!timeout.IsCancellationRequested)
    {
        var receipt = await web3.Eth.Transactions.GetTransactionReceipt.SendRequestAsync(hash);
        if (receipt != null) return receipt;
        await Task.Delay(200, timeout.Token);
    }
    throw new TimeoutException($"transaction {hash} was not mined within 20 seconds");
}

static string Word(int value) => value.ToString("x").PadLeft(64, '0');
