# Capability and evidence index

[Build status](BUILD_STATUS.md) tracks the acceptance gates. This index identifies the implementation and the closest executable evidence for each workbench capability.

| ID | Implementation | Evidence and scope |
| --- | --- | --- |
| M01 | [Lexer and parser](../src/lang/parser.rs) | Precedence, byte spans, formatting and per-row errors. Broad round-trip generation remains open. |
| M02 | [Exact rationals](../src/exact.rs) | Frozen exact scalar cases cover decimals, powers and overflow budgets. |
| M03 | [Typed values](../src/value.rs) | Real, complex, Boolean, list and matrix operation tests. Type checks occur at evaluation. |
| M04 | [Definitions and DAG](../src/program.rs) | Lexical dependencies and cycles, finite list folds and browser sliders. |
| M05 | [Guarded simplification](../src/simplify.rs) | Coordinate cancellation preserves zero exclusion and keeps explicit interval values distinct. |
| M06 | [Symbolic rules](../src/differentiate.rs), [forward mode](../src/dual.rs) | Analytic derivative comparisons and bounded rational powers. |
| M07 | [Directed interval arithmetic](../src/interval.rs) | MPFR bridge, periodic extrema, rational powers and explicit domain issues. |
| M08 | [Scalar escalation](../src/job.rs) | Cancellation fixture reaches a useful width after escalation. Range dependency remains explicit. |
| M09 | [Explicit curve cells](../src/graph.rs) | Rational poles, tangent poles, jumps and removable holes in graph corpus. |
| M10 | [Parametric and polar modes](../src/job.rs) | Gallery execution for self-intersections and singularities. Full independent parametric error corpus remains open. |
| M11 | [Implicit cells](../src/graph.rs) | Interval exclusion, monotone crossing witnesses and sampled ambiguity. |
| M12 | [Predicate enclosure](../src/value.rs) | Exact and interval comparisons distinguish strict and closed boundaries. |
| M13 | [Regular roots](../src/calculus.rs) | Endpoint and derivative evidence, interval Newton and fallback bisection. |
| M14 | [Sturm isolation](../src/polynomial.rs) | Independent known roots and multiplicities for 512 rational polynomials. |
| M15 | [Feature operations](../src/job.rs) | Exposed residual equations for intersections, stationary extrema, tangencies and area. |
| M16 | [Certified panels](../src/calculus.rs), [heuristic estimate](../src/quadrature.rs) | Finite reference integrals and separate approximate error labels. |
| M17 | [Improper families](../src/improper.rs) | Power and exponential tails, finite zero-endpoint power rule and explicit divergence. |
| M18 | [Derivative inspection](../src/job.rs) | Symbolic and forward bounds, finite difference and conditioning comparison. |
| M19 | [Complex coloring](../src/coloring.rs) | Principal logarithm and square-root branch fixtures, complex poles and sampled rasters. |
| M20 | [Spatial meshes](../src/spatial.rs) | Height, parametric and implicit meshes. Sphere and domain-hole references. Topology approximate. |
| M21 | [Vector fields and ODE](../src/ode.rs) | Embedded integration, event sign changes, stiffness diagnostic and optional local Picard tube. |
| M22 | [Exact matrix operations](../src/linear.rs), [floating factors](../src/floating.rs) | Exact determinant and solve, pivoted QR and Jacobi SVD rank and residual tests. |
| M23 | [Fit and residual pane](../web/src/analysis-table.ts) | Editable data, polynomial basis, residual plot and estimated standard errors. |
| M24 | [Normal statistics](../src/statistics.rs) | Directed normal tails, quantile and probability plot. Other distribution families remain open. |
| M25 | [Dimensions](../src/value.rs) | Unit conversion, integer dimensions after powers and explicit angle handling. |
| M26 | [Viewport and snapping](../web/src/main.ts) | Anchor zoom, keyboard pan, snapping and deterministic axes. Extreme ranges limited by binary64 coordinate storage. |
| M27 | [Expression workspace](../web/index.html) | Diagnostic ranges, math keyboard, folders, visibility and style. |
| M28 | [Linked workspace panes](../web/src/main.ts) | 2D, 3D and table share document parameters and source. |
| M29 | [Worker scheduling](../web/src/engine.ts) | Generation guard, termination and bounded signature cache. Definition and parameter changes invalidate conservatively. |
| M30 | [Document schema](../web/src/model.ts) | Migration, undo, redo, validated import and last-valid backup. |
| M31 | [Plot exports](../web/src/renderer.ts), [PNG metadata](../web/src/png.ts) | Independent metadata decoding, viewport and uncertainty retained. |
| M32 | [Inspector](../web/src/main.ts) | Scalar and calculus statuses, assumptions, witnesses and download. |
| M33 | [Parity probes](../scripts/parity.mjs) | Native/WASM serialized equality and Chromium signed-zero check. |
| M34 | [Independent oracles](../scripts/oracles.py) | Arb, exact rational and analytic curve checks, rehashed controls and source-bound replay. |
| M35 | [Explorations](../web/src/gallery.ts) | 58 editable studies executed by the browser worker. |
| M36 | [Browser integration tests](../web/tests/workspace.spec.ts) | Edits recompute numerical results, trajectories, data analysis and graph geometry. |
