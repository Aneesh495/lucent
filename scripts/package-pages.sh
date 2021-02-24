#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ $# -ne 1 || "$1" != /*/ ]]; then
  echo 'Pass an absolute URL path ending in slash, for example /lucent/.' >&2
  exit 2
fi
make wasm
(cd web && npm run build -- --base "$1")
site="$PWD/web/dist"
cache="$PWD/.agent-local/toolchain"
destination="$site/licenses"
mkdir -p "$destination"
cp "$cache/gmp-6.3.0.tar.xz" "$cache/mpfr-4.2.2.tar.xz" "$destination"
cp "$cache/gmp-6.3.0/COPYING.LESSERv3" "$destination/LGPL-3.0.txt"
cp "$cache/gmp-6.3.0/COPYINGv3" "$destination/GPL-3.0.txt"
cp target/wasm32-unknown-emscripten/release/liblucent.a "$destination/liblucent.a"
cp LICENSE "$destination/Lucent-MIT.txt"
git archive --format=tar.gz -o "$destination/lucent-source.tar.gz" HEAD
git rev-parse HEAD > "$site/SOURCE_COMMIT"
(cd "$destination" && shasum -a 256 *.xz *.gz *.a > SHA256SUMS)
cat > "$destination/index.html" <<'HTML'
<!doctype html>
<html lang="en">
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Lucent engine sources</title>
<h1>Lucent engine sources</h1>
<p>The WebAssembly module links GNU MPFR 4.2.2 and GNU GMP 6.3.0. The linked libraries are provided under LGPL version 3 or later. Lucent source is under MIT.</p>
<ul>
<li><a href="mpfr-4.2.2.tar.xz">MPFR corresponding source</a></li>
<li><a href="gmp-6.3.0.tar.xz">GMP corresponding source</a></li>
<li><a href="lucent-source.tar.gz">Lucent source, locks and build instructions</a></li>
<li><a href="liblucent.a">Relinkable Lucent archive</a></li>
<li><a href="LGPL-3.0.txt">GNU Lesser General Public License</a> and <a href="GPL-3.0.txt">GNU General Public License</a></li>
<li><a href="Lucent-MIT.txt">Lucent license</a> and <a href="SHA256SUMS">file checksums</a></li>
</ul>
<p>Use Emscripten 4.0.10. Build compatible static GMP and MPFR archives using the commands in the Lucent source archive, with any library changes you want to test. Run <code>scripts/relink-wasm.sh liblucent.a /path/to/static/libraries lucent.mjs</code>. Serve the generated module and WASM binary together from the application engine directory.</p>
</html>
HTML
python3 - <<'PY'
from pathlib import Path
path = Path("web/dist/index.html")
value = path.read_text()
anchor = '</aside>'
where = value.index(anchor)
path.write_text(value[:where] + '<a class="button quiet" href="licenses/index.html">Engine sources</a>\n' + value[where:])
PY
