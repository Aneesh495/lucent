#!/usr/bin/env python3
"""Verify a diagnostic bundle against a trusted checkout and bounded native replay."""
import argparse
from contextlib import ExitStack
import gzip
import hashlib
import json
from pathlib import Path, PurePosixPath
import subprocess
import sys

from oracles import graph, numeric

ROOT = Path(__file__).resolve().parents[1]
MAX_BYTES = 512 * 1024 * 1024
PROBES = {"scalar": [0, 174, 4018, 8130, 9971], "graph": [4, 475, 1123, 1457, 1832],
          "adversarial": [12, 221, 461], "calculus": [7, 78, 100, 157]}


def require(assertion, message):
    if not assertion:
        raise ValueError(message)


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def json_file(path):
    return json.loads(path.read_text())


def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT)


def inventory(bundle):
    manifest = json_file(bundle / "MANIFEST.json")
    require(isinstance(manifest, dict) and 6 < len(manifest) <= 350, "Malformed manifest")
    files = set()
    for relative, expected in manifest.items():
        ref = PurePosixPath(relative)
        require(not ref.is_absolute() and ".." not in ref.parts and str(ref) == relative, "Invalid manifest path")
        path = bundle / relative
        require(path.is_file() and path.resolve().is_relative_to(bundle) and not path.is_symlink(),
                f"Missing or unsafe artifact {relative}")
        require(path.stat().st_size <= MAX_BYTES and digest(path) == expected, f"Checksum mismatch for {relative}")
        files.add(relative)
    actual = {str(p.relative_to(bundle)) for p in bundle.rglob("*") if p.is_file()}
    require(actual == files | {"MANIFEST.json"}, "Artifact inventory differs from the manifest")


def check_source(bundle, expected):
    expected = git("rev-parse", expected + "^{commit}").decode().strip()
    head = git("rev-parse", "HEAD").decode().strip()
    require(expected == head and not git("status", "--porcelain", "--untracked-files=no"),
            "Trusted executable checkout must be clean and match the expected commit")
    claim = json_file(bundle / "SOURCE_INPUTS.json")
    require(claim.get("clean") and claim.get("commit") == expected, "Source commit mismatch")
    names = git("ls-tree", "-r", "--name-only", "-z", expected).decode().split("\0")[:-1]
    require(len(names) > 10 and set(claim["files"]) == set(names), "Source manifest is incomplete")
    for name in names:
        require(digest(ROOT / name) == claim["files"][name], f"Source input mismatch for {name}")
    return expected


def read_responses(stack, bundle, name, backend):
    path = bundle / f"{name}-{backend}.jsonl.gz"
    require(path.is_file(), f"Missing raw {backend} responses for {name}")
    return stack.enter_context(gzip.open(path, "rt"))


def checked_lines(stream):
    for row in stream:
        require(len(row) <= 16 * 1024 * 1024, "Raw observation exceeds response limit")
        yield row


def replay(probes, binary):
    require(probes, "No frozen replay probes")
    payload = "".join(json.dumps(request, allow_nan=False) + "\n" for _, request, _ in probes)
    execution = subprocess.run([binary], input=payload, capture_output=True, text=True, timeout=45, cwd=ROOT)
    require(execution.returncode == 0, "Bounded replay process failed")
    lines = execution.stdout.splitlines()
    require(len(lines) == len(probes), "Replay dropped requests")
    for (identity, _, saved), fresh in zip(probes, lines, strict=True):
        require(json.loads(fresh) == saved, f"Replay mismatch for {identity}")


