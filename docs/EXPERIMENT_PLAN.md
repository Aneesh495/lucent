# Experiment protocol v1

The diagnostic campaign freezes input bytes through [the corpus manifest](../data/MANIFEST.json). Workload generation is deterministic, with enumerated indices in place of a random seed. The unit of analysis is a request and its saved response. Native and Emscripten outputs are retained separately. Native failures and comparisons against the independent oracle remain visible.

| Suite | Work | Reference and rule |
| --- | ---: | --- |
| Scalar | 10,000 requests | Python Fraction for exact results. FLINT Arb at 240 bits for transcendental containment. Require zero incorrect results and at least 95% of the regular subset within 1e-12 relative to max(1, magnitude). |
| Graph | 2,000 viewports | Rational pole and jump locations, analytic tan poles, and dense analytic checks on regular explicit segments. Require no forbidden joins and less than 0.5 CSS pixels sampled deviation. Require whole-view coverage, including unresolved cells. Implicit topology remains approximate. |
| Adversarial | 500 requests | Expected failure classification for invalid syntax, illegal domains, budget exhaustion, dimensions and possible poles. |
| Calculus | 160 requests | Analytic reference roots and integrals from Arb or exact rationals. Require zero invalid enclosures and at least 95% of regular brackets and enclosures within 1e-12 absolute width. |
| Browser | Four integration tests in protocol v1 | Chromium desktop and narrow screen. Test actual engine requests, undo, error replacement, local recovery and keyboard controls. |
| Verifier | Four semantic mutations | Run the independent command on authentic and modified claims. Assert nonzero rejection with the expected explanation. |

Graph deviation uses 17 equally spaced points per visible returned segment and measures vertical error in CSS pixels. Offscreen cells are counted separately and verified by an analytic sample check. The sampled check does not certify the maximum possible geometric error. The full acceptance profile additionally requires topology and accelerated renderer campaigns, broader polynomial and calculus corpora, browser worker parity, accessibility, reproducible export, performance and release checks. This protocol does not substitute diagnostics for those gates.

The local diagnostic host is an Apple M2 with 16 GiB and macOS. The locked Chromium shell is installed by Playwright. Browser wall-clock tests are bounded by the Playwright test timeout. Numeric subruns have a 30 minute hard timeout per backend and a 128 MiB Wasm linear-memory ceiling. No omitted or retried units count as successful. Keep failed runs and use new run IDs after changes.

### Oracle erratum

The first pilot flagged an exact rational square root at scalar case 08130 because the independent Arb representation of 1/3 necessarily has nonzero radius. The oracle now verifies exact square-root claims using the rational square identity and the principal sign. Workload bytes and tolerance rules are unchanged. The pilot remains in the evidence directory.
