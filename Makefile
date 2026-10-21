PACKAGES = api-gateway share deposit trade-engine strategy-analyzer analyzer-rs

.PHONY: help clean ci clippy test strategy-check fmt fmt-fix frontend-install frontend-lint frontend-lint-fix frontend-test frontend-build dev prod proto prod-build logs logs-api-gateway restart test-api trade analyzer-test benchmark-test benchmark benchmark-web status seed seed-reset analyzer-rs-check analyzer-rs-test

help:
	@echo "CoinBot.v3 Makefile"
	@echo ""
	@echo "Getting Started"
	@echo "  make dev          Start all services (DB + Redis + API + worker + frontend)"
	@echo ""
	@echo "Testing (starts Postgres + Redis automatically)"
	@echo "  make test         Run all Rust tests"
	@echo "  make analyzer-test  Python analyzer unit tests only"
	@echo "  make benchmark-test  Benchmark unit tests only"
	@echo "  make frontend-test  Frontend tests only"
	@echo "  make analyzer-rs-check  Rust analyzer core: fmt + clippy + tests"
	@echo "  make analyzer-rs-test   Rust analyzer tests only"
	@echo ""
	@echo "Trading"
	@echo "  make trade        Run trade-engine + Rust strategy-analyzer live (needs DB + creds)"
	@echo "  make benchmark    Backtest all tickers (no services needed, just .env)"
	@echo "  make benchmark-web  Backtest and write a self-contained HTML replay report"
	@echo ""
	@echo "Development"
	@echo "  make logs         Follow logs from all services"
	@echo "  make logs-api-gateway Follow api_gateway logs only"
	@echo "  make restart      Recompile + restart Rust services (after .rs changes)"
	@echo "                    make restart SERVICE=api_gateway  (one service)"
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
	@echo "  make seed         Insert demo data into local DB (4 users, 6 contracts, 8 orders)"
	@echo "  make seed-reset   Wipe and re-seed demo data"
	@echo "  make db           Open psql shell to the local DB"

ci: fmt-fix clippy test strategy-check frontend-lint-fix frontend-test frontend-build

fmt:
	cargo fmt $(addprefix -p ,$(PACKAGES)) -- --check

fmt-fix:
	cargo fmt $(addprefix -p ,$(PACKAGES))

clippy:
	cargo clippy $(addprefix -p ,$(PACKAGES)) --no-deps -- -D warnings
	cargo clippy -p strategy-analyzer --all-features --no-deps -- -D warnings

test:
	@[ -f .env ] || cp .env.ci .env; \
	 docker compose up -d postgres redis && \
		sleep 2 && \
		DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/coinbot cargo test $(addprefix -p ,$(PACKAGES)) && \
		DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/coinbot cargo test -p strategy-analyzer --all-features; \
		rst=$$?; \
		docker compose stop redis postgres; \
		exit $$rst

analyzer-test:
	@cd process/analyzer && python3 -m pytest . -v

analyzer-rs-check:
	cargo fmt -p analyzer-rs -- --check
	cargo clippy -p analyzer-rs --all-targets --no-deps -- -D warnings
	cargo test -p analyzer-rs

analyzer-rs-test:
	cargo test -p analyzer-rs

benchmark-test:
	@cd process/analyzer && python3 -m pytest benchmark/tests/ -v

benchmark:
	@cd process && python3 -m analyzer.benchmark.main

benchmark-web:
	@cd process && python3 -m analyzer.benchmark.main --web

# Purity gate for the deterministic core: no float arithmetic and no hash-map iteration
# under src/strategy. The greps below are fast pre-filters that strip trailing `//` comments;
# the authoritative check is the `syn` token-level test (tests/purity.rs), which still catches
# a real `f64` on a line that carries a trailing comment and does not flag comments/strings.
strategy-check:
	@echo "checking strategy core purity..."
	@! grep -RInE '\b(f32|f64)\b' process/strategy-analyzer/src/strategy --include='*.rs' | sed 's|//.*||' | grep -qE '\b(f32|f64)\b' || (echo "float type in core" && exit 1)
	@! grep -RInE '\b(HashMap|HashSet)\b' process/strategy-analyzer/src/strategy --include='*.rs' | sed 's|//.*||' | grep -qE '\b(HashMap|HashSet)\b' || (echo "hash-map in core" && exit 1)
	cargo test -p strategy-analyzer --no-default-features --test purity
	cargo build -p strategy-analyzer --no-default-features
	cargo build -p strategy-analyzer --all-features

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
	docker compose --profile dev --profile prod down --rmi all --volumes --remove-orphans
	docker volume prune -f

status:
	@docker compose ps 2>/dev/null

restart:
	docker compose restart $(or $(SERVICE),api_gateway deposit-worker trade-engine strategy-analyzer)

dev:
	docker compose up -d postgres redis && \
		docker compose up api_gateway deposit-worker trade-engine strategy-analyzer frontend

prod-build:
	docker compose build deposit-worker-prod api_gateway-prod frontend-prod strategy-analyzer-prod

prod:
	docker compose --profile prod up -d deposit-worker-prod api_gateway-prod frontend-prod trade-engine-prod strategy-analyzer-prod redis postgres

logs:
	docker compose logs -f

logs-api-gateway:
	docker compose logs api_gateway -f

test-api:
	./scripts/test-api.sh

trade:
	@docker compose up -d postgres && \
		sleep 2 && \
		(GRPC_ADDR=127.0.0.1:50051 DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/coinbot cargo run -p trade-engine & \
		PID=$$!; sleep 5 && \
		GRPC_TRADE_ENGINE_ENDPOINT=http://127.0.0.1:50051 LIVE_SYMBOLS=XRP-USDT LIVE_CANDLE_INTERVAL=15min \
		cargo run -p strategy-analyzer --features "harness live grpc" --bin serve -- live; st=$$?; \
		kill $$PID 2>/dev/null; wait $$PID 2>/dev/null; \
		exit $$st); \
		docker compose stop postgres

proto:
	cargo build -p common --timings

seed:
	@PGPASSWORD=postgres psql -h 127.0.0.1 -U postgres -d coinbot -f scripts/seed-demo.sql

seed-reset:
	@PGPASSWORD=postgres psql -h 127.0.0.1 -U postgres -d coinbot -c "TRUNCATE contracts, users RESTART IDENTITY CASCADE;"
	@PGPASSWORD=postgres psql -h 127.0.0.1 -U postgres -d coinbot -f scripts/seed-demo.sql

db:
	@PGPASSWORD=postgres psql -h 127.0.0.1 -U postgres -d coinbot
