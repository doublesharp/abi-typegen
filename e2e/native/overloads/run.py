#!/usr/bin/env python3
"""Compile and exercise overload aliases with both ABI orders, without RPC.

Uses the existing native consumers' dependency manifests and locks. Set TMPDIR
to disposable storage before running; all generated projects are removed on exit.
"""

import argparse
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def run(*args, cwd):
    subprocess.run([str(arg) for arg in args], cwd=cwd, check=True)


def copy(source, destination):
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, destination)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True, choices=["rust", "go", "swift", "kotlin", "fsharp", "ocaml"])
    parser.add_argument("--typegen", type=Path, default=ROOT / "target/debug/abi-typegen")
    parser.add_argument("--runtime-dir", type=Path, default=ROOT / "target/debug")
    options = parser.parse_args()
    target = options.target
    typegen = options.typegen.resolve()
    runtime = options.runtime_dir.resolve()
    native = HERE.parent / target

    with tempfile.TemporaryDirectory(prefix=f"overload-{target}-") as temporary:
        project = Path(temporary)
        if target == "rust":
            for name in ["Cargo.toml", "Cargo.lock"]:
                copy(native / name, project / name)
            copy(HERE / "rust.rs", project / "src/lib.rs")
            output = project / "src"
            command = ["cargo", "test", "--locked"]
        elif target == "go":
            for name in ["go.mod", "go.sum"]:
                copy(native / name, project / name)
            copy(HERE / "go_test.go", project / "usage/overloads_test.go")
            output = project / "contracts"
            command = ["go", "test", "./..."]
        elif target == "swift":
            for name in ["Package.swift", "Package.resolved"]:
                copy(native / name, project / name)
            copy(HERE / "OverloadsTests.swift", project / "Tests/ConsumerTests/OverloadsTests.swift")
            placeholder = project / "Sources/Consumer/Consumer.swift"
            placeholder.parent.mkdir(parents=True)
            placeholder.write_text("import Generated\n")
            output = project / "Sources/Generated"
            command = ["swift", "test", "--jobs", "2"]
        elif target == "kotlin":
            for name in ["build.gradle.kts", "settings.gradle.kts"]:
                copy(native / name, project / name)
            copy(HERE / "OverloadsTest.kt", project / "src/test/kotlin/OverloadsTest.kt")
            output = project / "build/generated-contracts"
            command = ["gradle", "test", "--no-daemon", "--max-workers=2", "--console=plain"]
        elif target == "fsharp":
            copy(native / "Consumer.fsproj", project / "Consumer.fsproj")
            copy(HERE / "Program.fs", project / "Program.fs")
            output = project / "Generated"
            command = ["dotnet", "run", "--project", "Consumer.fsproj"]
        else:
            copy(HERE / "consumer.ml", project / "consumer.ml")
            output = project
            command = None

        artifact = json.loads((ROOT / "tests/fixtures/overloads.json").read_text())
        for reversed_order in [False, True]:
            if reversed_order:
                for start in range(0, len(artifact["abi"]), 2):
                    artifact["abi"][start:start + 2] = reversed(artifact["abi"][start:start + 2])
            inputs = project / ("raw-inputs" if reversed_order else "hardhat-inputs")
            if reversed_order:
                # Exercise the public raw-ABI import path, without methodIdentifiers.
                raw = project / "raw.json"
                raw.write_text(json.dumps(artifact["abi"]))
                run(typegen, "fetch", "--file", raw, "--name", "Overloads", "--artifacts", inputs, cwd=project)
                source = inputs / "Overloads.sol/Overloads.json"
            else:
                # A Hardhat artifact also has no compiler methodIdentifiers.
                source = inputs / "contracts/Overloads.sol/Overloads.json"
                source.parent.mkdir(parents=True)
                source.write_text(json.dumps({**artifact, "_format": "hh-sol-artifact-1", "contractName": "Overloads"}))
            before = source.read_bytes()
            run(typegen, "generate", "--hardhat", "--artifacts", inputs, "--out", output,
                "--target", target, "--package", "com.example.contracts" if target == "kotlin" else "contracts", cwd=project)
            assert source.read_bytes() == before, "generator modified its input artifact"
            if command:
                run(*command, cwd=project)
            else:
                run("ocamlfind", "ocamlc", "-package", "zarith", "-c", "Atg_overloads.ml", cwd=project)
                run("ocamlfind", "ocamlc", "-ccopt", "-Wall -Wextra -Werror", "-c", "Atg_overloads.ocaml.c", cwd=project)
                run("ocamlfind", "ocamlc", "-custom", "-package", "zarith", "-linkpkg",
                    "Atg_overloads.cmo", "consumer.ml", "Atg_overloads.ocaml.o",
                    "-cclib", f"-L{runtime}", "-cclib", "-labi_typegen_runtime",
                    "-cclib", f"-Wl,-rpath,{runtime}", "-o", "consumer", cwd=project)
                run(project / "consumer", cwd=project)
            print(f"{target}: {'reversed raw ABI' if reversed_order else 'Hardhat artifact'} aliases and selectors passed", flush=True)


if __name__ == "__main__":
    main()
