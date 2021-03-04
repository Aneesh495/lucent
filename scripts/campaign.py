#!/usr/bin/env python3
"""Execute and retain the frozen diagnostic workload and command records."""
import argparse
import resource
from contextlib import ExitStack
from hashlib import sha256
import gzip
import json
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import time

from workloads import COUNTS, compact, materialize

ROOT = Path(__file__).resolve().parents[1]
BACKENDS = {"native": ["target/release/lucent"], "wasm": ["node", "scripts/batch-wasm.mjs"],
            "browser": ["node", "scripts/batch-browser.mjs"]}
REMAINING = ["G06 broader polynomial edge coverage", "G08 complex, 3D and ODE acceptance",
             "G09 pinned-device interaction benchmarks", "G10 p95 cancellation measurements", "G12 comprehensive accessibility",
             "G13 independent reconstruction of all exports", "G15 complete reproduction", "G16 release acceptance"]


def write(path, value):
    path.write_text(json.dumps(value, indent=2, allow_nan=False) + "\n")


def command_value(argv):
    return subprocess.check_output(argv, cwd=ROOT, text=True).strip()


def sha(path):
    with path.open("rb") as stream:
        from hashlib import file_digest
        return file_digest(stream, "sha256").hexdigest()


def sources():
    commit = command_value(["git", "rev-parse", "HEAD"])
    status = command_value(["git", "status", "--porcelain", "--untracked-files=normal"])
    files = command_value(["git", "ls-files", "-z"]).rstrip("\x00").split("\x00")
    return {"commit": commit, "clean": not status, "files": {name: sha(ROOT / name) for name in files if (ROOT / name).is_file()}}


def record(path, argv, inp=None, out=None, *, timeout=1800):
    started = time.monotonic()
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    with (inp.open("rb") if inp else open("/dev/null", "rb")) as stdin, \
            (out.open("wb") if out else (path / "console.log").open("ab")) as stdout, \
            (path / "stderr.log").open("ab") as stderr:
        try:
            process = subprocess.run(argv, stdin=stdin, stdout=stdout, stderr=stderr, timeout=timeout, cwd=ROOT)
            code = process.returncode
        except subprocess.TimeoutExpired:
            code = 124
    info = {"argv": argv, "exit": code, "elapsed_seconds": time.monotonic() - started}
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    with (path / "RESOURCES.jsonl").open("a") as stream:
        print(compact({"argv": argv, "child_user_seconds": after.ru_utime - before.ru_utime,
            "child_system_seconds": after.ru_stime - before.ru_stime,
            "maximum_child_rss_bytes_seen": after.ru_maxrss * (1 if sys.platform == "darwin" else 1024)}), file=stream)
    if inp is not None:
        info["input"] = str(inp.relative_to(path))
    if out is not None:
        info["output"] = str(out.relative_to(path))
    with (path / "COMMANDS.jsonl").open("a") as stream:
        print(compact(info), file=stream)
    if code:
        raise RuntimeError(f"Command exited {code}: {argv}")


def observe(path, suite, fixture, outputs):
    from oracles import graph, numeric
    check = graph if suite == "graph" else numeric
    outcomes = path / f"{suite}-outcomes.jsonl"
    failures = 0
    useful = 0
    regular = 0
    maximum = 0
    count = 0
    with ExitStack() as stack:
        cases = stack.enter_context(fixture.open())
        streams = [stack.enter_context(file.open()) for file in outputs.values()]
        dest = stack.enter_context(outcomes.open("w"))
        for case, first, second, *other in zip(cases, *streams, strict=True):
            row = json.loads(case)
            report = {"id": row["id"], "status": "passed"}
            try:
                a, b = json.loads(first), json.loads(second)
                if suite != "graph" and first != second:
                    raise ValueError("Serialized native and WASM results differ")
                observed = check(row, a)
                for alternate in [b, *(json.loads(item) for item in other)]:
                    alternate_observed = check(row, alternate)
                    if observed != alternate_observed:
                        raise ValueError("Native and alternate-backend observations differ")
                    if suite != "graph" and a != alternate:
                        raise ValueError("Native and browser numerical results differ")
                report.update(observed)
                if "useful" in observed:
                    useful += int(observed["useful"])
                    regular += 1
                maximum = max(maximum, observed.get("sampled_error_px", 0.0))
            except (ValueError, TypeError, KeyError, IndexError, ZeroDivisionError) as error:
                failures += 1
                report.update(status="failed", reason=str(error))
            print(compact(report), file=dest)
            count += 1
    if count != COUNTS[suite]:
        raise ValueError(f"Requested {COUNTS[suite]} {suite} cases and completed {count}")
    return {"completed": count, "failures": failures, "useful": useful, "regular": regular, "max_sampled_error_px": maximum,
            "passed": failures == 0 and (not regular or useful >= 0.95 * regular)}


