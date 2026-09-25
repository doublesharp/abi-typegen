using System;
using System.Collections;
using System.Linq;
using System.Numerics;
using System.Threading;
using System.Threading.Tasks;
using AbiTypegen.Unity;
using Contracts;
using Nethereum.JsonRpc.Client;
using Nethereum.RPC.Eth.DTOs;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;

namespace AbiTypegen.Unity.E2E.Tests
{
    public sealed class AnvilPlayModeTests
    {
        private static string RpcUrl => Environment.GetEnvironmentVariable("ATG_RPC_URL");
        private static string ContractAddress => Environment.GetEnvironmentVariable("ATG_TOKEN_ADDRESS");
        private static string PrivateKey => Environment.GetEnvironmentVariable("ATG_PRIVATE_KEY");
        private static string ChainIdText => Environment.GetEnvironmentVariable("ATG_CHAIN_ID");

        [UnityTest, Order(1)]
        public IEnumerator AsyncReadAgainstDisposableAnvil()
        {
            RequireAnvilEnvironment();
            return AsyncEnumerator.Run(ReadInitialBalanceAsync());
        }

        [UnityTest, Order(2)]
        public IEnumerator InjectedDevelopmentSignerSubmitsTransactionAndStateChanges()
        {
            RequireAnvilEnvironment();
            return AsyncEnumerator.Run(MintApproveAndReadBackAsync());
        }

        [UnityTest, Order(3)]
        public IEnumerator JsonRpcErrorsAreSurfaced()
        {
            RequireAnvilEnvironment();
            return AsyncEnumerator.Run(AssertRpcErrorAsync());
        }

        [UnityTest, Order(4)]
        public IEnumerator ContractRevertsAreSurfaced()
        {
            RequireAnvilEnvironment();
            return AsyncEnumerator.Run(AssertTypedContractRevertAsync());
        }

        [UnityTest]
        public IEnumerator BackgroundRpcRequestReturnsToUnityThread()
        {
            RequireAnvilEnvironment();
            var client = new NethereumUnityRpcFactory().CreateClient(new Uri(RpcUrl));
            return AsyncEnumerator.Run(AssertBackgroundRpcAsync(client));
        }

        private static async Task AssertBackgroundRpcAsync(IClient client)
        {
            var version = await Task.Run(() => client.SendRequestAsync<string>("web3_clientVersion"));
            Assert.That(version, Is.Not.Null.And.StartsWith("anvil"));
        }

        private static async Task ReadInitialBalanceAsync()
        {
            var factory = new NethereumUnityRpcFactory();
            var web3 = factory.CreateReadOnlyWeb3(new Uri(RpcUrl));
            var signer = new DevelopmentPrivateKeySigner(new Uri(RpcUrl), PrivateKey, BigInteger.Parse(ChainIdText), factory);
            var binding = new TokenBinding(web3, ContractAddress);

            var go = new GameObject("abi-typegen-anvil-read");
            try
            {
                var host = go.AddComponent<AbiTypegenRequestHost>();
                var result = await host.RunAsync(
                    _ => binding.BalanceOfAsync(new TokenBalanceOfParams { Arg0 = signer.Address }));
                Assert.That(result.Arg0, Is.EqualTo(BigInteger.Zero), "fresh Anvil fixture should begin with zero token balance");
            }
            finally
            {
                UnityEngine.Object.Destroy(go);
            }
        }

