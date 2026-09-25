#!/bin/sh
set -eu

mode=${1:-offline}
case "$mode" in
    offline|anvil) ;;
    *) echo "usage: $0 [offline|anvil]" >&2; exit 2 ;;
esac

TYPEGEN=${TYPEGEN:-target/debug/abi-typegen}
project=e2e/native/elixir
generated=$project/lib/contracts
plain=$(mktemp -d)
trap 'rm -rf "$plain"' EXIT HUP INT TERM

rm -rf "$generated"
"$TYPEGEN" generate --artifacts e2e/foundry-sample/out --out "$generated" --target elixir
"$TYPEGEN" generate --artifacts e2e/foundry-sample/out --out "$plain" --target elixir --no-wrappers
"$TYPEGEN" generate --artifacts "$project/artifacts" --out "$generated" --target elixir
"$TYPEGEN" generate --artifacts "$project/artifacts" --out "$plain" --target elixir --no-wrappers

[ "$(find "$generated" -name '*.ex' -type f | wc -l | tr -d ' ')" = 27 ]
[ "$(find "$plain" -name '*.ex' -type f | wc -l | tr -d ' ')" = 27 ]

ATG_ELIXIR_PLAIN="$plain" elixir -e '
  dir = System.fetch_env!("ATG_ELIXIR_PLAIN")
  files = Path.wildcard(Path.join(dir, "*.ex"))
  Enum.each(files, &Code.compile_file/1)
  unless function_exported?(Token, :abi_json, 0) and
           String.contains?(Token.abi_json(), "balanceOf") and
           not function_exported?(Token, :balance_of, 1) do
    raise "metadata-only Token unexpectedly depends on Ethers wrappers"
  end
'

if [ "$mode" = anvil ]; then
    : "${ATG_RPC_URL:?Anvil RPC URL is required}"
    : "${ATG_TOKEN_ADDRESS:?deployed Token address is required}"
    : "${ATG_PRIVATE_KEY:?Anvil signer is required}"
    : "${ATG_CHAIN_ID:?Anvil chain ID is required}"
fi

(cd "$project" && mix deps.get --check-locked && mix test)
