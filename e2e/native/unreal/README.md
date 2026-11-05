# Unreal consumer

Run `make e2e-unreal UNREAL_ROOT=/path/to/UnrealEngine` with Unreal Engine
5.8.3, Foundry, Python 3.11+, and Rust installed. The target regenerates all
Foundry sample contracts, a synthetic nested-type naming fixture, and a separate
metadata-only fixture, stages the runtime and plugin, builds the automation host,
and runs the codec, lifecycle, polling, and Anvil suites in UnrealEditor-Cmd. It
writes fresh automation reports and logs under `TestResults/`.

The local target uses macOS arm64 with Unreal Engine 5.8.3. Windows build rules
have not been verified; Linux support is not implemented. Blueprint reads submit
`eth_call` with `to` and `data` at `latest`; they do not set `from`, so
sender-sensitive views use the provider default. Use generated native calldata
with an application-owned RPC flow when another caller is needed. Signing and
wallet storage belong to the application.

The optional manual GitHub workflow is skipped by default. To enable it, set
`UNREAL_ENGINE_CI_ENABLED=true`, set `UNREAL_ROOT_5_8_3` to the installed engine
path, and configure a macOS arm64 self-hosted runner labeled
`unreal-engine-5.8.3`. A skipped workflow is not a test pass or platform
qualification.

The reflected codec suite compares nested-array encodings with the native C codec,
checks fixed-array validation and decodes full-width words and indexed events.
The synthetic `GenericNames` fixture compiles array-element/sibling and nested
field names that used to collapse to the same generated C++ identifier. Metadata
fixtures retain reflected DTOs under `--no-wrappers`.

Polling automation uses scripted RPC transports for strict quantity/receipt
validation, confirmation rollback, canonical checkpoints, decode failure with an
unchanged cursor, cancellation and reorg detection. The Anvil write flow also
uses native simulation/gas-estimate helpers and confirmed receipt waits, then
checks generated Blueprint event delegates against real mint logs. Cancelling
from the first delegate suppresses the second log in the same confirmed range.

See [the Unreal adapter guide](../../../integrations/unreal/README.md) for the
Blueprint and native C++ surfaces, explicit event cursors and signer integration.
