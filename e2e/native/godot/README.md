# Godot consumer tests

The Godot fixture checks generated GDScript bindings and their native GDExtension
in Godot 4.7.2. It regenerates all Foundry sample contracts, plus the local
`CodecCases` and `MetadataOnlyToken` ABI fixtures. The extension links the shared
Rust runtime as a static library.

Install Godot 4.7.2, Rust/Cargo, Foundry, Python 3.11+, a C++ toolchain, and SCons
4.10.0, then run:

```sh
make e2e-godot
```

Set `GODOT_BIN` when the executable is not on `PATH`. `GODOT_PLATFORM` and
`GODOT_ARCH` default to the current host; `GODOT_JOBS` controls SCons parallelism.
The supported extension build pairs are Linux x86_64 and macOS arm64. If
`godot-cpp` is absent, Make fetches the version recorded in
`integrations/godot/extension/DEPENDENCIES.lock`.

The test runner imports the project, runs codec and packaging suites, then uses
the shared disposable Anvil harness for async reads, unlocked development-account
transaction submissions, receipt and event decoding, a revert, and pending-request lifetime cases.
The Anvil suite starts a bounded local mock JSON-RPC server for delayed, error,
and malformed responses. Runner success requires exact `PASS <suite>` output and
fails on Godot `ERROR:` or `SCRIPT ERROR:` diagnostics; Godot's import command can
otherwise return zero when an extension is missing.

The extension is compiled against C ABI v1 and statically links the selected
runtime archive. There is no runtime ABI version negotiation. The Godot CI
workflow is configured to build Linux x86_64 with the official Godot 4.7.2
release archive and a pinned SHA-256 digest; a green workflow run is still needed
before calling that platform qualified. The local full Make run has passed on
macOS arm64 with Godot 4.7.2. This target does not bundle local signing,
deployment helpers, event subscriptions, or fallback/receive wrappers. ABI
uint256 values use exact 32-byte `PackedByteArray` words. Export templates,
Android, iOS, and Web are outside this test target.
