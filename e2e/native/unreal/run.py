#!/usr/bin/env python3
"""Build and run the Unreal compatibility project, validating fresh automation JSON."""

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time


ROOT = Path(__file__).resolve().parents[3]
PROJECT_DIR = ROOT / "e2e" / "native" / "unreal"
PROJECT = PROJECT_DIR / "AbiTypegenUnrealTestHost.uproject"
PLUGIN_SOURCE = ROOT / "integrations" / "unreal" / "Plugins" / "AbiTypegen"
PLUGIN_STAGED = PROJECT_DIR / "Plugins" / "AbiTypegen"
EXPECTED_TESTS = (
    "AbiTypegen.Unreal.Rpc.StrictEnvelope",
    "AbiTypegen.Unreal.Codec.MetadataAndGeneratedNames",
    "AbiTypegen.Unreal.Codec.Uint256Above128Bits",
    "AbiTypegen.Unreal.Codec.OverloadsTuplesArrays",
    "AbiTypegen.Unreal.Codec.EventsAndCustomErrors",
    "AbiTypegen.Unreal.Lifecycle.Cancellation",
    "AbiTypegen.Unreal.Lifecycle.DestroyedOwner",
    "AbiTypegen.Unreal.Anvil.RpcErrorsAndReverts",
    "AbiTypegen.Unreal.Anvil.ReadWriteReceiptState",
    "AbiTypegen.Unreal.Anvil.BlueprintAsyncBalanceOf",
)
SUITE_MARKER = "AbiTypegen.Unreal."


def _field(mapping: dict, name: str):
    wanted = name.casefold()
    return next((value for key, value in mapping.items() if isinstance(key, str) and key.casefold() == wanted), None)


def _canonical_test_path(test: dict) -> str | None:
    path = _field(test, "FullTestPath") or ""
    if not isinstance(path, str):
        return None
    start = path.find(SUITE_MARKER)
    if start >= 0:
        return path[start:].rstrip(".")
    display = _field(test, "TestDisplayName") or ""
    return display if isinstance(display, str) and display.startswith(SUITE_MARKER) else None


def _state_is_success(state) -> bool:
    if isinstance(state, str):
        return state.casefold() in ("success", "3")
    return isinstance(state, int) and not isinstance(state, bool) and state == 3


def report_issues(report: object, expected: tuple[str, ...] = EXPECTED_TESTS) -> list[str]:
    if not isinstance(report, dict):
        return ["report root must be a JSON object"]
    tests = _field(report, "Tests")
    if not isinstance(tests, list):
        return ["report must contain a Tests array"]

    issues: list[str] = []
    selected: dict[str, dict] = {}
    unexpected: list[str] = []
    for test in tests:
        if not isinstance(test, dict):
            issues.append("Tests entries must be JSON objects")
            continue
        path = _canonical_test_path(test)
        if path is None:
            continue
        if path not in expected:
            unexpected.append(path)
            continue
        if path in selected:
            issues.append(f"duplicate test result: {path}")
        selected[path] = test

    for path in unexpected:
        issues.append(f"unexpected test in AbiTypegen.Unreal suite: {path}")
    missing = sorted(set(expected) - set(selected))
    for path in missing:
        issues.append(f"missing expected test: {path}")
    for path, test in selected.items():
        state = _field(test, "State")
        if not _state_is_success(state):
            issues.append(f"test did not succeed ({state!r}): {path}")
        for field in ("Errors", "Warnings"):
            count = _field(test, field)
            if not isinstance(count, int) or isinstance(count, bool) or count != 0:
                issues.append(f"test {field.lower()} must be zero, got {count!r}: {path}")

    for field in ("Failed", "InProcess", "SucceededWithWarnings", "NotRun"):
        value = _field(report, field)
        if not isinstance(value, int) or isinstance(value, bool) or value != 0:
            issues.append(f"report summary {field} must be zero, got {value!r}")
    succeeded = _field(report, "Succeeded")
    if not isinstance(succeeded, int) or isinstance(succeeded, bool) or succeeded != len(expected):
        issues.append(f"report summary Succeeded must equal {len(expected)}: got {succeeded!r}")
    return issues