def compress(file):
    dest = file.with_suffix(file.suffix + ".gz")
    with file.open("rb") as src, dest.open("wb") as dst, gzip.GzipFile(fileobj=dst, mode="wb", mtime=0, filename="") as payload:
        shutil.copyfileobj(src, payload)
    file.unlink()


def run(args):
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.-]{0,79}", args.run_id):
        raise ValueError("Invalid run identity")
    path = ROOT / "evidence" / args.run_id
    path.mkdir(parents=True)
    started = time.time()
    outcome = {"profile": args.profile, "run_id": args.run_id, "status": "failed", "suites": {}}
    try:
        from flint import __version__ as flint
        source = sources()
        write(path / "SOURCE_INPUTS.json", source)
        if not source["clean"]:
            raise ValueError("Campaign requires a clean tracked checkout")
        write(path / "ENVIRONMENT.json", {
            "platform": platform.platform(), "machine": platform.machine(), "python": platform.python_version(), "flint_python": flint,
            "rust": command_value(["rustc", "--version"]), "node": command_value(["node", "--version"]),
            "mpfr_native": json.loads(subprocess.check_output(
                [str(ROOT / "target/release/lucent")], input=b'{"ratio":"0","bits":96}'))["mpfr"],
            "gmp_headers": command_value(["pkg-config", "--modversion", "gmp"]),
            "wasm_sha256": sha(ROOT / "web/public/engine/lucent.wasm"),
            "native_sha256": sha(ROOT / "target/release/lucent"),
        })
        data = materialize(path / "fixtures")
        if data != json.loads((ROOT / "data" / "MANIFEST.json").read_text()):
            raise ValueError("Input bytes do not match the frozen corpus")
        write(path / "DATASETS.json", data)
        for suite in args.suites:
            fixture = path / "fixtures" / f"{suite}.jsonl"
            inputs = path / f"{suite}-requests.jsonl"
            with fixture.open() as src, inputs.open("w") as dest:
                for line in src:
                    print(compact(json.loads(line)["request"]), file=dest)
            outputs = {}
            engines = list(BACKENDS) if args.profile != "subset" else ["native", "wasm"]
            for backend in engines:
                argv = BACKENDS[backend]
                outputs[backend] = path / f"{suite}-{backend}.jsonl"
                record(path, argv, inputs, outputs[backend])
            result = observe(path, suite, fixture, outputs)
            outcome["suites"][suite] = result
            print(suite, result, flush=True)
            for output in outputs.values():
                compress(output)
        if args.profile != "subset":
            record(path, [sys.executable, "scripts/negative-controls.py", str(path / "controls")])
        passed = all(suite["passed"] for suite in outcome["suites"].values())
        if args.profile == "full":
            outcome["remaining"] = REMAINING
            outcome["status"] = "incomplete" if passed else "failed"
        elif passed:
            outcome["status"] = "diagnostic_passed"
    except Exception as exc:
        outcome["error"] = str(exc)
        print(exc, file=sys.stderr)
    outcome["wall_seconds"] = time.time() - started
    write(path / "ACCEPTANCE.json", outcome)
    manifest = {str(file.relative_to(path)): sha(file) for file in sorted(path.rglob("*")) if file.is_file()}
    write(path / "MANIFEST.json", manifest)
    print(path)
    return outcome["status"] == "diagnostic_passed"


if __name__ == "__main__":
    cli = argparse.ArgumentParser()
    cli.add_argument("--run-id", required=True)
    cli.add_argument("--profile", choices=["subset", "diagnostic", "full"], default="diagnostic")
    cli.add_argument("--suites", choices=list(COUNTS), nargs="+", default=list(COUNTS))
    args = cli.parse_args()
    if args.profile != "subset" and args.suites != list(COUNTS):
        cli.error("Diagnostic and full profiles require all frozen suites")
    sys.exit(0 if run(args) else 1)
