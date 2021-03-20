# Architecture

Lucent has one mathematical engine and two request transports. The native executable reads bounded newline-delimited JSON from stdin. The browser loads the Rust static archive and C bridge through Emscripten inside a dedicated worker. The same Rust request dispatcher, AST, numerical types and graph algorithms run in both. The worker returns values and geometry to the TypeScript workspace. The browser composites the results on a Canvas grid and a WebGL2 plot layer, with a Canvas fallback.

## Ownership and boundaries

| Component | Responsibility |
| --- | --- |
| [Language](../src/lang/mod.rs) | Byte-spanned AST and diagnostics, Pratt parsing and stable serialization |
| [Definitions](../src/program.rs) | Lexical functions, free references and DAG validation |
| [Evaluation](../src/eval.rs) | Bounded execution, short-circuit piecewise expressions and typed values |
| [Exact arithmetic](../src/exact.rs) | Decimal-to-rational conversion and integer bit ceilings |
| [MPFR bridge](../native/bridge.c) | Opaque floating objects and one directed primitive per call |
| [Interval engine](../src/interval.rs) | Outward operations, range and domain classification |
| [Calculus](../src/calculus.rs) | Derivative witnesses, safeguarded refinement and quadrature coverage |
| [Algebraic roots](../src/polynomial.rs) | Exact polynomial coefficients, square-free factors and Sturm variation |
| [Rendering geometry](../src/graph.rs) | Explored 2D partitions, domain checks and curve segments |
| [Advanced computation](../src/spatial.rs) | Bounded surface stencils and octree isosurface geometry |
| [Worker and document](../web/src/engine.ts) | Generation identity, cancellation and asynchronous dispatch |
| [Independent evidence](../scripts/verify-evidence.py) | Trusted source binding, raw observations, oracle checks and bounded replay |

The AST and request input belong to the current request. MPFR objects own their allocations through Rust drop. The worker allocates the input bytes in WASM memory and frees them after the synchronous ABI call. The result buffer is borrowed until the next request on that worker. The worker copies the response before returning it to the interface.

The TypeScript document owns editable source, presentation properties, parameter values and view bounds. Geometry is transient and always carries the request that produced it. Old geometry can be displayed as a navigation preview while the viewport is changing. The status reflects that state and export waits for a settled generation. Editing mathematical content invalidates displayed evidence and cancels outstanding requests.

## Diagrams

| Source | Render |
| --- | --- |
| [System context](diagrams/context.dot) | [SVG](diagrams/context.svg) |
| [Numerical components](diagrams/components.dot) | [SVG](diagrams/components.svg) |
| [Ownership](diagrams/ownership.dot) | [SVG](diagrams/ownership.svg) |
| [Expression-to-pixel path](diagrams/data-path.dot) | [SVG](diagrams/data-path.svg) |
| [Worker concurrency](diagrams/concurrency.dot) | [SVG](diagrams/concurrency.svg) |
| [Result state lifecycle](diagrams/state.dot) | [SVG](diagrams/state.svg) |
| [Persistence](diagrams/persistence.dot) | [SVG](diagrams/persistence.svg) |
| [Domain subdivision](diagrams/domain.dot) | [SVG](diagrams/domain.svg) |
| [Evidence lineage](diagrams/evidence.dot) | [SVG](diagrams/evidence.svg) |
| [Failure and recovery](diagrams/recovery.dot) | [SVG](diagrams/recovery.svg) |
| [Deployment](diagrams/deployment.dot) | [SVG](diagrams/deployment.svg) |
| [Verification](diagrams/testing.dot) | [SVG](diagrams/testing.svg) |
