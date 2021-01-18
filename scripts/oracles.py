"""Independent checks for frozen workloads. The Lucent engine is never imported."""
from fractions import Fraction
import math

from flint import arb, ctx, fmpq

ctx.prec = 240


def as_ball(value):
    q = Fraction(value)
    return arb(fmpq(q.numerator, q.denominator))


def reference(spec):
    if "exact" in spec:
        return as_ball(spec["exact"])
    argument = as_ball(spec["argument"])
    name = spec["function"]
    if name == "normal_sf":
        return (argument / arb(2).sqrt()).erfc() / 2
    if name == "exp_integral":
        return argument.expm1() / argument
    return getattr(argument, name)()


def bounds(value, exact):
    if not isinstance(value, list) or len(value) != 2 or not all(isinstance(x, (float, int)) and math.isfinite(x) for x in value):
        raise ValueError("Expected two finite numeric endpoints")
    a, b = value
    if not (as_ball(a) <= exact.lower() and as_ball(b) >= exact.upper()):
        raise ValueError(f"Claimed bounds {value} exclude independent reference {exact}")
    return b - a


def numeric(row, actual):
    spec = row["oracle"]
    status = actual.get("status")
    result = actual.get("result")
    if "status" in spec:
        if status != spec["status"]:
            raise ValueError(f"Expected {spec['status']} and observed {status}")
        return {"observed_status": status}
    if spec.get("operation") == "root":
        if status != "root_certificate" or len(result["unique"]) != 1 or result["unresolved"]:
            raise ValueError("Expected one certified root and complete coverage")
        width = bounds(result["unique"][0]["bracket"], reference(spec))
        if result["unique"][0]["derivative"][0] <= 0:
            raise ValueError("Witness should have positive derivative")
        return {"width": width, "useful": width <= 1e-12}
    if spec.get("operation") == "integral":
        if status != "integral_certificate" or not result["panels"] or result["unresolved"]:
            raise ValueError("Expected an integral enclosure with panel evidence")
        width = bounds(result["enclosure"], reference(spec))
        return {"width": width, "useful": width <= 1e-12}
    if status == "exact":
        expected = spec.get("exact")
        if expected is None:
            ball = reference(spec)
            if not ball.is_exact() or as_ball(result["value"]) != ball:
                raise ValueError(f"Unsupported exact classification against {ball}")
        elif Fraction(result["value"]) != Fraction(expected):
            raise ValueError(f"Expected {expected} and received {result['value']}")
        return {"width": 0, "useful": True}
    if status != "enclosure" or "exact" in spec:
        raise ValueError(f"Incorrect scalar classification {status}")
    expected = reference(spec)
    width = bounds(result["bounds"], expected)
    return {"width": width, "useful": width <= 1e-12 * max(1.0, abs(float(expected.mid())))}


def evaluated(family, k, shift, x):
    if family == "cubic":
        return k*x**3 - 2*x
    if family == "sine":
        return math.sin(k*x)
    if family == "sqrt":
        return math.sqrt(x+3+k)
    if family == "log":
        return math.log(x+3+k)
    if family == "exp":
        return math.exp(k*x/2)
    if family == "pole":
        return 1/(x-shift)
    if family == "removable":
        return x+1
    if family == "tan":
        return math.tan(x+shift)
    if family == "jump":
        return x + (2 if x < shift else -2)
    raise ValueError(f"Unknown family {family}")


REGULAR = {"cubic", "sine", "sqrt", "log", "exp"}


