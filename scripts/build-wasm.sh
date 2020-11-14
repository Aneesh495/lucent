#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cache="$PWD/.agent-local/toolchain"
if [[ ! -f "$cache/wasm/mpfr.ready" ]]; then
  printf '%s\n' 'Run make bootstrap-wasm before building the browser module.' >&2
  exit 1
fi
export EMSDK_QUIET=1
source "$cache/emsdk/emsdk_env.sh" >/dev/null
export LUCENT_MPFR_PREFIX="$cache/wasm"
export CC_wasm32_unknown_emscripten=emcc
export AR_wasm32_unknown_emscripten=emar
cargo build --locked --release --lib --target wasm32-unknown-emscripten
mkdir -p web/public/engine
emcc target/wasm32-unknown-emscripten/release/liblucent.a \
  "$LUCENT_MPFR_PREFIX/lib/libmpfr.a" "$LUCENT_MPFR_PREFIX/lib/libgmp.a" \
  -O3 --no-entry -sMODULARIZE=1 -sEXPORT_ES6=1 -sENVIRONMENT=web,worker,node \
  -sALLOW_MEMORY_GROWTH=1 -sMAXIMUM_MEMORY=134217728 -sFILESYSTEM=0 \
  -sEXPORTED_RUNTIME_METHODS=HEAPU8 \
  -sEXPORTED_FUNCTIONS='["_lc_alloc","_lc_free","_lc_run","_lc_output_len"]' \
  -o web/public/engine/lucent.mjs
