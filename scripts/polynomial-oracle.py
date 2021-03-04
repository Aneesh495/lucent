#!/usr/bin/env python3
"""Enumerate known rational roots and verify the exact isolators from raw responses."""
import argparse
from fractions import Fraction
import json
from pathlib import Path
import subprocess

COUNT = 512


def fixture(i):
    base = i // 16 - 16
    modulus = 7 + i % 16
    points = [Fraction(base) + Fraction(j, modulus) for j in [-3, 0, 2, 5]]
    weights = [1 + ((i >> j) & 3) for j in range(4)]
    factors = [f"(x-({p}))^{w}" for p,w in zip(points, weights)]
    # x^2 + a has no real roots since its constant coefficient is strictly positive.
    factors.append(f"(x^2+{i+1})")
    left = points[0] if i % 3 else points[1]
    right = points[-1] if i % 5 else points[-2]
    roots = [(str(point), weight) for point,weight in zip(points, weights) if left <= point <= right]
    request = {"op": "polynomial", "expr": "*".join(factors), "domain": [str(left), str(right)], "width": "1/10000000000"}
    return request, roots


def check(actual, roots):
    assert actual["status"] == "exact_root_count", actual
    result = actual["result"]
    assert result["distinct_count"] == len(roots) and result["multiplicity_count"] == sum(w for _, w in roots)
    expected = {Fraction(x): w for x, w in roots}
    for root in result["roots"]:
        a, b = Fraction(root["low"]), Fraction(root["high"])
        matches = [x for x in expected if (a == x == b) or (a < x < b)]
        assert len(matches) == 1 and b - a <= Fraction(1,10**10), root
        assert expected.pop(matches[0]) == root["multiplicity"], root
    assert not expected and not result["unresolved"]


def run(binary, output):
    output.parent.mkdir(parents=True, exist_ok=True)
    cases = [fixture(i) for i in range(COUNT)]
    requests = "".join(json.dumps(request) + "\n" for request, _ in cases)
    execution = subprocess.run([binary], input=requests, text=True, capture_output=True, check=True, timeout=120)
    with output.open("x") as stream:
        for index, ((request, expected), raw) in enumerate(zip(cases, execution.stdout.splitlines(), strict=True)):
            result = json.loads(raw)
            check(result, expected)
            print(json.dumps({"id": index, "request": request, "expected": expected, "response": result}), file=stream)
    print(f"{COUNT} rational polynomials checked for counts, isolators, endpoints and multiplicity")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    parser.add_argument("--binary", default="target/release/lucent")
    args = parser.parse_args()
    run(args.binary, args.output)
