# Supported language and result contracts

Enter one expression per row. Definitions use `name := expression` or `f(u,v) := expression`. Functions bind parameters lexically. A function sees its own arguments, global parameters, coordinate axes and other definitions. Definition cycles and duplicate names produce diagnostics.

## Grammar

From strongest to weakest, parentheses and named application, right-associative powers, prefix signs, multiplication and division, addition and subtraction, and comparisons. Thus `-2^2` is `-(2^2)`, `2^3^2` is `2^(3^2)`, and `1/2x` is `(1/2)*x`. Implicit multiplication before an identifier or opening parenthesis has ordinary multiplication precedence. An identifier followed by parentheses is a call. Comparisons use `=`, `!=`, `<`, `<=`, `>` or `>=`. They do not chain. Graph modes interpret implicit equations as the residual from left minus right.

Decimal literals are exact base-ten rationals, including scientific notation. An `e` without a signed or unsigned exponent can be the constant Euler's number in an implicit product. The formatter keeps decimal lexemes and inserts explicit multiplication. Source spans use UTF-8 byte offsets. There is a 384-token limit, a parser depth of 48 and an evaluator step budget.

## Types

- Real numbers are exact rational values or MPFR enclosures. Real powers accept bounded exact rational exponents, including odd real roots of negative bases. Complex powers require an exact integer. An evaluated zero base to power zero is one. Evaluation of the base can still fail.
- Quantities carry exponents for metres, seconds, kilograms and radians. Compatible units convert to these bases. Additions and comparisons require equal dimensions. Angular units override the global degree/radian interpretation of dimensionless trig arguments.
- Complex pairs support arithmetic, integer powers, conjugation, projections, magnitude, principal log and square root, and exponential. Explicit `i` or `complex(re,im)` selects complex evaluation. Real `sqrt(-1)` remains undefined.
- Lists use square brackets. Finite `sum` and `product` operate on a list. Nested rectangular lists support transpose, exact rational determinant and unique exact solve. Dot product checks length. Floating matrix jobs have explicit rank thresholds and residuals.
- Booleans are true, false or unknown when enclosures overlap a decision boundary. `if(predicate, yes, no)` evaluates a single proved branch. Indeterminate conditions require splitting or an unresolved result.

## Requests and identities

Native stdin accepts a UTF-8 JSON object per line with a 64 KiB limit. Browser workers use the same object format and limit. Numeric endpoints in JSON describe the exact binary64 values the decoder produced. Quote decimal or rational data for `parameters`, point `bindings`, polynomial domains and quantile probabilities where exact rational semantics are wanted.

| Operation | Required fields in addition to `op` |
| --- | --- |
| `simplify` | `expr`; returns a guarded expression and rewrite counts |
| `evaluate` | `expr`, optional exact `bindings` |
| `enclose` | `expr`, increasing `domain` |
| `graph` | `expr`, `view: {x, y, pixels}`, optional `mode`, `domain`, `horizontal` |
| `color` | `expr` in `z`, `view` |
| `roots`, `integral` | `expr`, increasing numeric `domain` |
| `polynomial` | Rational polynomial `expr`, two exact strings in `domain` |
| `feature` | `kind`, `expr`, increasing `domain`, `other` for paired operations |
| `improper` | `expr`, `lower`; upper endpoint is positive infinity |
| `derivative` | `expr`, `at` |
| `table` | `expr`, numeric `samples` |
| `surface`, `isosurface` | `expr`, `space: {x, y, z}`, `depth`; parametric mode also uses `x`, `y`, `parameter_domain` |
| `field` | `dx`, `dy`, `view`, `grid` |
| `ode` | `dx`, `dy`, `initial`, `time`, optional `event` and `validated` |
| `qr`, `svd` | Numeric `matrix`; `qr` optionally takes `rhs` |
| `regression` | `expr` list of points, `degree` |
| `quantile` | Exact string `probability` |
| `probability_plot` | `expr` list of observations |

Optional controls include `bits` (53 to 512), `max_bits`, `budget`, `tolerance`, `angle`, `definitions`, and `parameters`. `pixel_error` is separate from scalar tolerance. The serialized bound endpoints are outward binary64 projections. Wide bounds can be mathematically sound and fail the requested usefulness tolerance.

A certificate packet stores `{"op":"verify","request":{...},"claim":{...}}`. Native `lucent --verify packet.json` accepts up to 16 MiB and returns exit 1 for rejected claims. Plot data, SVG metadata and PNG iTXt store original requests, full saved responses, viewport, uncertainty flag and versioned document. The 3D JSON export also stores the camera.

## Workspace schema

Version 2 stores `title`, `cells`, `parameters`, `view`, `z`, and `angle`. Each cell has an identity, mode, source, visual properties and applicable coordinates, domains or data options. Imports cap file size at 192 KiB, 40 cells and 16 parameters. Version 1 `expr` rows migrate to `source` with explicit defaults. Decode rejects duplicate identities, invalid viewport bounds, unsupported versions and malformed required fields. The last valid primary autosave is copied to recovery before the next primary write.
