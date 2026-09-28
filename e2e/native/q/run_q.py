#!/usr/bin/env python3
"""Run q noninteractively; errors must not silently return to its prompt."""
import os
from pathlib import Path
import subprocess
import sys


def run_q(script, marker):
    script = Path(script).resolve()
    result = subprocess.run(
        [os.environ.get("Q", "q"), script.name, "-q"],
        cwd=script.parent,
        stdin=subprocess.DEVNULL,
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0 or marker not in result.stdout.splitlines():
        raise RuntimeError(
            f"q consumer did not complete (exit {result.returncode})\n"
            f"{result.stdout}{result.stderr}"
        )
    print(result.stdout, end="")


if __name__ == "__main__":
    run_q(sys.argv[1], sys.argv[2])
