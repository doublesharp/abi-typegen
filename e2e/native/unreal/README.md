# Unreal consumer

Run `make e2e-unreal UNREAL_ROOT=/path/to/UnrealEngine` with Unreal Engine
5.8.3, Foundry, Python 3.11+, and Rust installed. The target regenerates all 26
Foundry sample contracts and a separate metadata-only fixture, stages the
runtime and plugin, builds the automation host, and runs the codec, lifecycle,
and Anvil suites in UnrealEditor-Cmd. It writes fresh automation reports and
logs under `TestResults/`.

The full suite passed locally on macOS arm64 with Unreal Engine 5.8.3. Windows
build rules are unqualified; Linux support is not implemented. Blueprint reads submit
`eth_call` with `to` and `data` at `latest`; they do not set `from`, so
sender-sensitive views use the provider default. Use generated native calldata
with an application-owned RPC flow when another caller is needed. Signing and
wallet storage belong to the application.

The optional manual GitHub workflow is skipped by default. To enable it, set
`UNREAL_ENGINE_CI_ENABLED=true`, set `UNREAL_ROOT_5_8_3` to the installed engine
path, and configure a macOS arm64 self-hosted runner labeled
`unreal-engine-5.8.3`. A skipped workflow is not a test pass or platform
qualification.
