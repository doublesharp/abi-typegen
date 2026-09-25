#!/bin/bash
set -euo pipefail

source "${ATG_SHELL_GENERATED:?generated directory is required}/Token.sh"
: "${ATG_RPC_URL:?Anvil RPC URL is required}"
: "${ATG_TOKEN_ADDRESS:?deployed Token address is required}"
: "${ATG_PRIVATE_KEY:?Anvil signer is required}"

assert_equal() {
    if [[ $1 != "$2" ]]; then
        printf 'expected %s, got %s\n' "$2" "$1" >&2
        exit 1
    fi
}

assert_cast_integer() {
    local actual=$1 expected=$2 annotation
    local scientific='^[0-9]+([.][0-9]+)?[eE][+-]?[0-9]+$'
    case $actual in
        "$expected") return 0 ;;
        "$expected ["*"]")
            annotation=${actual#"$expected ["}
            annotation=${annotation%]}
            if [[ $annotation =~ $scientific ]]; then return 0; fi
            ;;
    esac
    printf 'expected exact integer %s, got %s\n' "$expected" "$actual" >&2
    exit 1
}

assert_hex() {
    if [[ $1 != 0x* ]]; then
        printf 'expected a 0x-prefixed value, got %s\n' "$1" >&2
        exit 1
    fi
}

owner=$(cast wallet address --private-key "$ATG_PRIVATE_KEY")
zero=0x0000000000000000000000000000000000000000
amount=340282366920938463463374607431768211456

# shellcheck disable=SC2034 # Generated helpers read this option array.
ATG_CAST_SEND_ARGS=(--json)
bytecode=$(python3 -c '
import json
from pathlib import Path
print(json.loads(Path("e2e/foundry-sample/out/Token.sol/Token.json").read_text())["bytecode"]["object"])
')
deployed=$(atg_token_deploy "$bytecode" 'Shell deployed' SHL 18)
deployed_address=$(printf '%s\n' "$deployed" | python3 -c '
import json, sys
receipt = json.load(sys.stdin)
assert int(receipt["status"], 16) == 1, receipt
print(receipt["contractAddress"])
')
assert_hex "$deployed_address"
assert_equal "$(atg_token_name_call "$deployed_address")" '"Shell deployed"'
assert_equal "$(atg_token_symbol_call "$deployed_address")" '"SHL"'

receipt=$(atg_token_mint_send "$ATG_TOKEN_ADDRESS" "$owner" "$amount")
hash=$(printf '%s\n' "$receipt" | python3 -c '
import json, sys
receipt = json.load(sys.stdin)
assert int(receipt["status"], 16) == 1, receipt
print(receipt["transactionHash"])
')
assert_hex "$hash"

balance=$(atg_token_balance_of_call "$ATG_TOKEN_ADDRESS" "$owner")
# Cast can append a human-readable approximation to a large integer.
assert_cast_integer "$balance" "$amount"
raw=$(atg_token_balance_of_call_raw "$ATG_TOKEN_ADDRESS" "$owner")
decoded=$(atg_token_balance_of_decode "$raw")
assert_cast_integer "$decoded" "$amount"

# shellcheck disable=SC2034 # Generated helpers read this option array.
ATG_CAST_LOG_ARGS=(--json)
logs=$(atg_token_transfer_event_logs "$ATG_TOKEN_ADDRESS" "$zero" "$owner")
event_data=$(printf '%s\n' "$logs" | python3 -c '
import json, sys
logs = json.load(sys.stdin)
assert len(logs) == 1, logs
log = logs[0]
assert len(log["topics"]) == 3, log
print(log["data"])
')
event_amount=$(atg_token_transfer_event_decode_data "$event_data")
assert_cast_integer "$event_amount" "$amount"

if atg_token_transfer_send "$ATG_TOKEN_ADDRESS" "$zero" 1 >/dev/null 2>&1; then
    echo 'transfer to zero address unexpectedly succeeded' >&2
    exit 1
fi

echo 'shell Anvil bindings passed'
