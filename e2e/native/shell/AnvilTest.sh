#!/bin/bash
set -euo pipefail

source "${ATG_SHELL_GENERATED:?generated directory is required}/Token.sh"
: "${ATG_RPC_URL:?Anvil RPC URL is required}"
: "${ATG_TOKEN_ADDRESS:?deployed Token address is required}"
: "${ATG_PRIVATE_KEY:?Anvil signer is required}"

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
[[ $deployed_address == 0x* ]]
[[ $(atg_token_name_call "$deployed_address") == 'Shell deployed' ]]
[[ $(atg_token_symbol_call "$deployed_address") == SHL ]]

receipt=$(atg_token_mint_send "$ATG_TOKEN_ADDRESS" "$owner" "$amount")
hash=$(printf '%s\n' "$receipt" | python3 -c '
import json, sys
receipt = json.load(sys.stdin)
assert int(receipt["status"], 16) == 1, receipt
print(receipt["transactionHash"])
')
[[ $hash == 0x* ]]

balance=$(atg_token_balance_of_call "$ATG_TOKEN_ADDRESS" "$owner")
[[ $balance == "$amount" ]]
raw=$(atg_token_balance_of_call_raw "$ATG_TOKEN_ADDRESS" "$owner")
decoded=$(atg_token_balance_of_decode "$raw")
[[ $decoded == "$amount" ]]

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
[[ $event_amount == "$amount" ]]

if atg_token_transfer_send "$ATG_TOKEN_ADDRESS" "$zero" 1 >/dev/null 2>&1; then
    echo 'transfer to zero address unexpectedly succeeded' >&2
    exit 1
fi

echo 'shell Anvil bindings passed'
