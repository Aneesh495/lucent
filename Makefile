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

.PHONY: bootstrap web demo browser-test

bootstrap: bootstrap-wasm
	cd web && npm ci
	cd web && npx playwright install chromium --only-shell

web: wasm
	cd web && npm run build

demo: web
	cd web && npm run preview -- --port 4173

browser-test: web
	cd web && npm test

.PHONY: certificate-check
certificate-check: build
	python3 scripts/negative-controls.py "$$(mktemp -d .agent-local/controls.XXXXXXXX)/packets"