        private static async Task MintApproveAndReadBackAsync()
        {
            var factory = new NethereumUnityRpcFactory();
            var endpoint = new Uri(RpcUrl);
            var web3 = factory.CreateReadOnlyWeb3(endpoint);
            var signer = new DevelopmentPrivateKeySigner(endpoint, PrivateKey, BigInteger.Parse(ChainIdText), factory);
            var binding = new TokenBinding(web3, ContractAddress);
            var go = new GameObject("abi-typegen-anvil-write");

            try
            {
                var host = go.AddComponent<AbiTypegenRequestHost>();

                var mintedAmount = (BigInteger.One << 200) + 41;
                var approvedAmount = (BigInteger.One << 129) + 10;
                var mintData = binding.EncodeMint(new TokenMintParams
                {
                    To = signer.Address,
                    Amount = mintedAmount
                });
                var mintInput = AbiTypegenTransaction.Create(
                    ContractAddress,
                    mintData,
                    signer.Address,
                    gasLimit: new BigInteger(300_000));
                TransactionReceipt mintReceipt;
                using (var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(20)))
                    mintReceipt = await host.RunAsync(
                        token => signer.SendTransactionAndWaitForReceiptAsync(mintInput, token), timeout.Token);

                Assert.That(mintReceipt.Status.Value, Is.EqualTo(BigInteger.One));
                var balance = await host.RunAsync(
                    _ => binding.BalanceOfAsync(new TokenBalanceOfParams { Arg0 = signer.Address }));
                Assert.That(balance.Arg0, Is.EqualTo(mintedAmount));

                var transfers = mintReceipt.Logs
                    .Select(log => binding.DecodeTransferEvent((FilterLog)log))
                    .ToList();
                Assert.That(transfers.Any(log => log.Amount == mintedAmount &&
                    log.To.Equals(signer.Address, StringComparison.OrdinalIgnoreCase)), Is.True);

                var approveData = binding.EncodeApprove(new TokenApproveParams
                {
                    Spender = signer.Address,
                    Amount = approvedAmount
                });
                var approveInput = AbiTypegenTransaction.Create(
                    ContractAddress,
                    approveData,
                    signer.Address,
                    gasLimit: new BigInteger(300_000));
                TransactionReceipt approveReceipt;
                using (var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(20)))
                    approveReceipt = await host.RunAsync(
                        token => signer.SendTransactionAndWaitForReceiptAsync(approveInput, token), timeout.Token);

                Assert.That(approveReceipt.Status.Value, Is.EqualTo(BigInteger.One));
                var allowance = await host.RunAsync(
                    _ => binding.AllowanceAsync(new TokenAllowanceParams
                    {
                        Arg0 = signer.Address,
                        Arg1 = signer.Address
                    }));
                Assert.That(allowance.Arg0, Is.EqualTo(approvedAmount));

                var approvals = approveReceipt.Logs
                    .Select(log => binding.DecodeApprovalEvent((FilterLog)log))
                    .ToList();
                Assert.That(approvals.Any(log => log.Amount == approvedAmount &&
                    log.Spender.Equals(signer.Address, StringComparison.OrdinalIgnoreCase)), Is.True);
            }
            finally
            {
                UnityEngine.Object.Destroy(go);
            }
        }

        private static async Task AssertRpcErrorAsync()
        {
            var factory = new NethereumUnityRpcFactory();
            var client = factory.CreateClient(new Uri(RpcUrl));
            try
            {
                await client.SendRequestAsync<string>("abi_typegen_method_that_does_not_exist");
                Assert.Fail("Expected JSON-RPC error for missing method.");
            }
            catch (RpcResponseException)
            {
                // Expected: Nethereum surfaces the server's JSON-RPC error instead of returning default data.
            }
        }

        private static async Task AssertTypedContractRevertAsync()
        {
            var factory = new NethereumUnityRpcFactory();
            var web3 = factory.CreateReadOnlyWeb3(new Uri(RpcUrl));
            const string unfunded = "0x0000000000000000000000000000000000000001";
            var binding = new TokenBinding(web3, ContractAddress);
            var amount = BigInteger.One << 200;
            var calldata = binding.EncodeTransfer(new TokenTransferParams
            {
                To = "0x0000000000000000000000000000000000000002",
                Amount = amount
            });
            try
            {
                await web3.Eth.Transactions.Call.SendRequestAsync(new CallInput
                {
                    From = unfunded,
                    To = ContractAddress,
                    Data = calldata
                });
                Assert.Fail("Expected InsufficientBalance for an unfunded token account.");
            }
            catch (RpcResponseException error)
            {
                var data = error.RpcError?.Data as string;
                Assert.That(data, Is.Not.Null.And.StartsWith("0x"), "Anvil should include custom error bytes");
                var decoded = binding.DecodeInsufficientBalanceError(data);
                Assert.That(decoded.Account, Is.EqualTo(unfunded).IgnoreCase);
                Assert.That(decoded.Available, Is.EqualTo(BigInteger.Zero));
                Assert.That(decoded.Required, Is.EqualTo(amount));
            }
        }

        private static void RequireAnvilEnvironment()
        {
            if (string.IsNullOrWhiteSpace(RpcUrl) ||
                string.IsNullOrWhiteSpace(ContractAddress) ||
                string.IsNullOrWhiteSpace(PrivateKey) ||
                string.IsNullOrWhiteSpace(ChainIdText))
            {
                throw new InvalidOperationException("Run this suite through e2e/native/anvil.py with ATG_RPC_URL, ATG_TOKEN_ADDRESS, ATG_PRIVATE_KEY and ATG_CHAIN_ID.");
            }
        }
    }
}
