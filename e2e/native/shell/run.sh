#!/bin/sh
set -eu

mode=${1:-offline}
case "$mode" in
    offline|anvil) ;;
    *) echo "usage: $0 [offline|anvil]" >&2; exit 2 ;;
esac

TYPEGEN=${TYPEGEN:-target/debug/abi-typegen}
build=$(mktemp -d)
trap 'rm -rf "$build"' EXIT HUP INT TERM

"$TYPEGEN" generate --artifacts e2e/foundry-sample/out --out "$build/generated" --target shell
"$TYPEGEN" generate --artifacts e2e/foundry-sample/out --out "$build/plain" --target shell --no-wrappers
"$TYPEGEN" generate --artifacts e2e/native/shell/artifacts --out "$build/collision" --target shell

for source in "$build/generated"/*.sh "$build/plain"/*.sh "$build/collision"/*.sh; do
    /bin/bash -n "$source"
done
/bin/bash -euo pipefail -c '
    source "$1"
    [[ -n ${ATG_TOKEN_ABI:-} ]]
    ! declare -F atg_token_balance_of_encode >/dev/null
' _ "$build/plain/Token.sh"

ATG_SHELL_GENERATED="$build/generated" /bin/bash e2e/native/shell/OfflineTest.sh
ATG_SHELL_COLLISION="$build/collision/ShellCollision.sh" /bin/bash e2e/native/shell/CollisionTest.sh
if [ "$mode" = anvil ]; then
    ATG_SHELL_GENERATED="$build/generated" /bin/bash e2e/native/shell/AnvilTest.sh
fi
