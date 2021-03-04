#!/usr/bin/env bash
set -u
cd "$(dirname "$0")/.."
printf 'OS and CPU\n'
uname -smr
case "$(uname -s)" in
  Darwin)
    sysctl -n machdep.cpu.brand_string hw.memsize hw.logicalcpu
    system_profiler SPDisplaysDataType | sed -n '1,16p'
    ;;
  Linux)
    lscpu | sed -n '1,15p'
    if command -v lspci >/dev/null; then lspci | awk '/VGA|3D controller|Display controller/'; fi
    ;;
esac
printf '\nToolchains\n'
rustc --version
node --version
python3 --version
if [[ -n "${LUCENT_MPFR_NATIVE:-}" ]]; then
  PKG_CONFIG_PATH="$LUCENT_MPFR_NATIVE/lib/pkgconfig" pkg-config --modversion mpfr gmp
else
  pkg-config --modversion mpfr gmp
fi
for dependency in git curl m4 xz dot; do command -v "$dependency" || printf 'Missing %s\n' "$dependency"; done
printf '\nStorage and prerequisites\n'
df -h .
for path in . data/MANIFEST.json .agent-local/toolchain/wasm/mpfr.ready .agent-local/oracle/bin/python; do
  if [[ -r "$path" ]]; then printf 'Readable %s\n' "$path"; else printf 'Absent or inaccessible %s\n' "$path"; fi
done
if [[ -w . ]]; then printf 'Checkout writable\n'; else printf 'Checkout read only\n'; fi
printf 'Runtime browser renderer identity is reported by make bench.\n'
