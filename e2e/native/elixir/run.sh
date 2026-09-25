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
artifacts=${ATG_ELIXIR_ARTIFACTS:-e2e/foundry-sample/out}
plain=$(mktemp -d)
trap 'rm -rf "$plain"' EXIT HUP INT TERM

rm -rf "$generated"
"$TYPEGEN" generate --artifacts "$artifacts" --out "$generated" --target elixir
"$TYPEGEN" generate --artifacts "$artifacts" --out "$plain" --target elixir --no-wrappers
"$TYPEGEN" generate --artifacts "$project/artifacts" --out "$generated" --target elixir
"$TYPEGEN" generate --artifacts "$project/artifacts" --out "$plain" --target elixir --no-wrappers

wrapper_count=$(find "$generated" -name '*.ex' -type f | wc -l | tr -d ' ')
plain_count=$(find "$plain" -name '*.ex' -type f | wc -l | tr -d ' ')
if [ "$plain_count" -ne "$wrapper_count" ]; then
    echo "Elixir generation mismatch: wrappers=$wrapper_count metadata=$plain_count" >&2
    exit 1
fi
for file in \
    big_int.ex boolean.ex data.ex edge_cases.ex exchange.ex \
    i_edge_cases.ex i_exchange.ex i_registry.ex i_token.ex i_vault.ex \
    list_contract.ex mod.ex naming_cases.ex native_cases.ex native_names.ex \
    override.ex registry_contract.ex string_contract.ex token.ex tuple_account.ex \
    tuple_amount.ex tuple_cases.ex uint256.ex vault.ex underscore_cases.ex
do
    if [ ! -f "$generated/$file" ] || [ ! -f "$plain/$file" ]; then
        echo "Elixir generation omitted $file in wrapper or metadata output" >&2
        exit 1
    fi
done

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
