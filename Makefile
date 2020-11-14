.PHONY: doctor build test

doctor:
	@rustc --version
	@node --version
	@pkg-config --modversion mpfr gmp
	@uname -sm
	@df -h .

build:
	cargo build --locked --release

test:
	cargo test --locked

.PHONY: bootstrap-wasm wasm parity

bootstrap-wasm:
	bash scripts/bootstrap-wasm.sh

wasm:
	bash scripts/build-wasm.sh

parity: build wasm
	node scripts/parity.mjs
