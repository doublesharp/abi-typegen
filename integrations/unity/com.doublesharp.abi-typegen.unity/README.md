# abi-typegen Unity adapter

Use abi-typegen's generated C# bindings with Unity's HTTP transport and object
lifetime. Nethereum provides the ABI codec and transaction APIs. Install the
package with Unity Package Manager using **Add package from disk** and select this
directory's `package.json`. The package requires Unity 6.3 or later and Nethereum
Unity 6.1.0.

The package contains:

- `NethereumUnityRpcFactory` for UnityWebRequest-backed RPC clients. Create the
  client on Unity's main thread so subsequent SDK requests can return to it.
- `IAbiTypegenSigner` for an application-provided signer.
- `AbiTypegenTransaction` for validated, exact-integer transaction inputs.
- `AbiTypegenRequestHost` for canceling waits when a GameObject is destroyed.

Create the RPC client on Unity's main thread, then pass generated contract
calldata to the application's signer:

```csharp
using System;
using System.Numerics;
using AbiTypegen.Unity;

var rpc = new NethereumUnityRpcFactory();
var web3 = rpc.CreateReadOnlyWeb3(new Uri(rpcUrl));
var transaction = AbiTypegenTransaction.Create(
    contractAddress, generatedCalldata, signer.Address,
    gasLimit: new BigInteger(150_000));
var receipt = await signer.SendTransactionAndWaitForReceiptAsync(transaction, cancellationToken);
```

The application supplies the signer and owns private-key storage. Canceling an
`AbiTypegenRequestHost` wait cancels the caller's await; it may not stop an HTTP
request already in flight.

See the [Unity consumer guide](../../../e2e/native/unity/README.md) for generated
binding integration details.
