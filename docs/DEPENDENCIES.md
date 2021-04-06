# Dependencies

Rust 1.88.0 is the pinned compiler. The browser backend targets Emscripten 4.0.10. The native bridge expects GMP 6.3.0 and MPFR 4.2.2. GMP and MPFR can be redistributed under LGPLv3+ subject to their license terms. Keep their original notices and corresponding source available for any distributed linked binary, and retain a path to relink with a modified library.

References: [GNU MPFR manual](https://www.mpfr.org/mpfr-current/mpfr.html), [FLINT interval calculus](https://flintlib.org/doc/arb_calc/index.html), [MathML Core](https://www.w3.org/TR/mathml-core/), [Rust Emscripten target](https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-emscripten.html).

The hosted preview packages the exact arithmetic library source archives, licenses, Lucent source, and relinkable application archive. Its license page includes checksums and the relink command. [Hosted engine sources](https://aneesh495.github.io/lucent/licenses/index.html).
