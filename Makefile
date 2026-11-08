PACKAGES = api-gateway share deposit trade-engine signal-gateway

.PHONY: help clean ci ci-check clippy test fmt fmt-fix frontend-install frontend-lint frontend-lint-fix frontend-test frontend-build frontend-ci dev proto prod-build prod logs logs-api-gateway restart test-api trade signal-gateway status seed seed-reset

help:
	@echo "CoinBot.v3 Makefile"
	@echo ""
	@echo "Getting Started"
	@echo "  make dev          Start all services (DB + Redis + API + worker + trade-engine + signal-gateway + opencode + frontend)"
	@echo ""
	@echo "Testing (starts Postgres + Redis automatically)"
	@echo "  make test         Run all Rust tests"
	@echo "  make frontend-test  Frontend tests only"
	@echo ""
	@echo "Trading"
	@echo "  make trade        Run trade-engine + signal-gateway live (needs DB + creds)"
	@echo "  make signal-gateway  Run the signal-gateway service (loads process/signal_gateway/.env)"
	@echo ""
	@echo "Development"
	@echo "  make logs         Follow logs from all services"
	@echo "  make logs-api-gateway Follow api_gateway logs only"
	@echo "  make restart      Recompile + restart Rust services (after .rs changes)"
	@echo "                    make restart SERVICE=api_gateway  (one service)"
	@echo "  make test-api     Run curl tests against the running API"
	@echo ""
	@echo "Production"
	@echo "  make prod-build   Build production images without running"
	@echo "  make prod         Pull images from ghcr (no build) + (re)start prod & opencode sandbox"
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
	@echo "  make ci           Local auto-fix pipeline (fmt-fix + lint-fix + tests)"
	@echo "  make ci-check     CI-equivalent checks (fmt + clippy + test + frontend lint/test/build)"
	@echo "                    Does not include the main-only prod/smoke job"
	@echo ""
	@echo "Cleanup"
	@echo "  make clean        Remove prod containers, images, and volumes"
	@echo "  make clean-all    Remove ALL containers, images, and volumes"
	@echo ""
	@echo "Seeding"
	@echo "  make seed         Insert demo data into local DB (4 users, 7 contracts, 8 orders)"
	@echo "  make seed-reset   Wipe and re-seed demo data"
	@echo "  make db           Open psql shell to the local DB"

ci: fmt-fix clippy test frontend-lint-fix frontend-test frontend-build

# Mirrors the PR checks in .github/workflows/ci.yml (backend + frontend jobs).
# It does not run the main-only prod image build / compose smoke test.
ci-check: fmt clippy test frontend-ci

fmt:
	cargo fmt $(addprefix -p ,$(PACKAGES)) -- --check

fmt-fix:
	cargo fmt $(addprefix -p ,$(PACKAGES))

clippy:
	cargo clippy $(addprefix -p ,$(PACKAGES)) --no-deps -- -D warnings

test:
	@[ -f .env ] || cp .env.ci .env; \
	 docker compose up -d postgres redis && \
		sleep 2 && \
		DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/coinbot cargo test $(addprefix -p ,$(PACKAGES)); \
		rst=$$?; \
		docker compose stop redis postgres; \
		exit $$rst

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

# One install, then the same lint/test/build sequence as the CI frontend job.
frontend-ci: frontend-install
	cd react && npm run lint
	cd react && npm run test
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
	docker compose restart $(or $(SERVICE),api_gateway deposit-worker trade-engine signal-gateway)

dev:
	docker compose up -d postgres redis && \
		docker compose up api_gateway deposit-worker trade-engine opencode signal-gateway frontend

prod-build:
	docker compose build deposit-worker-prod api_gateway-prod frontend-prod trade-engine-prod

# Pull pre-built images from ghcr (built by CI on main) and (re)start; never builds.
# First merge to main: image may not exist yet → docker compose build opencode
prod:
	docker compose --profile prod --profile tools pull --ignore-pull-failures
	docker compose --profile prod --profile tools up -d --no-build

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
		(cd process/signal_gateway && GRPC_TRADE_ENGINE_ENDPOINT=http://127.0.0.1:50051 cargo run -p signal-gateway); st=$$?; \
		kill $$PID 2>/dev/null; wait $$PID 2>/dev/null; \
		exit $$st); \
		docker compose stop postgres

signal-gateway:
	cd process/signal_gateway && cargo run -p signal-gateway

proto:
	cargo build -p share --timings

seed:
	@PGPASSWORD=postgres psql -h 127.0.0.1 -U postgres -d coinbot -f scripts/seed-demo.sql

seed-reset:
	@PGPASSWORD=postgres psql -h 127.0.0.1 -U postgres -d coinbot -c "TRUNCATE contracts, users RESTART IDENTITY CASCADE;"
	@PGPASSWORD=postgres psql -h 127.0.0.1 -U postgres -d coinbot -f scripts/seed-demo.sql

db:
	@PGPASSWORD=postgres psql -h 127.0.0.1 -U postgres -d coinbot
