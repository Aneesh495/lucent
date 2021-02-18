#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cache="$root/.agent-local/toolchain"
prefix="$cache/wasm"
mkdir -p "$cache"
cd "$cache"
if [[ ! -d emsdk/.git ]]; then
  git clone --depth 1 --branch 4.0.10 https://github.com/emscripten-core/emsdk.git
fi
test "$(git -C emsdk rev-parse HEAD)" = 62a853cd3b3134398ce85cde8bb5cbb2ef0194cb
if [[ ! -f emsdk/.emscripten ]]; then
  ./emsdk/emsdk install 4.0.10
  ./emsdk/emsdk activate 4.0.10
fi
export EMSDK_QUIET=1
source emsdk/emsdk_env.sh >/dev/null
rustup target add wasm32-unknown-emscripten --toolchain 1.88.0
"$root/scripts/download-sources.sh" "$cache"
if [[ ! -f "$prefix/gmp.ready" ]]; then
  tar xf gmp-6.3.0.tar.xz
  (
    cd gmp-6.3.0
    emconfigure ./configure --build="$(./config.guess)" --host=none --disable-assembly \
      --enable-static --disable-shared --prefix="$prefix" HOST_CC="$(command -v cc)" &&
    emmake make -j "${LUCENT_JOBS:-6}" &&
    emmake make install
  ) > gmp-build.log 2>&1 || { tail -60 gmp-build.log; exit 1; }
  touch "$prefix/gmp.ready"
fi
if [[ ! -f "$prefix/mpfr.ready" ]]; then
  tar xf mpfr-4.2.2.tar.xz
  (
    cd mpfr-4.2.2
    emconfigure ./configure --build="$(./config.guess)" --host=wasm32-unknown-emscripten \
      --with-gmp="$prefix" --disable-thread-safe --disable-decimal-float --disable-float128 \
      --enable-static --disable-shared --prefix="$prefix" HOST_CC="$(command -v cc)" &&
    emmake make -j "${LUCENT_JOBS:-6}" &&
    emmake make install
  ) > mpfr-build.log 2>&1 || { tail -60 mpfr-build.log; exit 1; }
  touch "$prefix/mpfr.ready"
fi
