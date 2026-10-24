<div align="center">
  <img src="react/public/coinbot-lockup.svg" alt="CoinBot" width="300" />
</div>

<p align="center"><b>Trade on autopilot — your wallet, your funds, your rules.</b></p>

<div align="center">
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" />
  <img src="https://img.shields.io/badge/Actix--Web-2f2f2f?style=for-the-badge&logo=rust&logoColor=white" />
  <img src="https://img.shields.io/badge/gRPC-Tonic-244c5a?style=for-the-badge&logo=grpc&logoColor=white" />
  <img src="https://img.shields.io/badge/SQLx-336791?style=for-the-badge&logo=postgresql&logoColor=white" />
  <img src="https://img.shields.io/badge/PostgreSQL-4169E1?style=for-the-badge&logo=postgresql&logoColor=white" />
  <img src="https://img.shields.io/badge/Redis-DC382D?style=for-the-badge&logo=redis&logoColor=white" />
  <img src="https://img.shields.io/badge/Python-3776AB?style=for-the-badge&logo=python&logoColor=white" />
  <img src="https://img.shields.io/badge/React-61DAFB?style=for-the-badge&logo=react&logoColor=black" />
  <img src="https://img.shields.io/badge/TypeScript-3178C6?style=for-the-badge&logo=typescript&logoColor=white" />
  <img src="https://img.shields.io/badge/Tailwind-06B6D4?style=for-the-badge&logo=tailwindcss&logoColor=white" />
  <img src="https://img.shields.io/badge/Docker-2496ED?style=for-the-badge&logo=docker&logoColor=white" />
  <img src="https://img.shields.io/badge/KuCoin-24b47e?style=for-the-badge&logo=kucoin&logoColor=white" />
  <img src="https://img.shields.io/badge/Ethereum-3C3C3D?style=for-the-badge&logo=ethereum&logoColor=white" />
</div>

<br />

Math does the watching. Our statistical models read the market around the clock, surface the trades worth taking, and fire them straight to KuCoin before the crowd even blinks.

## Processes

```mermaid
%%{init: {"theme":"base","themeVariables":{"clusterBkg":"transparent","clusterBorder":"#888","lineColor":"#888","primaryTextColor":"#fff","tertiaryTextColor":"#fff","edgeLabelBackground":"transparent"}}}%%
flowchart TB
    classDef plain fill:transparent,stroke:#888,color:#fff;

    UI["React SPA"]

    subgraph rust["Rust services"]
        GW["api_gateway"]
        DW["deposit-worker"]
        TE["trade-engine<br/>(gRPC server: EngineService)"]
        SA["analyzer<br/>(RSI momentum)<br/>[ Unvalidated ]"]
    end

    DB[("PostgreSQL")]
    RD[("Redis")]
    ETH["Ethereum RPC"]
    KC["KuCoin"]

    UI -->|"HTTP /api"| GW
    GW -->|"SQL"| DB
    GW -->|"gRPC"| DW
    DW -->|"verify tx"| ETH
    DW -->|"sweep"| KC
    DW -->|"upsert user + credit balance"| DB

    SA -->|"gRPC GetActiveSymbols (pull)"| TE
    SA -->|"gRPC Publish(Signal) (push)"| TE
    SA -->|"candles + tickers (public REST)"| KC

    GW -->|"sessions + nonces"| RD

    TE -->|"load + verify contracts"| DB
    TE -->|"execute + reconcile orders"| KC

    class UI,GW,DW,TE,SA,DB,RD,ETH,KC plain;
```

| Process          | Role                                                                                                                                                                                                                                                                                                                        | Docs                                       |
| ---------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------ |
| `api_gateway`    | HTTP API — auth, contract signing, deposit intake                                                                                                                                                                                                                                                                           | [README](process/api_gateway/README.md)    |
| `deposit-worker` | gRPC ticket + background deposit sweeper                                                                                                                                                                                                                                                                                    | [README](process/deposit-worker/README.md) |
| `trade-engine`   | Turns analyzer signals into KuCoin **cross-margin** orders, reconciles them against the exchange, and exits positions on an opposite target-side signal. Also serves the gRPC `EngineService` that `analyzer` calls. Sole writer of order rows and contract funds (via its `ledger`).       | [README](process/trade-engine/README.md)   |
| `analyzer`       | **[ Unvalidated ]** RSI momentum strategy (Rust): long above RSI 60, short/exit below RSI 50 on 4hour candles by default. Has a deterministic core, an offline backtest harness, and a live path that pushes target-side signals to `trade-engine` over gRPC.                                      | [README](process/analyzer/README.md)       |
| `frontend`       | React SPA                                                                                                                                                                                                                                                                                                                   | [README](react/README.md)                  |

Shared infrastructure: **PostgreSQL** (`users`, `deposits`, `contracts`, `trade_orders`). **Redis** holds gateway sessions (DB 0) and login nonces (DB 1).

### Signal path

`trade-engine` receives signals from `analyzer` over gRPC (`process/proto/analyzer.proto`). The analyzer pulls `GetActiveSymbols` every 30 s and pushes each signal with `Publish`, which the engine answers with an `Ack`. The link is configured with `GRPC_ADDR` on the engine (the bind address) and `GRPC_TRADE_ENGINE_ENDPOINT` on the analyzer (the engine URL).

Each signal gets exactly one publish attempt; nothing is resent after a restart. The engine deduplicates on `signal_id`. See the [analyzer README](process/analyzer/README.md) for the full live design.

## Repository layout

- `process/`: runtime services (`api_gateway`, `deposit-worker`, `trade-engine`, `analyzer`) plus `migrations/` (SQL, applied in order) and `proto/` (gRPC contracts, including `analyzer.proto` for the engine/analyzer link).
- `lib/share/`: shared Rust crate (config, DB models, Redis, RPC helpers).
- `react/`: frontend SPA.
- `docs/`, `scripts/`: project docs, demo seed data, and API smoke tests.

## Quick start

```bash
make dev     # Postgres + Redis + API + worker + trade-engine + analyzer + frontend
make trade   # trade-engine + analyzer live (starts Postgres + Redis)
make test    # Rust tests
make ci      # full pipeline: fmt-fix, clippy, test, analyzer tests, frontend lint-fix, frontend tests, build
```
