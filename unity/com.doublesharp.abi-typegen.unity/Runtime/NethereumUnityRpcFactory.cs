using System;
using System.Threading;
using Nethereum.JsonRpc.Client;
using Nethereum.Unity.Rpc;
using Nethereum.Web3;

namespace AbiTypegen.Unity
{
    /// <summary>Uses Nethereum's UnityWebRequest-backed asynchronous RPC client.</summary>
    public sealed class NethereumUnityRpcFactory : IAbiTypegenRpcFactory
    {
        public IClient CreateClient(Uri endpoint)
        {
            if (endpoint == null) throw new ArgumentNullException(nameof(endpoint));
            var context = SynchronizationContext.Current;
            if (context == null)
                throw new InvalidOperationException("Create the Unity RPC client on the Unity main thread.");
            return new MainThreadRpcClient(
                new UnityWebRequestRpcTaskClient(endpoint), context, Thread.CurrentThread.ManagedThreadId);
        }

        public Web3 CreateReadOnlyWeb3(Uri endpoint)
        {
            return new Web3(CreateClient(endpoint));
        }
    }
}
