#!/usr/bin/env bash
# Benchmark: abi-typegen vs TypeChain
# Runs each tool multiple times against the same contracts and reports timing.
# Both tools are warmed up first so only steady-state performance is measured.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
FOUNDRY_DIR="$SCRIPT_DIR/foundry-sample"
HARDHAT_DIR="$SCRIPT_DIR/hardhat-sample"
BINARY="$ROOT_DIR/target/release/abi-typegen"
RUNS=${1:-10}

echo "=== abi-typegen vs TypeChain benchmark ==="
echo "Contracts: Token, Vault, Registry, EdgeCases, Exchange (5 contracts)"
echo "Runs: $RUNS each (after warmup)"
echo ""

# Build release binary
echo "Building abi-typegen (release)..."
(cd "$ROOT_DIR" && cargo build --release --quiet)

# Ensure contracts are compiled
echo "Ensuring contracts are compiled..."
(cd "$FOUNDRY_DIR" && forge build --quiet 2>/dev/null)
# Compile hardhat without the abi-typegen plugin (just for TypeChain)
(cd "$HARDHAT_DIR" && pnpm exec hardhat typechain > /dev/null 2>&1)

# Warmup: run each tool once to prime filesystem caches, JIT, etc.
echo "Warming up..."
rm -rf "$FOUNDRY_DIR/src/generated"
"$BINARY" generate \
    --artifacts "$FOUNDRY_DIR/out" \
    --out "$FOUNDRY_DIR/src/generated" \
    --target viem > /dev/null 2>&1

rm -rf "$HARDHAT_DIR/typechain-types"
(cd "$HARDHAT_DIR" && pnpm exec hardhat typechain > /dev/null 2>&1)

echo ""

# ── abi-typegen ────────────────────────────────────────────────────────
echo "--- abi-typegen (release build) ---"
total_ft=0
for i in $(seq 1 $RUNS); do
    rm -rf "$FOUNDRY_DIR/src/generated"
    start=$(python3 -c 'import time; print(time.time())')
    "$BINARY" generate \
        --artifacts "$FOUNDRY_DIR/out" \
        --out "$FOUNDRY_DIR/src/generated" \
        --target ethers > /dev/null 2>&1
    end=$(python3 -c 'import time; print(time.time())')
    elapsed=$(python3 -c "print(f'{($end - $start) * 1000:.1f}')")
    total_ft=$(python3 -c "print($total_ft + $end - $start)")
    printf "  run %2d: %s ms\n" "$i" "$elapsed"
done
avg_ft=$(python3 -c "print(f'{($total_ft / $RUNS) * 1000:.1f}')")
echo "  avg: ${avg_ft} ms"
echo ""

# ── TypeChain ────────────────────────────────────────────────────────────
echo "--- TypeChain (via hardhat) ---"
total_tc=0
for i in $(seq 1 $RUNS); do
    rm -rf "$HARDHAT_DIR/typechain-types"
    start=$(python3 -c 'import time; print(time.time())')
    (cd "$HARDHAT_DIR" && pnpm exec hardhat typechain > /dev/null 2>&1)
    end=$(python3 -c 'import time; print(time.time())')
    elapsed=$(python3 -c "print(f'{($end - $start) * 1000:.1f}')")
    total_tc=$(python3 -c "print($total_tc + $end - $start)")
    printf "  run %2d: %s ms\n" "$i" "$elapsed"
done
avg_tc=$(python3 -c "print(f'{($total_tc / $RUNS) * 1000:.1f}')")
echo "  avg: ${avg_tc} ms"
echo ""

# ── Summary ──────────────────────────────────────────────────────────────
speedup=$(python3 -c "print(f'{$total_tc / $total_ft:.1f}')")
echo "=== Results ==="
echo "abi-typegen: ${avg_ft} ms avg"
echo "TypeChain:   ${avg_tc} ms avg"
echo "Speedup:     ${speedup}x"