def graph(row, actual):
    if actual.get("status") != "approximate_geometry":
        raise ValueError(f"Expected graph geometry, got {actual}")
    req = row["request"]
    view = req["view"]
    result = actual["result"]
    cells = result["cells"]
    spec = row["oracle"]
    family, k, shift = spec["class"], Fraction(spec["k"]), Fraction(spec["shift"])
    if not cells or result["processed"] > req["budget"]:
        raise ValueError("Missing cells or exceeded budget")
    if family == "implicit":
        return implicit(cells, view, k)
    cursor = view["x"][0]
    max_error = 0.0
    segments = 0
    unresolved = 0
    if family in {"pole", "removable", "jump"}:
        cuts = [shift]
    elif family == "tan":
        offset = as_ball(shift)
        cuts = [sign * arb.pi() / 2 - offset for sign in [-1, 1]]
    else:
        cuts = []
    for cell in cells:
        a, b = cell["domain"]
        if a != cursor or not a < b:
            raise ValueError("Curve coverage has a gap, overlap or reversed cell")
        cursor = b
        kind = cell["kind"]
        if kind == "unresolved":
            unresolved += 1
            if not cell.get("reason"):
                raise ValueError("Unresolved cell lacks a cause")
        if kind == "segment":
            segments += 1
            for cut in cuts:
                if isinstance(cut, Fraction):
                    enclosed = Fraction(a) <= cut <= Fraction(b)
                else:
                    enclosed = not (as_ball(a) > cut or as_ball(b) < cut)
                if enclosed:
                    raise ValueError(f"Segment {cell['domain']} bridges singularity or jump {cut}")
            left, right = cell["segment"]
            if left[0] != a or right[0] != b:
                raise ValueError("Segment endpoints disagree with the explored domain")
            if family in REGULAR:
                for i in range(17):
                    t = i / 16
                    x = a + (b - a) * t
                    y = left[1] + (right[1] - left[1]) * t
                    expected = evaluated(family, float(k), float(shift), x)
                    error = abs(expected - y) * view["pixels"][1] / (view["y"][1] - view["y"][0])
                    max_error = max(max_error, error)
        elif kind == "offscreen" and family in REGULAR:
            midpoint = (a+b)/2
            y = evaluated(family, float(k), float(shift), midpoint)
            if view["y"][0] <= y <= view["y"][1]:
                raise ValueError("Offscreen tile contains a visible analytic sample")
        elif kind not in {"offscreen", "unresolved", "excluded"}:
            raise ValueError(f"Unexpected curve tile {kind}")
    if cursor != view["x"][1] or not segments:
        raise ValueError("Incomplete viewport coverage or no useful segments")
    if family in REGULAR and (max_error >= 0.5 or unresolved):
        raise ValueError(f"Regular curve deviation {max_error} or unresolved cells {unresolved}")
    if cuts and not unresolved:
        raise ValueError("Expected visible unresolved coverage at the singularity")
    return {"segments": segments, "unresolved": unresolved, "sampled_error_px": max_error, "regular": family in REGULAR}


def implicit(cells, view, center):
    sweep = {}
    crossings = 0
    for index, cell in enumerate(cells):
        x, y = cell["x"], cell["y"]
        if not (view["x"][0] <= x[0] < x[1] <= view["x"][1] and view["y"][0] <= y[0] < y[1] <= view["y"][1]):
            raise ValueError("Implicit cell falls outside the viewport")
        sweep.setdefault(x[0], []).append((index, y))
        sweep.setdefault(x[1], []).append((index, None))
        if cell["kind"] == "crossing":
            crossings += 1
            if "topology unresolved" not in cell.get("reason", "") or not cell.get("segment"):
                raise ValueError("Implicit chord lacks its topology limitation")
        elif cell["kind"] == "empty":
            translated = [Fraction(a) - center for a in x]
            y = list(map(Fraction, y))
            min_sq = lambda p: 0 if p[0] <= 0 <= p[1] else min(a*a for a in p)
            max_sq = lambda p: max(a*a for a in p)
            if min_sq(translated) + min_sq(y) <= Fraction(1, 4) <= max_sq(translated) + max_sq(y):
                raise ValueError("Empty cell intersects analytic circle")
    active = {}
    cuts = sorted(sweep)
    if cuts[0] != view["x"][0] or cuts[-1] != view["x"][1] or not crossings:
        raise ValueError("Missing implicit coverage or useful crossings")
    for cut in cuts[:-1]:
        for key, bounds in sweep[cut]:
            if bounds is None:
                active.pop(key)
            else:
                active[key] = bounds
        cursor = view["y"][0]
        for lo, hi in sorted(active.values()):
            if lo != cursor:
                raise ValueError("Implicit cell partition has a gap or overlap")
            cursor = hi
        if cursor != view["y"][1]:
            raise ValueError("Incomplete implicit coverage")
    return {"segments": crossings, "unresolved": sum(c["kind"] == "unresolved" for c in cells), "regular": False}
