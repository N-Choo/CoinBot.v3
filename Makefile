PACKAGES = api-gateway

.PHONY: ci clippy test fmt fmt-fix frontend-install frontend-lint frontend-lint-fix frontend-test frontend-build

ci: fmt-fix clippy test frontend-lint-fix frontend-test frontend-build

fmt:
	cargo fmt $(addprefix -p ,$(PACKAGES)) -- --check

fmt-fix:
	cargo fmt $(addprefix -p ,$(PACKAGES))

clippy:
	cargo clippy $(addprefix -p ,$(PACKAGES)) -- -D warnings

test:
	cargo test -p api-gateway --lib

frontend-install:
	cd react && npm ci

frontend-lint: frontend-install
	cd react && npm run lint

frontend-lint-fix: frontend-install
	cd react && npm run lint -- --fix

frontend-test: frontend-install
	cd react && npm run test

frontend-build: frontend-install
	cd react && npm run build
