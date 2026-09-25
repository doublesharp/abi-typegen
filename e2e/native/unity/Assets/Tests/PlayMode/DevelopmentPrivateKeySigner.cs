using System;
using System.Numerics;
using System.Threading;
using System.Threading.Tasks;
using AbiTypegen.Unity;
using Nethereum.RPC.Eth.DTOs;
using Nethereum.Web3;
using Nethereum.Web3.Accounts;

namespace AbiTypegen.Unity.E2E.Tests
{
    /// <summary>Test-only signer. The key is supplied by the disposable Anvil harness through the environment.</summary>
    internal sealed class DevelopmentPrivateKeySigner : IAbiTypegenSigner
    {
        private readonly Web3 web3;

        public DevelopmentPrivateKeySigner(Uri rpc, string privateKey, BigInteger chainId, IAbiTypegenRpcFactory rpcFactory)
        {
            if (rpc == null) throw new ArgumentNullException(nameof(rpc));
            if (string.IsNullOrWhiteSpace(privateKey)) throw new ArgumentException("Development key is required.", nameof(privateKey));
            if (rpcFactory == null) throw new ArgumentNullException(nameof(rpcFactory));

            var account = new Account(privateKey, chainId);
            Address = account.Address;
            web3 = new Web3(account, rpcFactory.CreateClient(rpc));
            web3.TransactionManager.UseLegacyAsDefault = true;
        }

        public string Address { get; }

        public Task<TransactionReceipt> SendTransactionAndWaitForReceiptAsync(
            TransactionInput transaction,
            CancellationToken cancellationToken)
        {
            return web3.TransactionManager.SendTransactionAndWaitForReceiptAsync(transaction, cancellationToken);
        }
    }
}
