PACKAGES = api-gateway share deposit trade-engine

.PHONY: help clean ci clippy test fmt fmt-fix frontend-install frontend-lint frontend-lint-fix frontend-test frontend-build dev prod proto prod-build logs logs-backend test-api trade analyzer-test

help:
	@echo "Usage: make <target>"
	@echo ""
	@echo "Docker"
	@echo "  clean        Remove all containers, images, and volumes"
	@echo ""
	@echo "Development"
	@echo "  dev          Start deposit-worker + backend-dev + frontend"
	@echo "  trade        Run trade-engine + Python analyzer"
	@echo "  test-api     Run curl tests against the API"
	@echo "  logs         Follow logs from all services"
	@echo "  logs-backend Follow backend logs"
	@echo ""
	@echo "Production"
	@echo "  prod         Build + start all production services (detached)"
	@echo "  prod-build   Build production images without running"
	@echo ""
	@echo "Rust"
	@echo "  fmt          Check formatting"
	@echo "  fmt-fix      Fix formatting"
	@echo "  clippy       Lint (deny warnings)"
	@echo "  test         Run Rust unit tests"
	@echo "  analyzer-test Run Python analyzer unit tests"
	@echo "  benchmark-test Run benchmark unit tests"
	@echo "  benchmark      Run RSI signal backtesting (requires KuCoin access)"
	@echo "  proto        Compile wallet.proto (verify proto only)"
	@echo ""
	@echo "Frontend"
	@echo "  frontend-lint      Lint frontend"
	@echo "  frontend-lint-fix  Auto-fix frontend lint"
	@echo "  frontend-test      Run frontend tests"
	@echo "  frontend-build     Build frontend for production"
	@echo ""
	@echo "CI"
	@echo "  ci           Run full CI pipeline (fmt + clippy + test + lint + build)"

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
		cd analyzer && python3 -m pytest . -v; \
		pst=$$?; \
		docker compose stop redis; \
		exit $$(( rst + pst ))

analyzer-test:
	@cd analyzer && python3 -m pytest . -v

benchmark-test:
	@cd analyzer && python3 -m pytest benchmark/tests/ -v

benchmark:
	@python3 -m analyzer.benchmark.main

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
		(REDIS_HOST=127.0.0.1 REDIS_URL=redis://127.0.0.1:6379 python3 -m analyzer.main & \
		PID=$$!; sleep 2 && \
		REDIS_URL=redis://127.0.0.1:6379 cargo run -p trade-engine; st=$$?; \
		sleep 3; kill $$PID 2>/dev/null; wait $$PID 2>/dev/null; \
		exit $$st); \
		docker compose stop redis

proto:
	cargo build -p common --timings
