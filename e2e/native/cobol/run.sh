#!/bin/sh
set -eu

# Run from the repository root after building the CLI and shared ABI runtime.
mode=${1:-offline}
case "$mode" in
    offline|anvil) ;;
    *) echo "usage: $0 [offline|anvil]" >&2; exit 2 ;;
esac

TYPEGEN=${TYPEGEN:-target/debug/abi-typegen}
RUNTIME_DIR=${RUNTIME_DIR:-target/debug}
RUNTIME_DIR=$(cd "$RUNTIME_DIR" && pwd -P)
CC=${CC:-cc}
COBC=${COBC:-cobc}
root=e2e/native/cobol
build=$(mktemp -d)
trap 'rm -rf "$build"' EXIT HUP INT TERM

"$TYPEGEN" generate --artifacts e2e/foundry-sample/out --out "$build/generated" --target cobol
"$TYPEGEN" generate --artifacts e2e/foundry-sample/out --out "$build/plain" --target cobol --no-wrappers

for source in "$build/generated"/*.cobol.c "$build/plain"/*.cobol.c; do
    "$CC" -std=c11 -Wall -Wextra -Werror -fsyntax-only \
        -I"$(dirname "$source")" "$source"
    "$CC" -std=c11 -Wall -Wextra -Werror -fsyntax-only \
        -DATG_COBOL_WITH_CURL=1 \
        -I"$(pkg-config --variable=includedir json-c)" \
        -I"$(dirname "$source")" "$source"
done

for source in "$build/generated"/*.cob; do
    if grep -q '^PROGRAM-ID\.' "$source"; then
        "$COBC" -free -fsyntax-only "$source"
    fi
done

"$CC" -std=c11 -Wall -Wextra -Werror -DATG_COBOL_WITH_CURL=1 \
    -I"$(pkg-config --variable=includedir json-c)" \
    -I"$build/generated" -c "$build/generated/Token.cobol.c" -o "$build/Token.o"

"$CC" -std=c11 -Wall -Wextra -Werror -DATG_COBOL_WITH_CURL=1 \
    -I"$(pkg-config --variable=includedir json-c)" \
    -I"$build/generated" "$root/BridgeTest.c" \
    -L"$RUNTIME_DIR" -labi_typegen_runtime \
    -L"$(pkg-config --variable=libdir libcurl)" -lcurl \
    -L"$(pkg-config --variable=libdir json-c)" -ljson-c \
    -Wl,-rpath,"$RUNTIME_DIR" -o "$build/bridge-test"
"$build/bridge-test"

"$COBC" -x -free -o "$build/offline" \
    "$root/OfflineTest.cob" "$build/generated/Token.cob" "$build/Token.o" \
    -L"$RUNTIME_DIR" -labi_typegen_runtime \
    -L"$(pkg-config --variable=libdir libcurl)" -lcurl \
    -L"$(pkg-config --variable=libdir json-c)" -ljson-c \
    -Q "-Wl,-rpath,$RUNTIME_DIR"
"$build/offline"

if [ "$mode" = anvil ]; then
    : "${ATG_RPC_URL:?Anvil RPC URL is required}"
    : "${ATG_TOKEN_ADDRESS:?deployed Token address is required}"
    : "${ATG_PRIVATE_KEY:?Anvil development key is required}"
    owner=0x0000000000000000000000000000000000000001
    amount=340282366920938463463374607431768211456
    receipt=$(cast send --rpc-url "$ATG_RPC_URL" --private-key "$ATG_PRIVATE_KEY" \
        --json "$ATG_TOKEN_ADDRESS" 'mint(address,uint256)' "$owner" "$amount")
    printf '%s\n' "$receipt" | python3 -c \
        'import json,sys; r=json.load(sys.stdin); assert int(r["status"],16)==1, r'

    "$COBC" -x -free -o "$build/anvil" \
        "$root/AnvilTest.cob" "$build/generated/Token.cob" "$build/Token.o" \
        -L"$RUNTIME_DIR" -labi_typegen_runtime \
        -L"$(pkg-config --variable=libdir libcurl)" -lcurl \
        -L"$(pkg-config --variable=libdir json-c)" -ljson-c \
        -Q "-Wl,-rpath,$RUNTIME_DIR"
    "$build/anvil"
fi
