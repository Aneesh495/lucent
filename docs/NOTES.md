# Numerical and engineering notes

## 1. Language semantics

The Pratt parser uses asymmetric binding powers for left-associative operators. Equal powers on the two sides of exponentiation make the right operand absorb a following power. Prefix minus binds below power and above products. Implicit products enter the same loop as explicit multiplication. Keeping one precedence path prevents hidden distinctions such as `2x/3` versus `2*x/3`. Named calls and assignment heads have distinct AST forms. Validation of a definition graph follows free references with formal parameters removed, so an argument that shadows a global does not create an accidental dependency. [Parser](../src/lang/parser.rs) and [dependency analysis](../src/program.rs).

## 2. Exact and approximate values

`0.1` is stored as the integer ratio 1/10. Exact operations retain reduced ratios until a supported transcendental operation, interval binding or requested approximate analysis requires a conversion. GMP converts the whole rational into MPFR in a single directed primitive. Converting numerator and denominator separately would introduce dependency and potentially change an endpoint. The engine rejects exact operations whose intermediate representation would exceed a bit budget. Numerical fits, sampled derivatives and geometry use separate statuses and remain approximate. [Exact representation](../src/exact.rs) and [value dispatch](../src/value.rs).

## 3. Outward rounding

Each MPFR endpoint computation chooses downward or upward rounding in C. Addition pairs the corresponding extrema, subtraction reverses the second operand, and multiplication and division compare directed corner extrema. Division rejects a denominator enclosure containing zero. The endpoint representation in the wire format rounds out again when projecting to binary64. For example, a positive value smaller than the smallest binary64 subnormal can become `[0, 5e-324]`. A result that would serialize an infinite bound is unresolved. Trigonometric interval ranges test possible critical points by normalizing through an interval for pi. [Bridge](../native/bridge.c), [interval arithmetic](../src/interval.rs), and [MPFR manual](https://www.mpfr.org/mpfr-current/mpfr.html).

## 4. Domain-aware simplification

`x/x` and `0*(1/x)` retain the denominator exclusion. A quotient cannot become a globally defined constant by cancellation. Symbolic differentiation removes literal identities, but derivative-based tools also evaluate the original expression on their witness cells. This check matters for lexical expansion. A function may ignore a formal parameter even though the call evaluates its argument first. The exact polynomial and improper subsets therefore parse the original syntax before using algebraic rules. [Symbolic rules](../src/differentiate.rs), [evaluator](../src/eval.rs), and [polynomial extraction](../src/polynomial.rs).

## 5. Dependency and precision

Interval evaluation of `x-x` over `[1,2]` produces `[-1,1]` because the two appearances are treated as independent bounds. Increasing precision cannot recover this lost relation. At a scalar point, cancellation such as `((exp(1)+1e-40)-exp(1))/1e-40` can lose accuracy through rounding. Precision escalation repeats the operation with the same exact inputs and a larger MPFR mantissa. The result includes achieved precision and target status. Width caused by an interval input is a property of the evaluation problem. [Scalar escalation](../src/job.rs) and [range operations](../src/interval.rs).

## 6. Root existence and uniqueness

On a closed witness where the expression evaluates without a domain split, an exact endpoint zero or opposite proved signs establishes existence. A derivative interval excluding zero establishes strict monotonicity and uniqueness on that witness. The solver tries the interval Newton image around a midpoint, then falls back to a bracket-preserving bisection. Shared partition boundaries may carry two local witnesses for a single exact endpoint root. The verifier checks the witness partition and recomputes exclusions and endpoint signs. [Root proof and verification](../src/calculus.rs).

## 7. Repeated polynomial roots

Sign changes miss even multiplicity. The rational polynomial path computes a square-free decomposition using polynomial gcd and exact division. A Sturm chain isolates each distinct root of the square-free part. Multiplicity comes from the factor that contains the isolated root. Exact endpoint zeros are listed as point roots. Nonpoint isolators describe open intervals so separate point roots at the ends are unambiguous. The finite degree and coefficient limits keep Euclidean elimination bounded. [Polynomial engine](../src/polynomial.rs).

## 8. Quadrature and improper tails

The Simpson path encloses the fourth derivative over each panel. It multiplies the maximum absolute fourth derivative by `width^5/2880`, expands the rounded Simpson estimate by this error and sums the panels outward. When the derivative proof fails but the interval value is valid, a range-times-width panel is sound and can be wider. Unresolved cells block an integral certificate. Infinite-tail support recognizes rational-rate exponentials and integer powers, with a closed formula for the convergent tail and a divergence test on the supported domain. Other tails return unresolved. [Finite integration](../src/calculus.rs), [tail rules](../src/improper.rs), and [Arb integration reference](https://flintlib.org/doc/arb_calc/index.html).

## 9. Implicit topology

Interval exclusion tests a whole 2D cell before any corner samples. A true inequality enclosure fills the entire cell. A false enclosure discards it. Strict and non-strict predicates differ when a comparison can prove equality at a boundary. The cell splitter reduces the wider screen dimension until a size or budget limit. A regular cell can prove a single crossing by combining a nonzero interval partial derivative with opposite signs on two whole faces. Edge brackets then enclose the endpoint roots. Its connecting chord remains approximate. A fallback chord from two sampled edge hits carries unresolved topology. A tangency, small loop or multiple branches can survive interval exclusion while evading the corner pattern, so those cells remain unresolved. [Implicit cells](../src/graph.rs).

## 10. Complex branches

The principal complex logarithm uses phase in `(-pi, pi]` and chooses positive pi on the negative real axis. A rectangle that might cross that cut returns unresolved. Real and imaginary parts of the principal square root have the usual nonnegative real part and the sign of the input imaginary part away from the cut. Domain coloring samples pixel centres. Hue corresponds to the phase of the output. A sample raster can straddle a discontinuity and cannot prove continuity. [Complex operations](../src/eval.rs), [phase enclosure](../src/interval.rs), and [coloring](../src/coloring.rs).

## 11. ODE error and stiffness

Dormand-Prince shares seven stages between order-five and order-four estimates. Their component disagreement controls trial step size. This measures a local estimate. It supplies neither a global trajectory enclosure nor guaranteed detection of even-multiplicity events. The optional initial Picard tube encloses the vector field over time and a candidate state box. The box must strictly contain the Picard image and satisfy a contraction bound. A large initial Jacobian norm relative to the horizon or repeated error rejections can indicate stiffness. Undefined stages follow a separate failure path. [ODE integration](../src/ode.rs).

## 12. Conditioning

A smaller residual is distinct from trustworthy coefficients. Householder QR pivots columns on the remaining norms and reports rank. One-sided Jacobi SVD estimates sensitivity through a singular-value ratio and independently reconstructs the matrix for a relative residual. Regression scales the explanatory axis before constructing the polynomial basis. Standard errors use the triangular factor and residual variance under an independent, equal-variance model. For derivatives, Lucent displays symbolic and forward-mode results with step disagreement and relative conditioning when the value excludes zero. [Linear numerical methods](../src/floating.rs) and [dual numbers](../src/dual.rs).

## 13. Worker scheduling

Generation IDs belong to each runner. A new job invalidates the previous ticket, and a superseded synchronous WASM call can be interrupted by terminating its worker. A worker that completed a job can be reused. A bounded cache reuses unchanged complete request signatures across independent expression edits. A definition or parameter change alters the request context and invalidates matching cache entries conservatively. Work limits apply at the request parser, exact and interval layers, spatial subdivision and worker timeout. The interface draws at animation frames and presents pending, preview, refinement or settled states. The source, parameters, viewport and precision of each saved layer are available in exports. [Runner](../web/src/engine.ts) and [workspace controller](../web/src/main.ts).

## 14. Interaction and accessibility

Expression controls use actual HTML inputs with labels, focus indicators and byte-span diagnostics. Keyboard users can enter expressions directly or through the math keyboard. The canvas has pan/zoom keyboard alternatives, and the 3D mesh has camera buttons and keys. The sample table provides a textual path to computed values. Color distinguishes plot layers, while text, hatching and numeric evidence distinguish result categories. Motion occurs in response to interaction. Rich structured mathematical reading via MathML remains a separate design question. [Workspace markup](../web/index.html), [browser tests](../web/tests/workspace.spec.ts), and [MathML Core](https://www.w3.org/TR/mathml-core/).

## 15. Rendering error

For explicit and parametric plots, the engine compares a chord against a band from interval values and, when available, a second-derivative interpolation estimate. A domain failure forces subdivision or leaves a gap. Screen-space estimates are projected into the current viewport and drawn as independent segments. The reported diagnostic curve error samples independently known curves at 17 points per visible segment. A diagnostic maximum and a formally rounded bound on projected geometry are different quantities. [Curve subdivision](../src/graph.rs) and [oracle](../scripts/oracles.py).