def load_report(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8-sig"))


def run(command: list[str], *, cwd: Path, timeout: int, log_path: Path | None = None) -> str:
    print("$", " ".join(command), flush=True)
    if log_path:
        log_path.parent.mkdir(parents=True, exist_ok=True)
        with log_path.open("w", encoding="utf-8") as log:
            completed = subprocess.run(
                command,
                cwd=cwd,
                env=os.environ.copy(),
                text=True,
                stdout=log,
                stderr=subprocess.STDOUT,
                timeout=timeout,
                check=False,
            )
        output = log_path.read_text(encoding="utf-8", errors="replace")
    else:
        completed = subprocess.run(
            command,
            cwd=cwd,
            env=os.environ.copy(),
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=timeout,
            check=False,
        )
        output = completed.stdout or ""
    print(output, end="", flush=True)
    if completed.returncode != 0:
        raise RuntimeError(f"command exited with status {completed.returncode}: {command[0]}")
    return output


def engine_root(value: str | None) -> Path:
    candidate = value or os.environ.get("UNREAL_ROOT")
    if not candidate:
        raise RuntimeError("set UNREAL_ROOT to a locally installed Unreal Engine 5.8 directory")
    root = Path(candidate).expanduser().resolve()
    if not (root / "Engine" / "Build" / "BatchFiles" / "Mac" / "Build.sh").is_file():
        raise RuntimeError(f"Unreal Engine 5.8 Mac Build.sh not found below {root}")
    return root


def generate_bindings(typegen: str) -> None:
    stage = Path(tempfile.mkdtemp(prefix="abi-typegen-unreal-generate-"))
    try:
        output = stage / "wrappers"
        run(
            [typegen, "generate", "--artifacts", "e2e/foundry-sample/out", "--out", str(output),
             "--target", "unreal", "--package", "AbiTypegenUnrealTestHost"],
            cwd=ROOT,
            timeout=120,
        )
        public = PROJECT_DIR / "Source" / "AbiTypegenUnrealTestHost" / "Public" / "Generated"
        private = PROJECT_DIR / "Source" / "AbiTypegenUnrealTestHost" / "Private" / "Generated"
        shutil.rmtree(public, ignore_errors=True)
        shutil.rmtree(private, ignore_errors=True)
        public.mkdir(parents=True)
        private.mkdir(parents=True)
        for source in output.iterdir():
            if source.suffix == ".h":
                shutil.copy2(source, public / source.name)
            elif source.suffix in (".cpp", ".c"):
                shutil.copy2(source, private / source.name)

        metadata_out = stage / "metadata"
        run(
            [typegen, "generate", "--artifacts", "e2e/native/unreal/artifacts", "--out", str(metadata_out),
             "--target", "unreal", "--no-wrappers", "--package", "AbiTypegenUnrealTestHost",
             "--contracts", "MetadataOnlyToken"],
            cwd=ROOT,
            timeout=120,
        )
        for source in metadata_out.glob("*.h"):
            shutil.copy2(source, public / source.name)
        for source in metadata_out.glob("*.cpp"):
            shutil.copy2(source, private / source.name)
    finally:
        shutil.rmtree(stage, ignore_errors=True)


def stage_plugin(runtime_library: Path) -> None:
    if not runtime_library.is_file():
        raise RuntimeError(f"Rust runtime archive not found: {runtime_library}")
    include = PLUGIN_SOURCE / "Source" / "AbiTypegenRuntime" / "include" / "abi_typegen.h"
    if not include.is_file():
        raise RuntimeError(f"runtime header not found: {include}")
    runtime_dest = PLUGIN_SOURCE / "Source" / "AbiTypegenRuntime" / "lib" / "Mac" / "libabi_typegen_runtime.a"
    runtime_dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(runtime_library, runtime_dest)
    shutil.rmtree(PLUGIN_STAGED, ignore_errors=True)
    shutil.copytree(PLUGIN_SOURCE, PLUGIN_STAGED, ignore=shutil.ignore_patterns("Binaries", "Intermediate"))


def build_editor(root: Path, architecture: str, jobs: int) -> None:
    build_script = root / "Engine" / "Build" / "BatchFiles" / "Mac" / "Build.sh"
    command = [
        str(build_script), "AbiTypegenUnrealTestHostEditor", "Mac", "Development",
        f"-Project={PROJECT}", f"-architecture={architecture}", f"-MaxParallelActions={jobs}",
    ]
    run(command, cwd=PROJECT_DIR, timeout=1800, log_path=PROJECT_DIR / "TestResults" / "build.log")


def editor_test_command(executable: Path, architecture: str, report_path: Path) -> list[str]:
    return [
        str(executable), str(PROJECT), "-unattended", "-nop4", "-nosplash", "-nullrhi",
        "-stdout", "-FullStdOutLogOutput", "-log", f"-architecture={architecture}",
        f"-ReportExportPath={report_path}",
        "-ExecCmds=Automation RunTests AbiTypegen.Unreal",
        "-TestExit=Automation Test Queue Empty",
    ]


def run_editor_tests(root: Path, architecture: str, timeout: int) -> Path:
    executable = root / "Engine" / "Binaries" / "Mac" / "UnrealEditor-Cmd"
    if not executable.is_file():
        raise RuntimeError(f"UnrealEditor-Cmd not found: {executable}")
    run_id = f"run-{time.time_ns()}"
    report_dir = PROJECT_DIR / "TestResults" / run_id
    report_dir.mkdir(parents=True, exist_ok=False)
    report_path = report_dir / "AutomationReport"
    report_path.mkdir(parents=True)
    started_ns = time.time_ns()
    command = editor_test_command(executable, architecture, report_path)
    log_path = report_dir / "editor.log"
    output = run(command, cwd=PROJECT_DIR, timeout=timeout, log_path=log_path)
    report_file = report_path / "index.json"
    if not report_file.is_file() or report_file.stat().st_mtime_ns < started_ns:
        raise RuntimeError(f"fresh Unreal automation report was not written: {report_file}")
    try:
        report = load_report(report_file)
    except (OSError, json.JSONDecodeError) as error:
        raise RuntimeError(f"cannot read Unreal automation report {report_file}: {error}") from error
    issues = report_issues(report)
    if issues:
        raise RuntimeError("Unreal automation report failed validation:\n" + "\n".join(issues))
    print(f"PASS Unreal automation report: {len(EXPECTED_TESTS)} expected tests")
    return report_dir


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="action", required=True)
    generate = subparsers.add_parser("generate")
    generate.add_argument("--typegen", required=True)
    for name in ("stage", "build", "test"):
        child = subparsers.add_parser(name)
        child.add_argument("--engine-root")
        child.add_argument("--architecture", default="arm64")
        child.add_argument("--jobs", type=int, default=8)
        child.add_argument("--timeout", type=int, default=270)
        child.add_argument("--runtime-library", default="")
    args = parser.parse_args()
    try:
        if args.action == "generate":
            generate_bindings(args.typegen)
        elif args.action == "stage":
            if not args.runtime_library:
                parser.error("stage requires --runtime-library")
            engine_root(args.engine_root)
            stage_plugin(Path(args.runtime_library).resolve())
        else:
            root = engine_root(args.engine_root)
            if args.action == "build":
                build_editor(root, args.architecture, args.jobs)
            else:
                run_editor_tests(root, args.architecture, args.timeout)
    except (OSError, RuntimeError, subprocess.TimeoutExpired) as error:
        print(f"unreal runner: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
