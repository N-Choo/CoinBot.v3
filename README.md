<div align="center">
  <img src="https://capsule-render.vercel.app/api?type=waving&color=gradient&customColorList=2,3&height=200&section=header&text=CoinBot%20V3&fontSize=64&animation=fadeIn&fontAlignY=36&desc=CORE&descSize=20&descAlignY=54" alt="CoinBot V3 Banner" />
</div>
<p align="center">
  <b>A trading engine that learns. Powered by ML and local LLMs.</b>
</p>
<div align="center">
  <img src="https://img.shields.io/badge/Rust-Actix--Web-orange?style=for-the-badge&logo=rust&logoColor=white" />
  <img src="https://img.shields.io/badge/React-19-blue?style=for-the-badge&logo=react&logoColor=white" />
  <img src="https://img.shields.io/badge/TypeScript-3178C6?style=for-the-badge&logo=typescript&logoColor=white" />
  <img src="https://img.shields.io/badge/PostgreSQL-SQLx-336791?style=for-the-badge&logo=postgresql&logoColor=white" />
  <img src="https://img.shields.io/badge/KuCoin-API-24b47e?style=for-the-badge&logo=kucoin&logoColor=white" />
  <img src="https://img.shields.io/badge/Docker-Compose-2496ED?style=for-the-badge&logo=docker&logoColor=white" />
</div>

<br />

---

## Architecture

```text
.
├── Cargo.toml                       # Workspace root w/ shared deps
├── Makefile                         # Local CI runner
├── api_gateway/                     # Rust (Actix-Web) — auth, routing
│   ├── Cargo.toml
│   ├── Dockerfile
│   └── src/
│       ├── main.rs                  # HttpServer entry
│       ├── config.rs                # AppConfig, CORS
│       ├── constants.rs             # Token addresses, platform wallet
│       ├── state.rs                 # PgPool, cache, KuCoin client
│       ├── routes.rs
│       ├── handlers/                # EIP-191 auth, deposit HTTP handlers
│       └── models/                  # DTOs, AppError
│
├── common/                          # Shared library (proto stubs, db, rpc)
│   ├── Cargo.toml
│   ├── build.rs                     # Compiles proto/wallet.proto via tonic
│   └── src/
│       ├── lib.rs                   # Re-exports gRPC types, ServiceConfig
│       ├── db/                      # DB models, queries, Deposit, DepositFilter
│       └── rpc.rs                   # Ethereum RPC client (get_transaction)
│
├── deposit-worker/                  # Rust (tonic) — gRPC deposit server
│   ├── Cargo.toml
│   ├── Dockerfile
│   └── src/main.rs                  # DepositService gRPC skeleton
│
├── proto/
│   ├── wallet.proto                 # gRPC contract (DepositService)
│   └── README.md                    # gRPC patterns & conventions
│
├── react/                           # Vite + React 19 + TS
│   ├── Dockerfile
│   ├── nginx.conf
│   ├── package.json
│   └── src/
│       ├── App.tsx                  # Routes: /, /trading, /dashboard
│       ├── components/              # topbar, auth_guard, dashboard/, landing/, trading/
│       ├── hooks/                   # useAuth, useTheme, useScrollAnimation
│       ├── pages/                   # homePage, dashboard, tradingPage
│       ├── services/                # connectWallet, kucoin
│       └── styles/
│
├── docker-compose.yml
└── system_architecture/
    └── trader_agent_blueprint.md
```

## Security Architecture

### Separation of Duties (SoD)

Each business function runs in its own isolated process with its own database.
No single service performs both deposit and withdrawal logic.

| Service           | Responsibility                  | Cannot access   |
| ----------------- | ------------------------------- | --------------- |
| `api_gateway`     | Auth, routing                   | Modify balances |
| `deposit-worker`  | Record deposits via gRPC        | Withdrawal DB   |

### Ethical Walls

Communication between services is restricted to defined interfaces:

```
Client (caller)        Interface                Server (owner)
─────────────────────────────────────────────────────────────
api_gateway ──gRPC──▶ DepositService           deposit-worker
```

**Three enforcement layers:**

1. **Crate boundaries** — each crate only imports `common` (shared types, proto stubs). No crate imports another service's internals.
2. **Docker isolation** — each container has credentials for its own database only. No cross-service DB access.
3. **Proto contracts** — services only expose what `proto/wallet.proto` defines. The Rust compiler rejects mismatches.

See `proto/README.md` for the full gRPC + Redis communication guide.

## Features

### Backend

- [x] EIP-191 wallet-based authentication (challenge → sign → session)
- [x] Session cookie management (Moka cache, TTL-based)
- [x] CI pipeline (fmt, clippy, cargo test)
- [ ] ML signal pipeline (see `system_architecture/`)
- [ ] Local LLM trade agent
- [ ] Strategy backtesting engine

### Frontend

- [x] Wallet connect / disconnect flow (ethers.js + EIP-191)
- [x] TradingView chart integration
- [x] Portfolio dashboard (balance stats, P&L chart, contract cards)
- [x] Landing page (hero, pipeline animation, feature cards)
- [x] Dark/light theme toggle
- [x] Auth-guarded routing (redirect if unauthenticated)

---

## Local Development

Run all CI checks (fmt → clippy → test → frontend lint → test → build):

```sh
make ci
```

Or individually:

```sh
make fmt-fix    # auto-format Rust
make clippy     # Rust lints
make test       # Rust tests
make frontend-lint-fix  # auto-fix frontend lint
```

### CI Pipeline

The GitHub Actions workflow (`.github/workflows/ci.yml`) runs on push to `main` and
pull requests. It mirrors `make ci` — two jobs:

| Job | Checks |
|---|---|
| `backend-test` | Format (`make fmt`), Clippy (`make clippy`), Tests (`make test`) |
| `frontend-test` | Lint, Test, Build |

**Which packages are checked** is defined by `PACKAGES` in the Makefile:

```makefile
PACKAGES = api-gateway
```

Add new services to this list to include them in CI. Add new checks by extending
the Makefile — CI calls `make` targets, not raw cargo commands.

To run the full CI pipeline locally:

```sh
make ci
```

Or a single job:

```sh
make fmt clippy test   # equivalent to backend-test
```

## API Overview

| Method | Endpoint           | Description                               |
| ------ | ------------------ | ----------------------------------------- |
| `GET`  | `/api/user/auth`   | Request EIP-191 signing challenge (nonce) |
| `POST` | `/api/user/auth`   | Submit signed message → session cookie    |
| `POST` | `/api/user/logout` | Invalidate session                        |
| `POST` | `/api/user/verify` | Check session validity                    |

---
