#!/bin/sh
set -eu
cmp crates/abi-typegen-runtime/include/abi_typegen.h crates/abi-typegen-codegen/src/renderers/c/abi_typegen.h
# Run from repository root after cargo build --workspace.
TYPEGEN=${TYPEGEN:-target/debug/abi-typegen}
RUNTIME_DIR=${RUNTIME_DIR:-target/debug}
NATIVE_OUT=$(mktemp -d)
trap 'rm -rf "$NATIVE_OUT"' EXIT
# Compile every Foundry fixture together to catch cross-contract names and types.
"$TYPEGEN" generate --artifacts e2e/foundry-sample/out --out "$NATIVE_OUT/all" --target cpp
python3 - "$NATIVE_OUT/all" <<'PYTHON'
from pathlib import Path
import sys
root = Path(sys.argv[1])
for extension, output in [("h", "all.c"), ("hpp", "all.cpp")]:
    (root / output).write_text("".join(f'#include "{p.name}"\n' for p in sorted(root.glob(f"*.{extension}"))))
PYTHON
${CC:-cc} -std=c11 -Wall -Wextra -Werror -fsyntax-only -I"$NATIVE_OUT/all" "$NATIVE_OUT/all/all.c"
${CXX:-c++} -std=c++17 -Wall -Wextra -Werror -fsyntax-only -I"$NATIVE_OUT/all" "$NATIVE_OUT/all/all.cpp"
"$TYPEGEN" generate --artifacts e2e/native/c/artifacts --out "$NATIVE_OUT" --target cpp
${CC:-cc} -std=c11 -Wall -Wextra -Werror -I"$NATIVE_OUT" e2e/native/c/consumer.c -L"$RUNTIME_DIR" -labi_typegen_runtime -Wl,-rpath,"$RUNTIME_DIR" -o "$NATIVE_OUT/c-consumer"
${CXX:-c++} -std=c++17 -Wall -Wextra -Werror -I"$NATIVE_OUT" e2e/native/c/consumer.cpp -L"$RUNTIME_DIR" -labi_typegen_runtime -Wl,-rpath,"$RUNTIME_DIR" -o "$NATIVE_OUT/cpp-consumer"
"$NATIVE_OUT/c-consumer"
"$NATIVE_OUT/cpp-consumer"
if [ -n "${ATG_RPC_URL:-}" ]; then
    "$TYPEGEN" generate --artifacts e2e/foundry-sample/out --contracts Token --out "$NATIVE_OUT/rpc" --target cpp
    ${CC:-cc} -std=c11 -Wall -Wextra -Werror -I"$NATIVE_OUT/rpc" e2e/native/c/rpc.c -L"$RUNTIME_DIR" -labi_typegen_runtime -Wl,-rpath,"$RUNTIME_DIR" -o "$NATIVE_OUT/rpc-c"
    ${CXX:-c++} -std=c++17 -Wall -Wextra -Werror -I"$NATIVE_OUT/rpc" e2e/native/c/rpc.cpp -L"$RUNTIME_DIR" -labi_typegen_runtime -Wl,-rpath,"$RUNTIME_DIR" -o "$NATIVE_OUT/rpc-cpp"
    python3 e2e/native/c/rpc_transport.py "$NATIVE_OUT/rpc-c"
    python3 e2e/native/c/rpc_transport.py "$NATIVE_OUT/rpc-cpp"
fi
