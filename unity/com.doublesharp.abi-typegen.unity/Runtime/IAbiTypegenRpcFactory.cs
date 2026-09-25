using System;
using Nethereum.JsonRpc.Client;
using Nethereum.Web3;

namespace AbiTypegen.Unity
{
    /// <summary>Creates the RPC transport used by generated Nethereum bindings.</summary>
    public interface IAbiTypegenRpcFactory
    {
        IClient CreateClient(Uri endpoint);
        Web3 CreateReadOnlyWeb3(Uri endpoint);
    }
}
