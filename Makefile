PACKAGES = api-gateway share deposit trade-engine signal-gateway

.PHONY: help ci ci-test dev prod seed seed-clean

help:
	@echo "CoinBot.v3 Makefile"
	@echo ""
	@echo "  make dev       Start the dev stack (DB + Redis + all services + frontend)"
	@echo "  make prod      Pull pre-built images and (re)start the prod stack"
	@echo "  make seed      Insert demo data into the local DB"
	@echo "  make seed-clean  Wipe demo data from the local DB"
	@echo "  make ci-test   CI-equivalent checks (fmt + clippy + tests + frontend)"
	@echo "  make ci        Local auto-fix pipeline (fmt-fix + lint-fix + tests + build)"

ci: fmt-fix clippy test frontend-lint-fix frontend-test frontend-build

ci-test: fmt clippy test frontend-ci

# --- Internal helpers (used by ci / ci-test / GitHub Actions) ---

fmt:
	cargo fmt $(addprefix -p ,$(PACKAGES)) -- --check

fmt-fix:
	cargo fmt $(addprefix -p ,$(PACKAGES))

clippy:
	cargo clippy $(addprefix -p ,$(PACKAGES)) --no-deps -- -D warnings

test:
	@[ -f .env ] || cp .env.ci .env; \
	 pg_user=$$(sed -n 's/^POSTGRES_USER=//p' .env | tail -1 | tr -d '"'); \
	 pg_password=$$(sed -n 's/^POSTGRES_PASSWORD=//p' .env | tail -1 | tr -d '"'); \
	 pg_db=$$(sed -n 's/^POSTGRES_DB=//p' .env | tail -1 | tr -d '"'); \
	 docker compose up -d postgres redis && \
		sleep 2 && \
		DATABASE_URL="postgres://$${pg_user:-postgres}:$${pg_password:-postgres}@127.0.0.1:5432/$${pg_db:-coinbot}" cargo test $(addprefix -p ,$(PACKAGES)); \
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

frontend-ci: frontend-install
	cd react && npm run lint
	cd react && npm run test
	cd react && npm run build

# --- Public lifecycle targets ---

dev:
	docker compose up -d postgres redis && \
		docker compose up api_gateway deposit-worker trade-engine opencode signal-gateway frontend

prod-build:
	docker compose build deposit-worker-prod api_gateway-prod frontend-prod trade-engine-prod

# Pull pre-built images from ghcr (built by CI on main) and (re)start; never builds.
prod:
	docker compose --profile prod --profile tools pull --ignore-pull-failures
	docker compose --profile prod --profile tools up -d --no-build

test-api:
	./scripts/test-api.sh

seed:
	@PGPASSWORD=postgres psql -h 127.0.0.1 -U postgres -d coinbot -f scripts/seed-demo.sql

seed-clean:
	@PGPASSWORD=postgres psql -h 127.0.0.1 -U postgres -d coinbot -c "TRUNCATE users, contracts, trade_orders, deposits RESTART IDENTITY CASCADE;"
