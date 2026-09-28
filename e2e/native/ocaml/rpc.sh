#!/bin/sh
set -eu
TYPEGEN=${TYPEGEN:-target/debug/abi-typegen}
RUNTIME_DIR=${RUNTIME_DIR:-target/debug}
OCAMLFIND=${OCAMLFIND:-ocamlfind}
ROOT=$(pwd)
case "$RUNTIME_DIR" in /*) ;; *) RUNTIME_DIR="$ROOT/$RUNTIME_DIR";; esac
RPC_OUT=$(mktemp -d)
trap 'rm -rf "$RPC_OUT"' EXIT
"$TYPEGEN" generate --artifacts e2e/foundry-sample/out --contracts Token --out "$RPC_OUT" --target ocaml
cp e2e/native/ocaml/rpc.ml e2e/native/ocaml/rpc_data.py "$RPC_OUT/"
(cd "$RPC_OUT"
  "$OCAMLFIND" ocamlc -package zarith -c Atg_token.ml
  "$OCAMLFIND" ocamlc -ccopt '-Wall -Wextra -Werror' -c Atg_token.ocaml.c
  "$OCAMLFIND" ocamlc -custom -package zarith,unix -linkpkg Atg_token.cmo rpc.ml Atg_token.ocaml.o -cclib "-L$RUNTIME_DIR" -cclib -labi_typegen_runtime -cclib "-Wl,-rpath,$RUNTIME_DIR" -o rpc
  ./rpc)
