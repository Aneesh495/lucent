#!/usr/bin/env bash
set -euo pipefail
mkdir -p "$1"
cd "$1"
fetch() {
  local name="$1" url="$2" digest="$3"
  if [[ ! -f "$name.tar.xz" ]]; then
    curl -fsSL --retry 2 -o "$name.download" "$url"
    mv "$name.download" "$name.tar.xz"
  fi
  printf '%s  %s\n' "$digest" "$name.tar.xz" | shasum -a 256 -c -
}
fetch gmp-6.3.0 https://ftp.gnu.org/gnu/gmp/gmp-6.3.0.tar.xz a3c2b80201b89e68616f4ad30bc66aee4927c3ce50e33929ca819d5c43538898
fetch mpfr-4.2.2 https://www.mpfr.org/mpfr-4.2.2/mpfr-4.2.2.tar.xz b67ba0383ef7e8a8563734e2e889ef5ec3c3b898a01d00fa0a6869ad81c6ce01
