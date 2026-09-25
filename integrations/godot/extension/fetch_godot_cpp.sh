#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
expected=507ed9d840c01a3c5b2a39af8bb4000bfac30bf5
if [[ ! -e godot-cpp ]]; then
  git clone --depth 1 --branch 10.0.0-stable https://github.com/godotengine/godot-cpp.git godot-cpp
fi
if [[ ! -d godot-cpp/.git ]] || [[ "$(git -C godot-cpp rev-parse HEAD)" != "$expected" ]]; then
  echo "error: godot-cpp must be the pinned 10.0.0-stable commit $expected" >&2
  exit 1
fi
