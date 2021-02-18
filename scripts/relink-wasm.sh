#!/usr/bin/env bash
set -euo pipefail
if [[ $# -ne 3 ]]; then
  echo 'Usage: relink-wasm.sh LUCENT_ARCHIVE LIBRARY_DIRECTORY OUTPUT.mjs' >&2
  exit 2
fi
emcc "$1" \
  "$2/libmpfr.a" "$2/libgmp.a" \
  -O3 --no-entry -sMODULARIZE=1 -sEXPORT_ES6=1 -sENVIRONMENT=web,worker,node \
  -sALLOW_MEMORY_GROWTH=1 -sMAXIMUM_MEMORY=134217728 -sFILESYSTEM=0 \
  -sEXPORTED_RUNTIME_METHODS=HEAPU8 \
  -sEXPORTED_FUNCTIONS='["_lc_alloc","_lc_free","_lc_run","_lc_output_len"]' \
  -o "$3"
