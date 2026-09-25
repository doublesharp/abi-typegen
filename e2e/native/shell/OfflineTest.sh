#!/bin/bash
set -euo pipefail

source "${ATG_SHELL_GENERATED:?generated directory is required}/Token.sh"
source "$ATG_SHELL_GENERATED/EdgeCases.sh"

owner=0x0000000000000000000000000000000000000001
expected="0x70a082310000000000000000000000000000000000000000000000000000000000000001"
calldata=$(atg_token_balance_of_encode "$owner")
[[ $calldata == "$expected" ]]

word=0x0000000000000000000000000000000100000000000000000000000000000000
decoded=$(atg_token_balance_of_decode "$word")
[[ $decoded == 340282366920938463463374607431768211456 ]]

data=0x0000000000000000000000000000000000000000000000000000000000000005
event_amount=$(atg_token_transfer_event_decode_data "$data")
[[ $event_amount == 5 ]]

atg_token_invalid_recipient_error_decode "${ATG_TOKEN_INVALID_RECIPIENT_ERROR_SELECTOR}"
if atg_token_invalid_recipient_error_decode 0x00000000 >/dev/null 2>&1; then
    echo 'wrong error selector was accepted' >&2
    exit 1
fi

if atg_token_balance_of_encode >/dev/null 2>&1; then
    echo 'missing ABI argument was accepted' >&2
    exit 1
fi
if atg_token_balance_of_encode "$owner" "$owner" >/dev/null 2>&1; then
    echo 'extra ABI argument was accepted' >&2
    exit 1
fi

ATG_CAST_BIN=true atg_token_balance_of_call "$owner" "$owner"
# shellcheck disable=SC2034 # Generated helpers read this option array.
ATG_CAST_CALL_ARGS=()
ATG_CAST_BIN=true atg_token_balance_of_call "$owner" "$owner"
# shellcheck disable=SC2034 # Generated helpers read this option array.
ATG_CAST_LOG_ARGS=()
ATG_CAST_BIN=true atg_token_transfer_event_logs "$owner"

if ATG_VALUE=1 ATG_CAST_BIN=true atg_token_mint_send "$owner" "$owner" 1 >/dev/null 2>&1; then
    echo 'nonpayable mint accepted ATG_VALUE' >&2
    exit 1
fi
ATG_CAST_SEND_ARGS=(--value 1)
if ATG_CAST_BIN=true atg_token_mint_send "$owner" "$owner" 1 >/dev/null 2>&1; then
    echo 'nonpayable mint accepted --value in Cast arguments' >&2
    exit 1
fi

# shellcheck disable=SC2034 # Generated helpers read this option array.
ATG_CAST_SEND_ARGS=()
if ATG_CAST_BIN=true atg_token_deploy --help 'test' TST 18 >/dev/null 2>&1; then
    echo 'deploy accepted flag-like bytecode' >&2
    exit 1
fi
if ATG_CAST_BIN=true atg_token_deploy 0x123 'test' TST 18 >/dev/null 2>&1; then
    echo 'deploy accepted odd-length bytecode' >&2
    exit 1
fi
capture=$(mktemp)
captured_args=$(mktemp)
trap 'rm -f "$capture" "$captured_args"' EXIT
cat >"$capture" <<'SH'
#!/bin/sh
printf '%s\n' "$@" >"$ATG_ARGS_FILE"
SH
chmod +x "$capture"
ATG_CAST_BIN="$capture" ATG_ARGS_FILE="$captured_args" ATG_VALUE=17 \
    atg_edge_cases_fund_send "$owner"
grep -Fx -- '--value' "$captured_args" >/dev/null
grep -Fx -- '17' "$captured_args" >/dev/null
grep -Fx -- 'fund()' "$captured_args" >/dev/null

echo 'shell offline bindings passed'
