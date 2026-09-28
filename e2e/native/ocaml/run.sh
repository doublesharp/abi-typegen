#!/bin/sh
set -eu
TYPEGEN=${TYPEGEN:-target/debug/abi-typegen}
RUNTIME_DIR=${RUNTIME_DIR:-target/debug}
OCAMLFIND=${OCAMLFIND:-ocamlfind}
OCAML_OUT=$(mktemp -d)
trap 'rm -rf "$OCAML_OUT"' EXIT
ROOT=$(pwd)
case "$RUNTIME_DIR" in /*) ;; *) RUNTIME_DIR="$ROOT/$RUNTIME_DIR";; esac
"$TYPEGEN" generate --artifacts e2e/native/ocaml/artifacts --out "$OCAML_OUT" --target ocaml
cp e2e/native/ocaml/consumer.ml "$OCAML_OUT/consumer.ml"
(cd "$OCAML_OUT"; for SOURCE in *.ml; do "$OCAMLFIND" ocamlc -package zarith -c "$SOURCE"; done; "$OCAMLFIND" ocamlc -ccopt '-Wall -Wextra -Werror' -c Atg_sample.ocaml.c; "$OCAMLFIND" ocamlc -custom -package zarith -linkpkg Atg_sample.cmo consumer.ml Atg_sample.ocaml.o -cclib "-L$RUNTIME_DIR" -cclib -labi_typegen_runtime -cclib "-Wl,-rpath,$RUNTIME_DIR" -o consumer; ./consumer)
# Link an instrumented bridge separately to force OCaml allocation exceptions.
cp e2e/native/ocaml/ownership.c e2e/native/ocaml/ownership.ml "$OCAML_OUT/"
(cd "$OCAML_OUT"
  "$OCAMLFIND" ocamlc -ccopt '-Wall -Wextra -Werror' -c ownership.c
  "$OCAMLFIND" ocamlc -custom -package zarith -linkpkg Atg_sample.cmo ownership.ml ownership.o -cclib "-L$RUNTIME_DIR" -cclib -labi_typegen_runtime -cclib "-Wl,-rpath,$RUNTIME_DIR" -o ownership
  ./ownership)
# Check all native fixtures, including metadata-only output.
for WRAPPERS in true false; do
  set --
  if [ "$WRAPPERS" = false ]; then set -- --no-wrappers; fi
  "$TYPEGEN" generate --artifacts e2e/native/c/artifacts --out "$OCAML_OUT/all-$WRAPPERS" --target ocaml "$@"
  (cd "$OCAML_OUT/all-$WRAPPERS"; for SOURCE in *.ml; do "$OCAMLFIND" ocamlc -package zarith -c "$SOURCE"; done)
done

if [ -d e2e/foundry-sample/out ]; then
  "$TYPEGEN" generate --artifacts e2e/foundry-sample/out --out "$OCAML_OUT/foundry" --target ocaml
  (cd "$OCAML_OUT/foundry"; for SOURCE in *.ml; do "$OCAMLFIND" ocamlc -package zarith -c "$SOURCE"; done)
fi

if [ -n "${ATG_RPC_URL:-}" ]; then
  sh e2e/native/ocaml/rpc.sh
fi
