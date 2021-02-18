#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cache="$root/.agent-local/toolchain"
prefix="$cache/native"
"$root/scripts/download-sources.sh" "$cache"
mkdir -p "$cache/native-build"
cd "$cache/native-build"
if [[ ! -f "$prefix/gmp.ready" ]]; then
  tar xf "$cache/gmp-6.3.0.tar.xz"
  (
    cd gmp-6.3.0
    ./configure --disable-assembly --enable-static --disable-shared --prefix="$prefix" &&
      make -j "${LUCENT_JOBS:-6}" &&
      make install
  ) > gmp.log 2>&1 || { tail -60 gmp.log; exit 1; }
  touch "$prefix/gmp.ready"
fi
if [[ ! -f "$prefix/mpfr.ready" ]]; then
  tar xf "$cache/mpfr-4.2.2.tar.xz"
  (
    cd mpfr-4.2.2
    ./configure --with-gmp="$prefix" --enable-static --disable-shared --prefix="$prefix" &&
      make -j "${LUCENT_JOBS:-6}" &&
      make install
  ) > mpfr.log 2>&1 || { tail -60 mpfr.log; exit 1; }
  touch "$prefix/mpfr.ready"
fi
