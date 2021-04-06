# Observed results

Executable source [c7da45b](https://github.com/Aneesh495/lucent/commit/c7da45b4cf7d1294098f7e3e21c885fb5c3f762f) was checked out from GitHub and built in an isolated directory. The `lucent-full-c7da45b` run recorded native, Node WASM, and real Chromium worker responses. A separate verifier checked all raw outcomes, replayed 17 frozen cases and re-executed the four semantic negative controls. The bundle verifier also rejected four deliberately corrupted or misbound variants.

| Measurement | Observed | Raw evidence in the bundle |
| --- | ---: | --- |
| Scalar containment and exact arithmetic | 10,000 of 10,000 correct, all useful | `scalar-outcomes.jsonl`, `scalar-{native,wasm,browser}.jsonl.gz` |
| Graph cases | 2,000 of 2,000, no forbidden joins | `graph-outcomes.jsonl`, `graph-{native,wasm,browser}.jsonl.gz` |
| Maximum sampled regular-curve deviation | 0.446895 CSS px | `graph-outcomes.jsonl` |
| Adversarial classifications | 500 of 500 | `adversarial-outcomes.jsonl` |
| Regular root and integral cases | 160 of 160 sound and useful | `calculus-outcomes.jsonl` |
| Known-root polynomials | 512 of 512, with multiplicities | `polynomials.jsonl` |
| Workspace stress | 5,500 operations, 524 backup recoveries | `workspace-stress.log` |
| Browser interaction checks | 11 passed, including two automated accessibility viewports | `browser-tests/results.json` |
| Gallery | 58 editable studies exercised | `gallery/gallery.json` |
| Certificate controls | 4 positive and 4 rehashed negative packets | `controls/controls.json` |
| Bundle controls | 4 rejected variants | `lucent-controls-c7da45b/results.json` |

The interaction profile used headless Chromium 158.0.8127.18 and WebGL through SwiftShader on an Apple M2 host. It measured 59.88 median FPS, 16.7 ms p95 frame time, 33.9 ms p95 first preview, 114.5 ms p95 refinement, and 0.2 ms p95 worker termination after supersession. The 60 FPS median threshold failed. The profile recorded that failure and full acceptance returned nonzero. The interaction JSON includes frame timestamps and the thresholds used.

The hosted preview served the exact executable commit. A Chromium smoke test loaded the WASM module from that deployment, observed the unresolved cells around the default pole, then computed the root of `x^2-2` on `[1,2]` through the inspector. It returned the bracket `[1.414213562373095, 1.4142135623730951]`. [Deployment run](https://github.com/Aneesh495/lucent/actions/runs/42702013189). The same source passed hosted Linux build and browser integration checks. [Validation run](https://github.com/Aneesh495/lucent/actions/runs/42702011974).

## Verification

With the archive extracted inside `evidence/` in a clean checkout, run:

```sh
git switch --detach c7da45b4cf7d1294098f7e3e21c885fb5c3f762f
make bootstrap-oracle
make verify EVIDENCE=evidence/lucent-full-c7da45b
```

This verifies the diagnostic scope of the evidence. The full release still requires the unresolved gates in [build status](BUILD_STATUS.md) and the remaining capabilities in [the coverage index](COVERAGE.md).

Archive SHA-256 `8efd8d48ba0a3d7234c1e148d772389240e608456da30e3abb4b33623db1ef50`.
