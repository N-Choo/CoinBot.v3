PACKAGES = api-gateway

.PHONY: help ci clippy test fmt fmt-fix frontend-install frontend-lint frontend-lint-fix frontend-test frontend-build dev prod prod-build logs logs-backend test-api

help:
	@echo "Usage: make <target>"
	@echo ""
	@echo "Development"
	@echo "  dev          Start backend-dev (cargo run) + frontend (Vite HMR)"
	@echo "  test-api     Run curl tests against the API"
	@echo "  logs         Follow logs from all services"
	@echo "  logs-backend Follow backend logs"
	@echo ""
	@echo "Production"
	@echo "  prod         Build + start production backend + frontend (detached)"
	@echo "  prod-build   Build production images without running"
	@echo ""
	@echo "Rust"
	@echo "  fmt          Check formatting"
	@echo "  fmt-fix      Fix formatting"
	@echo "  clippy       Lint (deny warnings)"
	@echo "  test         Run unit tests"
	@echo ""
	@echo "Frontend"
	@echo "  frontend-lint      Lint frontend"
	@echo "  frontend-lint-fix  Auto-fix frontend lint"
	@echo "  frontend-test      Run frontend tests"
	@echo "  frontend-build     Build frontend for production"
	@echo ""
	@echo "CI"
	@echo "  ci           Run full CI pipeline (fmt + clippy + test + lint + build)"

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

dev:
	docker compose up --no-deps backend-dev frontend

prod-build:
	docker compose build backend frontend-prod

prod:
	docker compose -f docker-compose.yml --profile prod up -d backend frontend-prod

logs:
	docker compose logs -f

logs-backend:
	docker compose logs backend -f

test-api:
	./test-api.sh
