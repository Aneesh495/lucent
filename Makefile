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

ORACLE_PYTHON ?= .agent-local/oracle/bin/python
RUN_ID ?= diagnostic-$(shell date -u +%Y%m%dT%H%M%SZ)
PROFILE ?= diagnostic

.PHONY: bootstrap-oracle data numeric-corpus graph-corpus acceptance
bootstrap-oracle:
	uv venv --python 3.13 .agent-local/oracle
	uv pip install --python $(ORACLE_PYTHON) python-flint==0.8.0

data:
	python3 scripts/workloads.py .agent-local/corpus

numeric-corpus: build wasm
	$(ORACLE_PYTHON) scripts/campaign.py --run-id "$(RUN_ID)" --profile subset --suites scalar adversarial calculus

graph-corpus: build wasm
	$(ORACLE_PYTHON) scripts/campaign.py --run-id "$(RUN_ID)" --profile subset --suites graph

acceptance: build wasm
	$(ORACLE_PYTHON) scripts/campaign.py --run-id "$(RUN_ID)" --profile "$(PROFILE)"
