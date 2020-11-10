# Numerical contract

Exact real literals represent integers or rational numbers with source provenance. Enclosures use directed MPFR arithmetic over closed domains. An interval that crosses an excluded point requires subdivision or an unresolved result. A sign change establishes a root only over a domain with proved continuity. Uniqueness additionally requires a derivative that excludes zero, or an exact algebraic argument.

Rendered geometry carries its own error model and coverage. A line segment cannot bridge a cell whose domain validity is unresolved. Geometry is approximate unless a separate bound is available.

Lucent will use the same Rust evaluator and C bridge on native and WebAssembly targets. GMP and MPFR supply arithmetic primitives. The expression model, subdivision, and proof conditions belong to Lucent.
