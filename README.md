# Lucent

Lucent is an interactive graphing workbench with an original Rust engine. Edit an expression to see exact rational values, directed MPFR enclosures, adaptive geometry or explicitly unresolved cells.

[Open the browser preview](https://aneesh495.github.io/lucent/)

## Try it

Plot `1/(x-.000013)`. The curve stays separated at the pole, and the uncertainty overlay tracks the unresolved cell as you zoom. For an exact multiplicity result, open **Repeated roots** and calculate in the inspector. The gallery also includes complex branches, phase-plane trajectories, surfaces, residual plots and normal probability plots.

```sh
make doctor
make bootstrap
make demo
```

Open the local URL printed by the server. The browser build uses Rust 1.88.0, Emscripten 4.0.10 and Node 24. For the native CLI, install GMP 6.3.0, MPFR 4.2.2 and pkg-config, or build the pinned libraries as described in the [recovery guide](docs/RECOVERY.md).

```sh
make build
printf '%s\n' '{"op":"evaluate","expr":"1/3+.2"}' | target/release/lucent
printf '%s\n' '{"op":"roots","expr":"x^2=2","domain":[1,2]}' | target/release/lucent
```

The first request gives `8/15`. The second includes a bracket for the positive root, the endpoint signs and the derivative enclosure used for uniqueness.

## Mechanics

- [Expression parser](src/lang/parser.rs) and [lexical dependency graph](src/program.rs)
- [Directed interval evaluation](src/interval.rs), [root and integral witnesses](src/calculus.rs), and [exact polynomial isolation](src/polynomial.rs)
- [Adaptive graph cells](src/graph.rs), [spatial mesh](src/spatial.rs), and [browser worker](web/src/engine.ts)
- [Evidence protocol](docs/EXPERIMENT_PLAN.md), [independent verifier](scripts/verify-evidence.py), and [build status](docs/BUILD_STATUS.md)

The frozen diagnostic covers 10,000 scalar, 2,000 graph, 500 adversarial and 160 calculus cases. The worst independently sampled deviation on the regular explicit curves was 0.447 CSS pixels. Implicit chords and 3D meshes remain approximate geometry. The hosted preview is under active development.

[Language and schema](docs/SPECIFICATION.md) · [Architecture and diagrams](docs/ARCHITECTURE.md) · [Numerical notes](docs/NOTES.md) · [Walkthroughs](docs/WALKTHROUGHS.md) · [Design decisions](docs/DECISIONS.md)
