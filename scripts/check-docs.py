#!/usr/bin/env python3
"""Check public wording, relative links, and editable diagram exports."""
from pathlib import Path
import re
import subprocess
import tempfile
from urllib.parse import unquote
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / "docs"
for path in [ROOT / "README.md", *DOCS.rglob("*.md")]:
    text = path.read_text()
    if "\N{EM DASH}" in text:
        raise SystemExit(f"Forbidden em dash in {path.relative_to(ROOT)}")
    for href in re.findall(r"\]\(([^\s()]+)\)", text):
        href = href.split("#", 1)[0]
        if href and not re.match(r"^[A-Za-z][A-Za-z0-9+.-]*:", href) and not (path.parent / unquote(href)).exists():
            raise SystemExit(f"Broken link {href} in {path.relative_to(ROOT)}")

graphs = sorted((DOCS / "diagrams").glob("*.dot"))
if len(graphs) < 12:
    raise SystemExit("Expected twelve editable diagrams")
namespace = "{http://www.w3.org/2000/svg}"
with tempfile.TemporaryDirectory() as directory:
    for source in graphs:
        generated = Path(directory) / source.with_suffix(".svg").name
        subprocess.run(["dot", "-Tsvg", source, "-o", generated], check=True)
        live = ET.parse(generated).getroot()
        saved = ET.parse(source.with_suffix(".svg")).getroot()
        label = lambda tree: sorted(node.text or "" for node in tree.iter(namespace + "text"))
        if live.tag != namespace + "svg" or saved.tag != namespace + "svg" or label(live) != label(saved):
            raise SystemExit(f"Diagram export is stale: {source.name}")
print(f"Checked relative links, public prose and {len(graphs)} editable diagrams")
