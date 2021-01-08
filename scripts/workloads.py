#!/usr/bin/env python3
"""Frozen independent workload definitions for protocol lucent-diagnostic-v1."""
from fractions import Fraction
from hashlib import sha256
from pathlib import Path
import argparse
import json

ROOT = Path(__file__).resolve().parents[1]
COUNTS = {"scalar": 10_000, "graph": 2_000, "adversarial": 500, "calculus": 160}


def scalar():
    for family in range(10):
        for i in range(1000):
            n, d = i - 500, 3 * i + 1001
            a = Fraction(n, d)
            b = Fraction(i + 7, i + 1103)
            if family == 0:
                source = f"({a}+{b})*({a}-{b})"
                oracle = {"exact": str(a * a - b * b)}
            elif family == 1:
                k = i % 7 + 1
                source = f"({b})^(-{k})"
                oracle = {"exact": str(b ** -k)}
            elif family == 2:
                source = f"{i}.125 + .0625 - {i}.05"
                oracle = {"exact": str(Fraction(11, 80))}
            elif family == 3:
                source = f"sqrt({b.numerator**2}/{b.denominator**2})"
                oracle = {"exact": str(b)}
            else:
                function = ["exp", "sin", "cos", "log", "sqrt", "normal_sf"][family - 4]
                argument = a if family <= 6 else b
                if family == 9:
                    argument *= 18
                source = f"{function}({argument})"
                oracle = {"function": function, "argument": str(argument)}
            yield {"request": {"op": "evaluate", "expr": source, "bits": 80}, "oracle": oracle}


def graph():
    for family in range(10):
        for i in range(200):
            k = Fraction(15 + i, 103)
            shift = Fraction(2 * i + 1, 100_000)
            view = {"x": [-2, 2], "y": [-10, 10], "pixels": [480, 360]}
            fields = {}
            if family == 0:
                source, check = f"({k})x^3-2x", "cubic"
            elif family == 1:
                source, check = f"sin(({k})x)", "sine"
            elif family == 2:
                source, check = f"sqrt(x+3+{k})", "sqrt"
            elif family == 3:
                source, check = f"log(x+3+{k})", "log"
            elif family == 4:
                source, check = f"exp(({k})x/2)", "exp"
            elif family == 5:
                source, check = f"1/(x-{shift})", "pole"
            elif family == 6:
                source, check = f"(x-{shift})*(x+1)/(x-{shift})", "removable"
            elif family == 7:
                source, check = f"tan(x+{shift})", "tan"
            elif family == 8:
                source, check = f"if(x<{shift}, x+2, x-2)", "jump"
            else:
                source, check = f"(x-{k})^2+y^2=1/4", "implicit"
                fields["mode"] = "implicit"
                view = {"x": [-1, 3], "y": [-2, 2], "pixels": [180, 180]}
            yield {
                "request": {"op": "graph", "expr": source, "bits": 80, "pixel_error": 0.45,
                            "budget": 1600, "view": view, **fields},
                "oracle": {"class": check, "k": str(k), "shift": str(shift)},
            }


def adversarial():
    for family in range(10):
        for i in range(50):
            request = {"op": "evaluate"}
            if family == 0:
                expr, status = f"({i}+1)/(1-1)", "undefined"
            elif family == 1:
                expr, status = f"sqrt(-{i+1})", "undefined"
            elif family == 2:
                expr, status = f"log(-{i+1})", "undefined"
            elif family == 3:
                expr, status = f"1e{100_000+i}", "unresolved"
            elif family == 4:
                expr, status = f"exp({800+i})", "unresolved"
            elif family == 5:
                expr, status = f"if(interval(-{i+1},1)>0,1,0)", "unresolved"
            elif family == 6:
                expr, status = "(" * (50 + i) + "1" + ")" * (50 + i), "error"
            elif family == 7:
                expr, status = f"{i}.12.13", "error"
            elif family == 8:
                expr, status = f"sin({i+1}kg)", "unsupported"
            else:
                expr, status = f"1/(x-{i}/1000)", "unresolved"
                request = {"op": "enclose", "domain": [-1, 1]}
            yield {"request": {**request, "expr": expr}, "oracle": {"status": status}}


def calculus():
    for i in range(80):
        q = Fraction(150 + i, 100)
        yield {
            "request": {"op": "roots", "expr": f"x^2-{q}", "domain": [1, 2]},
            "oracle": {"function": "sqrt", "argument": str(q), "operation": "root"},
        }
    for i in range(40):
        k = Fraction(i + 4, 31)
        yield {
            "request": {"op": "integral", "expr": f"({k})x^3-2x+1", "domain": [0, 1]},
            "oracle": {"exact": str(k / 4), "operation": "integral"},
        }
    for i in range(40):
        k = Fraction(i + 1, 75)
        yield {
            "request": {"op": "integral", "expr": f"exp(({k})x)", "domain": [0, 1]},
            "oracle": {"function": "exp_integral", "argument": str(k), "operation": "integral"},
        }


def compact(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False)


def materialize(destination):
    destination.mkdir(parents=True, exist_ok=True)
    out = {}
    for name, generator in (("scalar", scalar), ("graph", graph), ("adversarial", adversarial), ("calculus", calculus)):
        rows = list(generator())
        assert len(rows) == COUNTS[name]
        payload = "".join(compact({"id": f"{name}-{i:05d}", **row}) + "\n" for i, row in enumerate(rows)).encode()
        (destination / (name + ".jsonl")).write_bytes(payload)
        out[name] = {"cases": len(rows), "sha256": sha256(payload).hexdigest()}
    return {"format": "lucent-diagnostic-v1", "datasets": out}


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("destination", type=Path)
    parser.add_argument("--freeze", action="store_true")
    args = parser.parse_args()
    result = materialize(args.destination)
    manifest = ROOT / "data" / "MANIFEST.json"
    if args.freeze:
        manifest.write_text(json.dumps(result, indent=2) + "\n")
    elif result != json.loads(manifest.read_text()):
        raise SystemExit("Generated workload bytes differ from the frozen manifest")
    print(", ".join(f"{name}: {row['cases']}" for name, row in result["datasets"].items()))
