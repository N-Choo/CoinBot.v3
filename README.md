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
├── server/                          # Rust (Actix-Web)
│   ├── Cargo.toml
│   ├── Dockerfile
│   └── src/
│       ├── main.rs                  # HttpServer entry
│       ├── config.rs                # AppConfig, CORS
│       ├── state.rs                 # PgPool, cache, KuCoin client
│       ├── routes.rs
│       ├── handlers/user/auth.rs    # EIP-191 auth endpoints
│       └── models/                  # DTOs, AppError
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
└── system_architecture/
    └── trader_agent_blueprint.md
```

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

## API Overview
| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/api/user/auth` | Request EIP-191 signing challenge (nonce) |
| `POST` | `/api/user/auth` | Submit signed message → session cookie |
| `POST` | `/api/user/logout` | Invalidate session |
| `POST` | `/api/user/verify` | Check session validity |
---
