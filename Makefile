PACKAGES = api-gateway share deposit trade-engine signal-gateway withdraw-worker
COMPOSE_ENV_FILE ?= .env
COMPOSE_DEV = docker compose -f docker-compose.yml -f docker-compose.dev.yml
COMPOSE_TEST = docker compose -p coinbot-test --env-file .env.ci -f docker-compose.yml -f docker-compose.test.yml
COMPOSE_PROD = docker compose --env-file $(COMPOSE_ENV_FILE) -f docker-compose.yml -f docker-compose.prod.yml
LOG_TAIL ?= 200

.PHONY: help ci ci-test dev prod prod-build pin-images log seed seed-clean test test-api \
	fmt fmt-fix clippy frontend-install frontend-lint frontend-lint-fix \
	frontend-test frontend-build frontend-ci

help:
	@echo "CoinBot.v3 Makefile"
	@echo ""
	@echo "  make dev       Start the dev stack (DB + Redis + all services + frontend)"
	@echo "  make prod      Pull pre-built images and (re)start the prod stack"
	@echo "  make pin-images  Resolve image digests and rewrite docker-compose.prod.yml"
	@echo "  make log       Follow prod container logs (SERVICE=<name> for one service)"
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
	 [ -f .env.ci ] || { echo ".env.ci missing"; exit 1; }; \
	 pg_user=$$(sed -n 's/^POSTGRES_USER=//p' .env.ci | tail -1 | tr -d '"'); \
	 pg_password=$$(sed -n 's/^POSTGRES_PASSWORD=//p' .env.ci | tail -1 | tr -d '"'); \
	 pg_db=$$(sed -n 's/^POSTGRES_DB=//p' .env.ci | tail -1 | tr -d '"'); \
	 redis_password=$$(sed -n 's/^REDIS_PASSWORD=//p' .env.ci | tail -1 | tr -d '"'); \
	 [ -n "$$pg_password" ] || { echo "POSTGRES_PASSWORD missing from .env.ci"; exit 1; }; \
	 [ -n "$$redis_password" ] || { echo "REDIS_PASSWORD missing from .env.ci"; exit 1; }; \
	 pulled=0; \
	 for attempt in 1 2 3; do \
	   if $(COMPOSE_TEST) pull postgres redis; then pulled=1; break; fi; \
	   echo "compose pull failed (attempt $$attempt), retrying"; sleep 5; \
	 done; \
	 [ "$$pulled" = 1 ] || exit 1; \
	 $(COMPOSE_TEST) down -v --remove-orphans >/dev/null 2>&1 || true; \
	 $(COMPOSE_TEST) up -d --wait postgres redis && \
		REDIS_URL="redis://:$$redis_password@127.0.0.1:56379" \
		DATABASE_URL="postgres://$${pg_user:-postgres}:$$pg_password@127.0.0.1:55432/$${pg_db:-coinbot}" \
		cargo test $(addprefix -p ,$(PACKAGES)); \
	 rst=$$?; \
	 $(COMPOSE_TEST) down -v --remove-orphans; \
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
	$(COMPOSE_DEV) up -d postgres redis && \
		$(COMPOSE_DEV) up api_gateway deposit-worker withdraw-worker trade-engine opencode signal-gateway frontend

prod-build:
	docker compose --env-file $(COMPOSE_ENV_FILE) --profile prod --profile tools build deposit-worker-prod api_gateway-prod frontend-prod trade-engine-prod withdraw-worker-prod signal-gateway-prod opencode

# Resolve current image digests and rewrite docker-compose.prod.yml.
pin-images:
	./scripts/pin-images.sh

# Pull pre-built images from ghcr (built by CI on main) and (re)start; never builds.
prod:
	$(COMPOSE_PROD) --profile prod --profile tools pull
	$(COMPOSE_PROD) --profile prod --profile tools up -d --no-build

log:
	$(COMPOSE_PROD) --profile prod --profile tools logs -f --tail=$(LOG_TAIL) $(SERVICE)

test-api:
	./scripts/test-api.sh

seed:
	@set -a; . ./.env; set +a; \
	PGPASSWORD="$$POSTGRES_PASSWORD" psql -h 127.0.0.1 -U "$$POSTGRES_USER" -d "$$POSTGRES_DB" -f scripts/seed-demo.sql

seed-clean:
	@set -a; . ./.env; set +a; \
	PGPASSWORD="$$POSTGRES_PASSWORD" psql -h 127.0.0.1 -U "$$POSTGRES_USER" -d "$$POSTGRES_DB" -c "TRUNCATE users, contracts, trade_orders, deposits RESTART IDENTITY CASCADE;"
