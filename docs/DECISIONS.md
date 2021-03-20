# Architecture decision records

| ADR | Decision, alternative and consequence |
| --- | --- |
| 01. Core language | Use Rust for typed expressions, numerical composition and bounded execution. A TypeScript-only evaluator would avoid cross-compilation but require a second native implementation or a JavaScript CLI. One Rust core makes shared semantics testable. |
| 02. Floating primitives | Use MPFR via a narrow C ABI. Implementing correctly rounded elementary functions here would consume the work needed for expression semantics and validation. A narrow bridge keeps interval propagation and proofs inspectable in Rust. |
| 03. Browser ABI | Link Rust and the same versioned GMP/MPFR C libraries with Emscripten. A browser-only f64 backend would be quicker to build and could not carry the native enclosure contract. The pinned cross-build and relink archive add packaging work. |
| 04. Literal values | Parse decimal notation as a rational. Parsing through binary64 would confuse input meaning with machine encoding. Exact values use explicit bit and token limits to bound arithmetic. |
| 05. Interval error algebra | Distinguish fully excluded, possibly split, unsupported and exhausted. A single NaN or missing point would erase useful domain information. Distinct states let graphing refine uncertain cells and retain gaps. |
| 06. Lexical definitions | Validate free names with a topological graph before evaluation. Dynamic lookup from caller frames would make a function's dependencies unstable. Function parameters shadow globals only inside the function body. |
| 07. Root proof | Combine endpoint signs and derivative exclusion, with interval Newton as a refinement. A dense sign-change scan would miss repeated roots and can cross poles. The exact polynomial path covers multiplicity in its separate finite language. |
| 08. Integration | Use fourth-derivative Simpson enclosures and an interval range fallback. A heuristic error estimate alone would allow a missed spike to look certified. Unsupported panels can force an incomplete outcome even when a heuristic could return a number. |
| 09. Curve visibility | Emit a full subdivision partition, including offscreen and unresolved cells. A compact point strip is easy to render and makes excluded domains unrecoverable. Layer exports preserve what the engine actually explored. |
| 10. Implicit geometry | Use interval exclusion and sampled edge chords with explicit topology uncertainty. A uniform marching-squares grid would mask tiny components and ambiguous cells. A refined grid still leaves undecidable geometry under a finite budget. |
| 11. Cancellation | Terminate a worker that is running a superseded synchronous call. Posting a cancel message to the same busy worker cannot preempt that call. A generation ticket protects the main thread from old responses even after worker turnover. |
| 12. Storage | Keep a versioned local document with a last-valid backup. A remote service would require authentication and complicate reproducibility. Local storage limits must be handled and exports provide portable copies. |
| 13. Evidence | Freeze workload bytes by hash and verify independently from raw observations with bounded replay. A manifest alone detects corruption, while rehashed semantic mutations require mathematical checks. |
| 14. Deployment | Deploy a static preview through GitHub Pages with the linked engine sources and a relinkable archive. A server calculator would make browser results dependent on network and service state. The static site can compute locally once loaded. |
