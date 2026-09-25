# Unity compatibility tests

This project checks the existing C# bindings inside Unity. The reusable adapter
lives at `integrations/unity/com.doublesharp.abi-typegen.unity`; it uses Nethereum for ABI
encoding, RPC, and signing. There is no separate Unity generator target.

## Generate the fixtures

From the repository root:

```sh
make e2e-unity-generate
```

This builds the Foundry fixtures and regenerates the C# files used by the tests.
Do not edit the generated snapshots. `FORGE=/path/to/forge-ds` selects an existing
fork binary without changing the default toolchain.

## Run in Unity

Install and activate Unity 6000.6.3f1. The project pins Nethereum Unity 6.1.0,
Unity Newtonsoft JSON 3.2.2, and Test Framework 1.4.6. The package lockfile comes from the Editor
import; regenerate it through Unity when changing dependencies.

Set `UNITY_EDITOR` to the Editor executable, then run:

```sh
python3 e2e/native/unity/run.py editmode
python3 e2e/native/anvil.py python3 e2e/native/unity/run.py playmode
```

EditMode checks metadata and codecs. PlayMode checks signed local transactions,
reads, events, errors, cancellation, and request lifetime. The runner requires
fresh XML results, passing cases, and the expected test classes. Missing RPC
configuration or skipped cases fail the check.

To qualify standalone players, install Mono and IL2CPP build support for your
host platform, then run on macOS or Linux:

```sh
make e2e-unity
```

This regenerates fixtures, runs both Editor suites, and builds and executes each
standalone player. The player must finish its codec assertions and exit
successfully. A build by itself is insufficient. Test output is under
`TestResults/` and `Build/` in this project.

## CI

The manual `Unity compatibility` workflow requires an activated Linux runner
with the `unity-6000.6.3f1` and `unity-linux-il2cpp` labels. Set the repository
variable `UNITY_EDITOR_6000_6_3F1` to its Editor executable. The workflow installs
Foundry, Rust, and Python, then runs the same Make target and uploads logs.

Unity tests are separate from the ordinary native matrix because they require
an Editor license and platform modules. They are not silently skipped when the
Editor is missing.

## Limits

Unity 6000.6.3f1 on Apple Silicon macOS passes EditMode and PlayMode tests,
including signed Anvil requests, plus standalone Mono and IL2CPP codec checks.
Player checks cover exact uint256 values, named tuples, and custom errors.
Linux CI, Android, iOS, and WebGL remain unqualified.
Cancellation stops waiting for a result; it does not promise to abort an HTTP
request already dispatched by Nethereum. The private-key signer exists only in
the disposable Anvil tests.
