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
scripts/relink-wasm.sh target/wasm32-unknown-emscripten/release/liblucent.a \
  "$LUCENT_MPFR_PREFIX/lib" web/public/engine/lucent.mjs
