using System.Threading;
using System.Threading.Tasks;
using Nethereum.RPC.Eth.DTOs;

namespace AbiTypegen.Unity
{
    /// <summary>
    /// Application-provided signing boundary. Production private keys do not belong in
    /// generated bindings or in this Unity compatibility package.
    /// </summary>
    public interface IAbiTypegenSigner
    {
        string Address { get; }

        Task<TransactionReceipt> SendTransactionAndWaitForReceiptAsync(
            TransactionInput transaction,
            CancellationToken cancellationToken);
    }
}
