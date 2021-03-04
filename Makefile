.PHONY: doctor build test

doctor:
	bash scripts/doctor.sh

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

acceptance: build web
	$(ORACLE_PYTHON) scripts/campaign.py --run-id "$(RUN_ID)" --profile "$(PROFILE)"

.PHONY: bootstrap-native
bootstrap-native:
	bash scripts/bootstrap-native.sh

.PHONY: pages
pages:
	bash scripts/package-pages.sh /lucent/

.PHONY: verify
verify: build
	@test -n "$(EVIDENCE)" || { echo 'Set EVIDENCE to a campaign directory'; exit 2; }
	$(ORACLE_PYTHON) scripts/verify-evidence.py "$(EVIDENCE)" $(if $(EXPECTED_COMMIT),--expected-commit "$(EXPECTED_COMMIT)")

.PHONY: stress
stress:
	node scripts/workspace-stress.mjs

.PHONY: gallery polynomial-oracle
gallery: web
	cd web && npx playwright test --config playwright.gallery.config.ts

polynomial-oracle: build
	python3 scripts/polynomial-oracle.py "$$(mktemp -d .agent-local/polynomial.XXXXXXXX)/observations.jsonl"

.PHONY: docs
docs:
	python3 scripts/check-docs.py

.PHONY: bench
bench: web
	cd web && npx playwright test --config playwright.bench.config.ts

.PHONY: release
release: build
	@test -n "$(EVIDENCE)" || { echo 'A full acceptance evidence directory is required'; exit 2; }
	$(ORACLE_PYTHON) scripts/verify-evidence.py "$(EVIDENCE)" --require-full
