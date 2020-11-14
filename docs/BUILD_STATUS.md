# Build status

The public repository and numerical ABI are established. Native and Emscripten builds use MPFR 4.2.2 and GMP 6.3.0 on the development host. The Node WebAssembly probe matches native JSON for seven directed-rounding requests, including a subnormal bound, exact unity, and range failures. This is ABI evidence. The full native and browser numerical corpus is pending.

Next increment: connect the source-spanned parser, exact literals and interval evaluator.
