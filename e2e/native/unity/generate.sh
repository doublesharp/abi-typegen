#!/usr/bin/env sh
set -eu

here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
repo=$(CDPATH='' cd -- "$here/../../.." && pwd)
typegen=${TYPEGEN:-"$repo/target/debug/abi-typegen"}
staging=$(mktemp -d)
trap 'rm -rf "$staging"' EXIT HUP INT TERM

"$typegen" generate --artifacts "$repo/e2e/foundry-sample/out" --out "$staging/wrappers" --target csharp --contracts Token
"$typegen" generate --artifacts "$here/artifacts" --out "$staging/metadata" --target csharp --no-wrappers --contracts CodecCases

cp "$staging/wrappers/Token.cs" "$here/Assets/Generated/Token.generated.cs"
cp "$staging/metadata/CodecCases.cs" "$here/Assets/Generated/CodecCases.metadata.generated.cs"
