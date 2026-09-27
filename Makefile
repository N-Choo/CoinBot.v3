PACKAGES = api-gateway share deposit trade-engine

.PHONY: help clean ci clippy test fmt fmt-fix frontend-install frontend-lint frontend-lint-fix frontend-test frontend-build dev prod proto prod-build logs logs-api-gateway test-api trade analyzer-test status seed seed-reset

help:
	@echo "CoinBot.v3 Makefile"
	@echo ""
	@echo "Getting Started"
	@echo "  make dev          Start all services (DB + Redis + API + worker + frontend)"
	@echo ""
	@echo "Testing (starts Postgres + Redis automatically)"
	@echo "  make test         Run all tests (Rust + Python)"
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
	@echo "  make logs-api-gateway Follow api_gateway logs only"
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
	@echo "  make clean        Remove prod containers, images, and volumes"
	@echo "  make clean-all    Remove ALL containers, images, and volumes"
	@echo ""
	@echo "Seeding"
	@echo "  make seed         Insert demo data into local DB (3 users, 5 contracts)"
	@echo "  make seed-reset   Wipe and re-seed demo data"
	@echo "  make db           Open psql shell to the local DB"

ci: fmt-fix clippy test analyzer-test frontend-lint-fix frontend-test frontend-build

fmt:
	cargo fmt $(addprefix -p ,$(PACKAGES)) -- --check

fmt-fix:
	cargo fmt $(addprefix -p ,$(PACKAGES))

clippy:
	cargo clippy $(addprefix -p ,$(PACKAGES)) -- -D warnings

test:
	@[ -f .env ] || cp .env.ci .env; \
	 docker compose up -d postgres redis && \
		sleep 2 && \
		DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/coinbot cargo test $(addprefix -p ,$(PACKAGES)); \
		rst=$$?; \
		if [ -d process/analyzer ]; then (cd process/analyzer && python3 -m pytest . -v); pst=$$?; else echo "skipping analyzer tests (process/analyzer absent)"; pst=0; fi; \
		docker compose stop redis postgres; \
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
	docker compose --profile prod stop
	docker compose --profile prod down --rmi all --volumes --remove-orphans

clean-all:
	docker compose stop
	docker compose down --rmi all --volumes --remove-orphans
	docker volume prune -f

status:
	@docker compose ps 2>/dev/null

dev:
	docker compose up -d postgres redis && \
		docker compose up api_gateway deposit-worker trade-engine frontend

prod-build:
	docker compose build deposit-worker-prod api_gateway frontend-prod

prod:
	docker compose --profile prod up -d deposit-worker-prod api_gateway-prod frontend-prod trade-engine-prod analyzer redis postgres

logs:
	docker compose logs -f

logs-api-gateway:
	docker compose logs api_gateway -f

test-api:
	./scripts/test-api.sh

trade:
	@docker compose up -d postgres redis && \
		sleep 2 && \
		(cd process && DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/coinbot REDIS_HOST=127.0.0.1 REDIS_URL=redis://127.0.0.1:6379 python3 -m analyzer.main & \
		PID=$$!; sleep 2 && \
		DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/coinbot REDIS_URL=redis://127.0.0.1:6379 cargo run -p trade-engine; st=$$?; \
		sleep 3; kill $$PID 2>/dev/null; wait $$PID 2>/dev/null; \
		exit $$st); \
		docker compose stop redis postgres

proto:
	cargo build -p common --timings

seed:
	@PGPASSWORD=postgres psql -h 127.0.0.1 -U postgres -d coinbot -f scripts/seed-demo.sql

seed-reset:
	@PGPASSWORD=postgres psql -h 127.0.0.1 -U postgres -d coinbot -c "TRUNCATE contracts, users RESTART IDENTITY CASCADE;"
	@PGPASSWORD=postgres psql -h 127.0.0.1 -U postgres -d coinbot -f scripts/seed-demo.sql

db:
	@PGPASSWORD=postgres psql -h 127.0.0.1 -U postgres -d coinbot
