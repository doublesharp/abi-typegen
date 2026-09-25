# abi-typegen Unity adapter

Use abi-typegen's C# bindings with Unity's HTTP transport and object lifetime.
Nethereum provides the ABI codec and transaction APIs.

The package contains:

- `NethereumUnityRpcFactory` for UnityWebRequest-backed RPC clients. Create the
  client on Unity's main thread so subsequent SDK requests can return to it.
- `IAbiTypegenSigner` for an application-provided signer.
- `AbiTypegenTransaction` for validated, exact-integer transaction inputs.
- `AbiTypegenRequestHost` for canceling waits when a GameObject is destroyed.

A canceled wait does not guarantee that Nethereum aborts an HTTP request already
in flight. The package contains no private-key signer.

The consumer project and verification commands are in
[`e2e/native/unity`](../../e2e/native/unity/README.md). Unity 6000.6.3f1 on macOS passes Editor tests and standalone Mono/IL2CPP codec
checks. Linux, Android, iOS, and WebGL remain unqualified.
