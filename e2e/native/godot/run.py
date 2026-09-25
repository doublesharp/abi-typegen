#!/usr/bin/env python3
"""Run Godot headless tests and fail closed on engine diagnostics or stale output."""

import argparse
import os
from pathlib import Path
import re
import shutil
import socket
import subprocess
import sys
import time


ROOT = Path(__file__).resolve().parents[3]
PROJECT = Path(__file__).resolve().parent / "test_project"
EXTENSION_FAILURE = re.compile(
    r"(?:failed to load|cannot load|could not load).*GDExtension|"
    r"GDExtension.*(?:failed|cannot|could not) load|"
    r"No GDExtension library found for current OS and architecture|"
    r"GDExtension dynamic library not found",
    re.IGNORECASE,
)
ANSI = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]")


def suite_passed(output: str, suite: str) -> bool:
    lines = ANSI.sub("", output).splitlines()
    return f"PASS {suite}" in lines and not any(line.startswith(f"FAIL {suite}:") for line in lines)


def has_extension_load_failure(output: str) -> bool:
    return EXTENSION_FAILURE.search(ANSI.sub("", output)) is not None


def has_failure_diagnostic(output: str) -> bool:
    return any(
        line.startswith(("ERROR:", "SCRIPT ERROR:"))
        for line in ANSI.sub("", output).splitlines()
    )


def godot_binary(explicit: str | None) -> str:
    candidate = explicit or os.environ.get("GODOT_BIN") or shutil.which("godot")
    if candidate:
        return candidate
    macos = Path("/Applications/Godot.app/Contents/MacOS/Godot")
    if macos.is_file():
        return str(macos)
    raise RuntimeError("Godot 4.7.2 is required; set GODOT_BIN to the executable")


def run_command(command: list[str], *, timeout: int, env: dict[str, str] | None = None) -> str:
    print("$", " ".join(command), flush=True)
    result = subprocess.run(
        command,
        cwd=ROOT,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=timeout,
        check=False,
    )
    print(result.stdout, end="", flush=True)
    if result.returncode != 0:
        raise RuntimeError(f"command exited {result.returncode}: {command[0]}")
    return result.stdout


def run_import(binary: str) -> None:
    output = run_command(
        [binary, "--headless", "--editor", "--path", str(PROJECT), "--import", "--quit"],
        timeout=180,
    )
    if has_failure_diagnostic(output):
        raise RuntimeError("Godot emitted an ERROR or SCRIPT ERROR diagnostic during import")
    if has_extension_load_failure(output):
        raise RuntimeError("Godot reported a GDExtension load failure during import")


def free_port() -> int:
    with socket.socket() as server:
        server.bind(("127.0.0.1", 0))
        return int(server.getsockname()[1])


def start_delayed_rpc() -> tuple[subprocess.Popen[str], str]:
    port = free_port()
    server_script = PROJECT / "tests" / "delayed_rpc.py"
    process = subprocess.Popen(
        [sys.executable, str(server_script), "--host", "127.0.0.1", "--port", str(port)],
        cwd=ROOT,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
    )
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        if process.poll() is not None:
            detail = process.stderr.read() if process.stderr else ""
            raise RuntimeError(f"delayed RPC fixture exited early: {detail}")
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=0.1):
                return process, f"http://127.0.0.1:{port}"
        except OSError:
            time.sleep(0.05)
    process.terminate()
    raise RuntimeError("delayed RPC fixture did not start within five seconds")


def run_suite(binary: str, suite: str) -> None:
    environment = os.environ.copy()
    process = None
    if suite == "anvil":
        process, delayed_url = start_delayed_rpc()
        environment["ATG_DELAY_RPC_URL"] = delayed_url
    try:
        output = run_command(
            [
                binary,
                "--headless",
                "--path",
                str(PROJECT),
                "--script",
                "res://tests/test_runner.gd",
                "--",
                f"--suite={suite}",
            ],
            timeout=180,
            env=environment,
        )
        if has_failure_diagnostic(output):
            raise RuntimeError("Godot emitted an ERROR or SCRIPT ERROR diagnostic")
        if has_extension_load_failure(output):
            raise RuntimeError("Godot reported a GDExtension load failure")
        if not suite_passed(output, suite):
            raise RuntimeError(f"Godot did not emit a fresh PASS {suite} marker")
    finally:
        if process is not None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("import", "suite"))
    parser.add_argument("--godot", help="Godot executable (or set GODOT_BIN)")
    parser.add_argument("--suite", choices=("codec", "packaging", "anvil"))
    args = parser.parse_args()
    if args.action == "suite" and not args.suite:
        parser.error("suite action requires --suite")
    try:
        binary = godot_binary(args.godot)
        if args.action == "import":
            run_import(binary)
        else:
            run_suite(binary, args.suite)
    except (OSError, RuntimeError, subprocess.TimeoutExpired) as error:
        print(f"godot runner: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
