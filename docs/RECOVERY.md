# Reproduction and recovery

## Local prerequisites

Use Git, Make, rustup, Node 24, Python 3.13, a C compiler, pkg-config, m4, xz and Graphviz. The tested development host is macOS arm64 with MPFR 4.2.2 and GMP 6.3.0. Use `make bootstrap-native` to build the pinned native libraries, then set `LUCENT_MPFR_NATIVE="$PWD/.agent-local/toolchain/native"` for subsequent native commands. A system installation discovered by pkg-config also works when the pinned libraries are present.

`make bootstrap` installs the browser build tools and Playwright Chromium. `make bootstrap-oracle` uses uv to set up Python FLINT. `make doctor` prints toolchain and host details. If a browser is missing, run the locked Playwright install step again. On Linux, add the system browser dependencies with `cd web && npx playwright install --with-deps chromium --only-shell`.

## Verify a diagnostic campaign

From a clean checkout at the executable source commit, run:

```sh
make acceptance RUN_ID=local-diagnostic-1 PROFILE=diagnostic
make verify EVIDENCE=evidence/local-diagnostic-1
```

The first command creates a new directory. Preserve it if a failure occurs. The verifier requires the checked-out executable source to match the bundle and independently checks fixture hashes, bounds, sampled geometry, raw command exits and frozen native replay probes. It rejects missing artifacts, changed source and replay mismatches. `PROFILE=full` records the implemented workload and exits nonzero while mandatory acceptance gates remain open.

## Recover from a worker or document failure

If an expression remains unresolved, inspect the domain and reason before increasing work. Reducing the viewport or splitting a calculus domain can separate an excluded point from a regular region. Superseding a running computation terminates the worker and creates a new ticket. The current expression is retained when a worker times out.

An invalid import does not replace the current document. A corrupt primary autosave triggers last-valid backup recovery. Export the recovered document before further edits if the failed save must be investigated. A file with an unknown version requires explicit migration.

For a failed native or WASM toolchain build, check the compiler identity and the corresponding log under the toolchain cache. Fetch commands verify the GMP and MPFR tarballs against the pinned digests. Recreate the failed build prefix only after preserving the configure and build logs.

## Contributing

Keep mathematical claims and output statuses aligned with the implementation. Include an independent expected result for a numerical change, particularly at a domain boundary. Run the focused tests first, then `make test parity certificate-check browser-test` and `make docs`. Changes to frozen workload definitions require a new protocol and retained evidence from the earlier run. Keep build outputs and local recovery state out of Git.
