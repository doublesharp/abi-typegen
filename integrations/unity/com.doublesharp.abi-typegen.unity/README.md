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
- `AbiTypegenRpcOperations` for simulation, exact gas estimation, canonical
  receipt tracking, and typed HTTP event polling for any generated ABI.

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

Use existing generated DTOs and decoder methods with the same RPC client:

```csharp
var operations = new AbiTypegenRpcOperations(web3.Client);
var options = new AbiTypegenPollingOptions {
    Confirmations = 3,
    PollInterval = TimeSpan.FromSeconds(1),
    Timeout = TimeSpan.FromMinutes(5),
    MaxBlocksPerPoll = 500
};
var simulated = await operations.SimulateAsync(
    transaction, binding.DecodeTransferResult, cancellationToken: cancellationToken);
var estimatedGas = await operations.EstimateGasAsync(transaction, cancellationToken);
// The application signer still owns submission. Its existing interface is unchanged.
var submitted = await signer.SendTransactionAndWaitForReceiptAsync(transaction, cancellationToken);
var confirmed = await operations.WaitForReceiptAsync(
    submitted.TransactionHash, options, cancellationToken);
```

Simulation performs `eth_call` and propagates RPC reverts. Estimates and
transaction amounts remain `BigInteger`; an estimate does not guarantee execution
or set the transaction's gas limit. Receipt tracking validates the returned
transaction hash, block hash, inclusion number, gas and status. It rechecks the
canonical block and receipt before returning at the configured depth. A canonical
confirmed receipt with status zero raises an error. One confirmation means the
inclusion block. Null or orphaned receipts remain pending until cancellation or
timeout.

Events use generated address/topic filters and typed decoder delegates:

```csharp
var cursor = new AbiTypegenEventCursor(startBlock); // explicit inclusive BigInteger block
var filter = binding.FilterTransfer(); // indexed topic filters work as well
var events = await operations.PollEventsAsync(
    filter, cursor, binding.DecodeTransferEvent, options, cancellationToken);

await host.RunAsync(token => operations.WatchEventsAsync(
    filter, cursor, binding.DecodeTransferEvent,
    async (batch, batchToken) => { await ApplyTransfersAsync(batch, batchToken); },
    options, token), cancellationToken);
```

Each poll fetches a contiguous range up to `MaxBlocksPerPoll` ending at the
confirmed head. The entire range is checked for valid log fields, canonical block
hashes, address/topic matching, conflicting duplicates and a stable end block
before decoding. Identical duplicate logs are collapsed and results are sorted by
block and log index. Failed RPC requests, invalid data and decoder errors leave
the cursor unchanged and propagate to the caller. Filters are copied; their
original block bounds are replaced by the cursor and confirmation policy.

The watcher owns its cursor while running and acknowledges a range only after its
batch handler completes. Empty successful ranges advance the cursor too. Handler
errors stop the watch with the range unacknowledged. Make handlers idempotent if
you restart after a partial handler failure or cancellation: application effects
and an in-memory cursor cannot be committed atomically. Reorgs behind the
acknowledged checkpoint raise an error; reconcile previously applied events and
create a new cursor at the desired replay block. The adapter does not silently
rewind or emit removed-event notifications.

`Timeout` bounds a receipt wait, a single poll, or the entire watch. External
cancellation and `AbiTypegenRequestHost` destruction stop the await and future
polls. An already dispatched SDK request or application handler may still finish;
handlers should honor the provided token. Cursors are in memory; persist your
application's acknowledged progress separately if needed. Polling uses HTTP and
does not provide WebSocket subscriptions.

See the [Unity consumer guide](../../../e2e/native/unity/README.md) for generated
binding integration details.
