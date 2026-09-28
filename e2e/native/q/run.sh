#!/bin/sh
set -eu
# Requires an existing licensed q installation and its official C header.
TYPEGEN=${TYPEGEN:-target/debug/abi-typegen}
RUNTIME_DIR=${RUNTIME_DIR:-target/debug}
Q=${Q:-q}
: "${K_INCLUDE:?Set K_INCLUDE to the directory containing the official k.h}"
RUNTIME_DIR=$(cd "$RUNTIME_DIR" && pwd)
Q_OUT=$(mktemp -d)
trap 'rm -rf "$Q_OUT"' EXIT
"$TYPEGEN" generate --artifacts e2e/native/q/artifacts --out "$Q_OUT" --target q
case "$(uname -s)" in
  Darwin) Q_LINK_FLAGS='-undefined dynamic_lookup' ;;
  *) Q_LINK_FLAGS='' ;;
esac
# Intentional word splitting for platform linker flags.
${CC:-cc} -std=c11 -Wall -Wextra -Werror -shared -fPIC $Q_LINK_FLAGS \
  -I"$K_INCLUDE" -I"$Q_OUT" "$Q_OUT/abi_typegen_q.c" \
  -L"$RUNTIME_DIR" -labi_typegen_runtime -Wl,-rpath,"$RUNTIME_DIR" \
  -o "$Q_OUT/abi_typegen_q.so"
cat "$Q_OUT/Events.q" e2e/native/q/consumer.q > "$Q_OUT/check.q"
export Q
python3 e2e/native/q/test_runner.py
python3 e2e/native/q/run_q.py "$Q_OUT/check.q" "q event decoder checks passed"
if [ -n "${ATG_RPC_URL:-}" ]; then
    "$TYPEGEN" generate --artifacts e2e/foundry-sample/out --contracts Token --out "$Q_OUT" --target q
    python3 e2e/native/q/live.py "$Q_OUT"
fi
