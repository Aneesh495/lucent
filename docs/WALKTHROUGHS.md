# Walkthroughs

## A pole that fits inside a pixel

Open **A pole smaller than a pixel** and inspect `1/(x - .000013)`. The [lexer](../src/lang/lexer.rs) keeps the decimal lexeme. The [exact conversion](../src/exact.rs) reduces it to a rational. The [range evaluator](../src/interval.rs) refuses division on each interval whose denominator might contain zero. The [curve engine](../src/graph.rs) splits that interval until its budget or spatial limit and leaves a marked gap. Pan, zoom or refine to move the bounds. Export plot data to see the cell partition and reason alongside the segment vertices.

## A repeated root on a requested endpoint

Open **Repeated roots** and calculate with the polynomial operation. The expression `(x-1)^2*(x+1)^3` on `[-1,2]` includes the left endpoint. The [polynomial engine](../src/polynomial.rs) extracts rational coefficients, factors by multiplicity, evaluates the endpoints and counts the roots of the square-free part. The root at -1 carries multiplicity three. The root at 1 carries multiplicity two. The [inspector](../web/src/main.ts) displays both the distinct count and the weighted count.

## A convergent and a divergent tail

Enter `1/x^3` and select **Integrate to infinity** with lower endpoint 2. The [tail recognizer](../src/improper.rs) reads the integer power and uses `2^(-2)/2 = 1/8`. Replacing the integrand with `1/x` reaches the divergent branch of the power test. Replacing it with `sin(x)/x` reaches the unknown branch. The three results have different statuses and different evidence.

## A stiff or undefined ODE

Open **Fast decay in phase space**. The fast component `-1000y` contributes a large initial Jacobian norm to the [trajectory solver](../src/ode.rs). Rejected trial steps reduce the step size. The inspector and exported trajectory distinguish local error from the optional first-step Picard box. Change the horizontal derivative to `1/(x-x)` and recompute. The engine spends its bounded work on undefined stages, returns partial with an explanation, and leaves the interface responsive. The [runner](../web/src/engine.ts) can terminate a superseded solve.

## Autosave recovery

A valid import first passes [schema decoding](../web/src/model.ts), including migration from v1. On save, the previous valid primary becomes the backup. When primary decoding later fails, the loader tries the backup and reports recovery. The [stress campaign](../scripts/workspace-stress.mjs) independently models edit, undo, redo and reload history across thousands of operations.
