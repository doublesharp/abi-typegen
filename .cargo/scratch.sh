#!/bin/sh
# Optional wrapper for commands outside Cargo, pnpm, and Make.
set -eu
if [ "$#" -eq 0 ]; then
    echo 'Usage: .cargo/scratch.sh COMMAND [ARG ...]' >&2
    exit 2
fi
script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
exec python3 "$script_dir/setup-scratch.py" --run "$@"
