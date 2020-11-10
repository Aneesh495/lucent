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
