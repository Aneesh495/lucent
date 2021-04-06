# Build status

Lucent has a working native CLI, a browser WASM workspace, and a hosted preview. Full acceptance remains incomplete. The supported contracts and open questions are recorded in [the specification](SPECIFICATION.md) and [experiment plan](EXPERIMENT_PLAN.md).

| Gate | Current evidence | Status |
| --- | --- | --- |
| G01 Semantics | Focused parser, precedence, scope and exact-value tests | Passed on tested language subset |
| G02 Interval soundness | Frozen scalar corpus checked with rational arithmetic and Arb across native, WASM and Chromium | Passed on frozen scalar corpus |
| G03 Usefulness | Frozen regular scalar and calculus fixtures checked for finite, useful bounds | Passed on declared diagnostic subset |
| G04 Discontinuities | Rational poles, removable holes, tangent poles and piecewise jumps in the frozen graph corpus | Passed on frozen graph corpus |
| G05 Curve accuracy | Sampled error on regular explicit graph fixtures | Passed on frozen viewports |
| G06 Roots | Repeated-root fixtures, regular-root witnesses and a separate known-root polynomial oracle | Broader edge coverage unverified |
| G07 Calculus | 160 finite-calculus fixtures plus targeted derivative and improper-tail cases | Broader improper and conditioning cases unverified |
| G08 Complex, 3D and ODE | Branch-cut, sphere, surface-hole and trajectory-event tests | Full campaign unverified |
| G09 Interaction | Headless Chromium performance observations | Failed at 59.88 median FPS in the exact-source audit |
| G10 Cancellation | Measured worker termination and later-generation checks | Passed in the headless audit |
| G11 Workspace | Versioned migration and 5,500 generated operations with recovery | Passed in the diagnostic |
| G12 Accessibility | Keyboard, focus and zero automated violations at two Chromium viewports | Comprehensive accessibility audit unverified |
| G13 Export | Independently decoded JSON, SVG and PNG metadata plus certificate checks | Independent pixel and 3D reconstruction unverified |
| G14 Semantic verification | Authentic and rehashed altered claims through the CLI and bundle verifier | Passed for covered claim types |
| G15 Reproduction | Clean source builds, real Chromium worker corpus and editable gallery | Complete capability matrix unverified |
| G16 Release | Hosted preview available | Incomplete |

The [observed results](FINDINGS.md) identify the executable source, commands, measurements and raw evidence. [Capability coverage](COVERAGE.md) links the individual subsystems. See [reproduction and recovery](RECOVERY.md) for exact local commands and evidence handling.
