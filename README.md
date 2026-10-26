<div align="center">
  <img src="react/public/coinbot-lockup.svg" alt="CoinBot" width="300" />
</div>

<p align="center"><b>Automated crypto trading: your wallet, your funds, your rules.</b></p>

<div align="center">
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" />
  <img src="https://img.shields.io/badge/gRPC-Tonic-244c5a?style=for-the-badge&logo=grpc&logoColor=white" />
  <img src="https://img.shields.io/badge/PostgreSQL-4169E1?style=for-the-badge&logo=postgresql&logoColor=white" />
  <img src="https://img.shields.io/badge/Redis-DC382D?style=for-the-badge&logo=redis&logoColor=white" />
  <img src="https://img.shields.io/badge/React-61DAFB?style=for-the-badge&logo=react&logoColor=black" />
  <img src="https://img.shields.io/badge/TypeScript-3178C6?style=for-the-badge&logo=typescript&logoColor=white" />
  <img src="https://img.shields.io/badge/Docker-2496ED?style=for-the-badge&logo=docker&logoColor=white" />
  <img src="https://img.shields.io/badge/KuCoin-24b47e?style=for-the-badge&logo=kucoin&logoColor=white" />
</div>

<br />

CoinBot turns trading signals into KuCoin orders automatically. **Status: experimental. No strategy has been validated as profitable.**

## Architecture

```mermaid
%%{init: {"theme":"base","themeVariables":{"clusterBkg":"transparent","clusterBorder":"#888","lineColor":"#888","primaryTextColor":"#fff","tertiaryTextColor":"#fff","edgeLabelBackground":"transparent"}}}%%
flowchart TB
    classDef plain fill:transparent,stroke:#888,color:#fff;

    UI["React SPA"]

    subgraph rust["Rust services"]
        GW["api_gateway"]
        DW["deposit-worker"]
        TE["trade-engine"]
        SA["analyzer"]
    end

    DB[("PostgreSQL")]
    RD[("Redis")]
    ETH["Ethereum RPC"]
    KC["KuCoin"]

    UI -->|"HTTP"| GW
    GW --> DB
    GW --> RD
    GW -->|"gRPC"| DW
    DW -->|"verify tx"| ETH
    DW -->|"sweep"| KC
    DW --> DB
    SA -->|"gRPC signals"| TE
    SA -->|"candles"| KC
    TE --> DB
    TE -->|"orders"| KC

    class UI,GW,DW,TE,SA,DB,RD,ETH,KC plain;
```

| Service | Role | Docs |
|---|---|---|
| `api_gateway` | HTTP API: auth, contracts, deposits | [README](process/api_gateway/README.md) |
| `deposit-worker` | Verifies and sweeps deposits | [README](process/deposit-worker/README.md) |
| `trade-engine` | Turns signals into KuCoin cross-margin orders and manages TP/SL exits | [README](process/trade-engine/README.md) |
| `analyzer` | RSI momentum signal generator (unvalidated) | [README](process/analyzer/README.md) |
| `frontend` | React SPA | [README](react/README.md) |

**Signal path:** `analyzer` polls active symbols from `trade-engine` and pushes each signal once over gRPC. The engine deduplicates by `signal_id`. Set `GRPC_ADDR` on the engine and `GRPC_TRADE_ENGINE_ENDPOINT` on the analyzer.

## Layout

- `process/`: services, SQL `migrations/`, and gRPC `proto/`
- `lib/share/`: shared Rust crate
- `react/`: frontend
- `docs/`, `scripts/`: docs, seed data, smoke tests

## Quick start

```bash
make dev     # run everything
make trade   # trade-engine + analyzer only
make test    # Rust tests
make ci      # full pipeline
```
