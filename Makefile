PACKAGES = api-gateway share deposit trade-engine

.PHONY: help clean ci clippy test fmt fmt-fix frontend-install frontend-lint frontend-lint-fix frontend-test frontend-build dev prod proto prod-build logs logs-backend test-api trade analyzer-test

help:
	@echo "CoinBot.v3 Makefile"
	@echo ""
	@echo "Getting Started"
	@echo "  make dev          Start all services (API + worker + DB + Redis + frontend)"
	@echo ""
	@echo "Testing"
	@echo "  make test         Run all tests (Rust + Python, needs Redis running)"
	@echo "  make analyzer-test  Python analyzer unit tests only"
	@echo "  make benchmark-test  Benchmark unit tests only"
	@echo "  make frontend-test  Frontend tests only"
	@echo ""
	@echo "Trading"
	@echo "  make trade        Run trade-engine + Python analyzer live (needs DB + Redis)"
	@echo "  make benchmark    Backtest all tickers (no services needed, just .env)"
	@echo ""
	@echo "Development"
	@echo "  make logs         Follow logs from all services"
	@echo "  make logs-backend Follow backend logs only"
	@echo "  make test-api     Run curl tests against the running API"
	@echo ""
	@echo "Production"
	@echo "  make prod         Build + start all production services (detached)"
	@echo "  make prod-build   Build production images without running"
	@echo ""
	@echo "Rust"
	@echo "  make fmt          Check Rust formatting"
	@echo "  make fmt-fix      Fix Rust formatting"
	@echo "  make clippy       Lint Rust (deny warnings)"
	@echo ""
	@echo "Frontend"
	@echo "  make frontend-lint      Lint frontend"
	@echo "  make frontend-lint-fix  Auto-fix frontend lint"
	@echo "  make frontend-build     Build frontend for production"
	@echo ""
	@echo "CI"
	@echo "  make ci           Full pipeline (fmt + clippy + test + lint + build)"
	@echo ""
	@echo "Cleanup"
	@echo "  make clean        Remove all containers, images, and volumes"

ci: fmt-fix clippy test analyzer-test frontend-lint-fix frontend-test frontend-build

fmt:
	cargo fmt $(addprefix -p ,$(PACKAGES)) -- --check

fmt-fix:
	cargo fmt $(addprefix -p ,$(PACKAGES))

clippy:
	cargo clippy $(addprefix -p ,$(PACKAGES)) -- -D warnings

test:
	@docker compose up -d redis && \
		cargo test $(addprefix -p ,$(PACKAGES)); \
		rst=$$?; \
		cd process/analyzer && python3 -m pytest . -v; \
		pst=$$?; \
		docker compose stop redis; \
		exit $$(( rst + pst ))

analyzer-test:
	@cd process/analyzer && python3 -m pytest . -v

benchmark-test:
	@cd process/analyzer && python3 -m pytest benchmark/tests/ -v

benchmark:
	@cd process && python3 -m analyzer.benchmark.main

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

clean:
	docker compose down --rmi all -v

dev:
	docker compose up backend-dev deposit-worker trade-engine frontend redis

prod-build:
	docker compose build deposit-worker-prod backend frontend-prod

prod:
	docker compose --profile prod up -d deposit-worker-prod backend frontend-prod trade-engine-prod analyzer redis

logs:
	docker compose logs -f

logs-backend:
	docker compose logs backend -f

test-api:
	./scripts/test-api.sh

trade:
	@docker compose up -d redis && \
		(cd process && REDIS_HOST=127.0.0.1 REDIS_URL=redis://127.0.0.1:6379 python3 -m analyzer.main & \
		PID=$$!; sleep 2 && \
		REDIS_URL=redis://127.0.0.1:6379 cargo run -p trade-engine; st=$$?; \
		sleep 3; kill $$PID 2>/dev/null; wait $$PID 2>/dev/null; \
		exit $$st); \
		docker compose stop redis

proto:
	cargo build -p common --timings
