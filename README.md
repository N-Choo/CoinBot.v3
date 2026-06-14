<div align="center">
  <img src="https://capsule-render.vercel.app/api?type=waving&color=gradient&customColorList=2,3&height=200&section=header&text=CoinBot%20V3&fontSize=64&animation=fadeIn&fontAlignY=36&descSize=20&descAlignY=54" alt="CoinBot V3 Banner" />
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

```
User → React SPA → API Gateway (actix-web :8080)
                        ├── PostgreSQL (users, deposits, contracts)
                        ├── gRPC → Deposit Worker (:50051)
                        │               └── KuCoin sweep → balance credit
                        └── EIP-191 signature verification
                              └── Contract signing with nonce + message

Deposit Worker (background)
    └── Polls KuCoin every 60s → confirms → credits user

```

## Stack

- **Backend:** Rust (Actix-Web, Tonic, SQLx)
- **Frontend:** React 19 + TypeScript + Vite
- **Database:** PostgreSQL
- **Infrastructure:** Docker Compose, gRPC, GitHub Actions
- **Live:** [CoinBot-V3](https://raspberrypi.ibex-mooneye.ts.net:8443/)

## Quick start

```bash
make dev              # starts all services (backend + worker + frontend)
make ci               # full pipeline (fmt → clippy → test → build)
```

## API

| Method | Endpoint                  | Description                               |
| ------ | ------------------------- | ----------------------------------------- |
| GET    | /api/user/auth            | Request signing challenge (nonce)         |
| POST   | /api/user/auth            | Submit signed message → session cookie    |
| POST   | /api/user/verify          | Check session validity                    |
| POST   | /api/user/logout          | Clear session                             |
| GET    | /api/config               | Platform wallet address, supported tokens |
| GET    | /api/transactions         | List user deposits                        |
| POST   | /api/transactions/deposit | Submit deposit tx hash (USDT only)        |
| GET    | /api/contracts/nonce      | Get nonce for contract signing            |
| POST   | /api/contracts/sign       | Sign contract with EIP-191 message        |

## Project structure

```
process/
  api_gateway/          HTTP API — auth, routing, deposit validation, contract signing
  deposit-worker/       gRPC server + background KuCoin sweeper
  migrations/           SQL migrations
  proto/                gRPC contract definitions
lib/
  common/               Proto-compiled gRPC stubs
  share/                Shared DB models, RPC client, ERC20 decoder, contract model
scripts/
  test-api.sh           Curl-based API smoke tests
  crontab.example       Example crontab for background tasks
react/                  SPA dashboard + trading interface
```

## Features

- [x] EIP-191 wallet-based authentication (session cookie via nonce challenge)
- [x] USDT-only deposit with on-chain verification (receipt, sender, ERC20 recipient)
- [x] Background KuCoin deposit sweeper via gRPC
- [x] Transactional user balance management
- [x] Contract signing with EIP-191 message verification
  - [x] Settings parsed from signed message (tamper-proof)
  - [x] Nonce replay protection (in-memory cache + DB unique index)
  - [x] BotSettings validation (amount > 0, SL/TP in [0,100], SL < TP, ticker ends with /USDT)
  - [x] Insufficient funds detection → deposit modal prompt
- [x] Deposit modal integrated in trading form and dashboard sidebar
- [x] ARM64 Docker images via CI
