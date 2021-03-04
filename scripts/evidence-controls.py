#!/usr/bin/env python3
"""Test verifier handling of corrupt, incomplete, misbound and unreplayable evidence."""
import argparse
import gzip
from hashlib import file_digest
import json
from pathlib import Path
import shutil
import subprocess
import sys

HERE = Path(__file__).resolve().parent


def rehash(bundle):
    manifest = {}
    for path in sorted(bundle.rglob("*")):
        if path.is_file() and path.name != "MANIFEST.json":
            with path.open("rb") as file:
                manifest[str(path.relative_to(bundle))] = file_digest(file, "sha256").hexdigest()
    (bundle / "MANIFEST.json").write_text(json.dumps(manifest))


def run(original, destination):
    destination.mkdir(parents=True)
    checks = [("corrupt", "Checksum mismatch"), ("missing", "Missing or unsafe artifact"),
              ("source", "Source commit mismatch"), ("replay", "Replay mismatch for scalar-00000")]
    records = []
    for name, message in checks:
        target = destination / name
        shutil.copytree(original, target)
        if name == "corrupt":
            with (target / "scalar-native.jsonl.gz").open("ab") as file:
                file.write(b"corrupted")
        elif name == "missing":
            (target / "calculus-native.jsonl.gz").unlink()
        elif name == "source":
            path = target / "SOURCE_INPUTS.json"
            source = json.loads(path.read_text())
            source["commit"] = "0" * 40
            path.write_text(json.dumps(source))
            rehash(target)
        else:
            for backend in ["native", "wasm", "browser"]:
                path = target / f"scalar-{backend}.jsonl.gz"
                with gzip.open(path, "rt") as stream:
                    first, body = stream.readline(), stream.read()
                value = json.loads(first)
                value["canonical"] = "stale equation metadata"
                with gzip.open(path, "wt") as stream:
                    stream.write(json.dumps(value, separators=(",", ":"), sort_keys=True) + "\n" + body)
            rehash(target)
        argv = [sys.executable, str(HERE / "verify-evidence.py"), str(target)]
        run = subprocess.run(argv, capture_output=True, text=True, timeout=120)
        records.append({"case":name,"argv":argv,"exit":run.returncode,"stdout":run.stdout,"stderr":run.stderr})
        if run.returncode != 1 or message not in run.stderr:
            raise ValueError(records[-1])
        shutil.rmtree(target)
    (destination / "results.json").write_text(json.dumps(records, indent=2) + "\n")
    print("Bundle verifier rejected corruption, missing evidence, wrong source and failed replay")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("bundle", type=Path)
    parser.add_argument("destination", type=Path)
    args = parser.parse_args()
    run(args.bundle, args.destination)