def verify(bundle, expected, binary):
    inventory(bundle)
    commit = check_source(bundle, expected)
    definition = json_file(ROOT / "data" / "MANIFEST.json")
    require(json_file(bundle / "DATASETS.json") == definition, "Frozen dataset identity mismatch")
    summary = json_file(bundle / "ACCEPTANCE.json")
    profile = summary.get("profile")
    require(profile in {"subset", "diagnostic", "full"}, "Unknown campaign profile")
    if profile == "subset":
        names = list(summary.get("suites", {}))
        require(names and set(names) <= set(PROBES), "Invalid subset workload")
    else:
        names = list(definition["datasets"])
    commands = [json.loads(line) for line in (bundle / "COMMANDS.jsonl").read_text().splitlines()]
    require(commands and all(row["exit"] == 0 and row["elapsed_seconds"] > 0 for row in commands), "Missing or failed command execution")
    observed = {}
    probes = []
    backends = ["native", "wasm"] if profile == "subset" else ["native", "wasm", "browser"]
    for name in names:
        meta = definition["datasets"][name]
        cases = bundle / "fixtures" / f"{name}.jsonl"
        require(digest(cases) == meta["sha256"], f"Changed fixture bytes for {name}")
        check = graph if name == "graph" else numeric
        count = 0
        regular = 0
        useful = 0
        max_error = 0.0
        with ExitStack() as stack:
            fixtures = stack.enter_context(cases.open())
            raw = [read_responses(stack, bundle, name, backend) for backend in backends]
            for index, (line, *outputs) in enumerate(zip(fixtures, *(checked_lines(s) for s in raw), strict=True)):
                row = json.loads(line)
                decoded = [json.loads(out) for out in outputs]
                baseline = check(row, decoded[0])
                for i, alternate in enumerate(decoded[1:], start=1):
                    require(check(row, alternate) == baseline, f"Backend observations differ for {row['id']}")
                    if name != "graph":
                        require(alternate == decoded[0], f"Numerical parity mismatch for {row['id']}")
                        if i == 1:
                            require(outputs[0] == outputs[1], f"Native/WASM serialized parity differs for {row['id']}")
                if index in PROBES[name]:
                    probes.append((row["id"], row["request"], decoded[0]))
                if "useful" in baseline:
                    regular += 1
                    useful += int(baseline["useful"])
                max_error = max(max_error, baseline.get("sampled_error_px", 0))
                count += 1
        require(count == meta["cases"] and (not regular or useful >= 0.95 * regular), f"Workload or usefulness failure in {name}")
        observed[name] = {"cases": count, "useful": useful, "regular": regular, "max_error_px": max_error}
    if profile != "subset":
        controls = json_file(bundle / "controls" / "controls.json")
        require(len(controls) == 8, "Missing semantic controls")
        for entry in controls:
            name = entry["id"]
            require(name in {f"{kind}-{polarity}" for kind in
                ["false-interval", "false-uniqueness", "bridge-pole", "stale-geometry"] for polarity in ["positive", "negative"]},
                "Unexpected semantic control")
            packet = bundle / "controls" / (name + ".json")
            run = subprocess.run([binary, "--verify", packet], capture_output=True, text=True, cwd=ROOT, timeout=20)
            expected_code = 0 if name.endswith("-positive") else 1
            require(run.returncode == expected_code and run.stdout == entry["stdout"], f"Control replay failed for {name}")
    replay(probes, binary)
    return {"source": commit, "profile": profile, "verified": "diagnostic", "replay_probes": len(probes),
            "backends": backends, "observed": observed}


if __name__ == "__main__":
    cli = argparse.ArgumentParser()
    cli.add_argument("bundle", type=Path)
    cli.add_argument("--expected-commit", default="HEAD")
    cli.add_argument("--binary", default="target/release/lucent")
    cli.add_argument("--require-full", action="store_true")
    args = cli.parse_args()
    try:
        report = verify(args.bundle.resolve(), args.expected_commit, args.binary)
        if args.require_full:
            raise ValueError("Diagnostic evidence verified. Full acceptance gates remain open.")
        print(json.dumps(report, indent=2))
    except (ValueError, OSError, KeyError, TypeError, subprocess.SubprocessError) as exc:
        print(f"Verification failed: {exc}", file=sys.stderr)
        sys.exit(1)
