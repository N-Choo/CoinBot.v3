<div align="center">
  <img src="https://capsule-render.vercel.app/api?type=waving&color=gradient&customColorList=2,3&height=180&section=header&text=CoinBot%20x%20KuCoin&fontSize=56&animation=fadeIn&fontAlignY=38" alt="CoinBot x KuCoin" />
</div>

<p align="center"><b>Automated crypto trading — signed contracts, on-chain deposits, signal-driven execution.</b></p>

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

**CoinBot.v3** is an event-driven crypto trading platform that turns user-signed contracts into automated trades on KuCoin futures. Users authenticate with an Ethereum wallet, deposit USDT on-chain, and sign a contract allocating funds to a strategy; a Rust **trade engine** consumes RSI-divergence signals from a Python **analyzer** and executes them, while an **API gateway** and **deposit worker** handle wallet auth, contract signing, and on-chain deposit verification.

## Processes

```mermaid
%%{init: {"theme":"base","themeVariables":{"clusterBkg":"transparent","clusterBorder":"#888","lineColor":"#888","primaryTextColor":"#fff","tertiaryTextColor":"#fff","edgeLabelBackground":"transparent"}}}%%
flowchart TB
    classDef plain fill:transparent,stroke:#888,color:#fff;

    UI["React SPA"]

    subgraph rust["Rust services"]
        GW["api_gateway"]
        DW["deposit-worker"]
        TE["trade-engine"]
    end

    AN["analyzer (Python)"]
    DB[("PostgreSQL")]
    RD[("Redis")]
    ETH["Ethereum RPC"]
    KC["KuCoin"]

    UI -->|"HTTP /api"| GW
    GW -->|"SQL"| DB
    GW -->|"gRPC"| DW
    DW -->|"verify tx"| ETH
    DW -->|"sweep"| KC
    DW -->|"credit balance"| DB

    TE -->|"publish tickers:analyze"| RD
    RD -->|"subscribe"| AN
    AN -->|"publish signals:result"| RD
    RD -->|"signals"| TE
    TE -->|"load + verify contracts"| DB
    TE -.->|"place orders"| KC

    class UI,GW,DW,TE,AN,DB,RD,ETH,KC plain;
```

| Process          | Path                     | Role                                                    | Talks to                                      |
| ---------------- | ------------------------ | ------------------------------------------------------- | --------------------------------------------- |
| `api_gateway`    | `process/api_gateway`    | HTTP API — auth, contract signing, deposit intake       | React, PostgreSQL, deposit-worker             |
| `deposit-worker` | `process/deposit-worker` | gRPC ticket + background deposit sweeper                | api_gateway, PostgreSQL, Ethereum RPC, KuCoin |
| `trade-engine`   | `process/trade-engine`   | Publishes tickers, consumes signals, verifies contracts | Redis, PostgreSQL, KuCoin                     |
| `analyzer`       | `process/analyzer`       | signal engine (Python)                                  | Redis, KuCoin                                 |

Shared infrastructure: **PostgreSQL** (`users`, `deposits`, `contracts`, `trade_orders`) and **Redis** (pub/sub channels `tickers:analyze`, `signals:result`).

## Flows

### Authentication

Wallet challenge/response login — proves wallet ownership and issues a session cookie.

```mermaid
%%{init: {"theme":"base","themeVariables":{"actorBkg":"transparent","actorBorder":"#888","actorTextColor":"#fff","signalColor":"#888","signalTextColor":"#fff","labelBoxBkgColor":"transparent","labelBoxBorderColor":"#888","labelTextColor":"#fff","noteBkgColor":"transparent","noteBorderColor":"#888","noteTextColor":"#fff"}}}%%
sequenceDiagram
    participant U as Wallet / SPA
    participant G as api_gateway
    participant R as Redis (nonce)

    U->>G: GET /api/user/auth?wallet_address=0x..
    G->>R: store nonce
    G-->>U: nonce
    U->>U: sign(nonce)
    U->>G: POST /api/user/auth (signature, msg)
    G->>G: recover wallet, check nonce
    G->>R: invalidate nonce, store session
    G-->>U: Set-Cookie session_token
```

### Contract signing

Authorize a strategy — verify the signed settings and lock the allocated funds on a new contract.

```mermaid
%%{init: {"theme":"base","themeVariables":{"actorBkg":"transparent","actorBorder":"#888","actorTextColor":"#fff","signalColor":"#888","signalTextColor":"#fff","labelBoxBkgColor":"transparent","labelBoxBorderColor":"#888","labelTextColor":"#fff","noteBkgColor":"transparent","noteBorderColor":"#888","noteTextColor":"#fff"}}}%%
sequenceDiagram
    participant U as Wallet / SPA
    participant G as api_gateway
    participant R as Redis (nonce)
    participant DB as PostgreSQL

    U->>G: GET /api/contracts/nonce
    G->>R: store nonce
    G-->>U: nonce
    U->>U: sign(nonce, settings)
    U->>G: POST /api/contracts/sign (nonce, message, signature)
    G->>G: verify signature + settings
    G->>DB: lock init_fund (balance -> locked_balance)
    G->>DB: insert contract (available = init_fund, used = 0)
    G-->>U: 200 Contract signed
```

### Deposit

On-chain USDT deposit — verify the transaction, then credit the user's balance.

```mermaid
%%{init: {"theme":"base","themeVariables":{"actorBkg":"transparent","actorBorder":"#888","actorTextColor":"#fff","signalColor":"#888","signalTextColor":"#fff","labelBoxBkgColor":"transparent","labelBoxBorderColor":"#888","labelTextColor":"#fff","noteBkgColor":"transparent","noteBorderColor":"#888","noteTextColor":"#fff"}}}%%
sequenceDiagram
    participant U as Wallet / SPA
    participant G as api_gateway
    participant W as deposit-worker
    participant E as Ethereum RPC
    participant DB as PostgreSQL

    U->>U: send USDT on-chain to platform wallet
    U->>G: POST /api/transactions/deposit (tx_hash)
    G->>E: get transaction + receipt
    E-->>G: tx (from, to, input)
    G->>G: verify sender, USDT contract, recipient
    G->>W: gRPC create_ticket0(tx_hash, uid)
    W->>DB: insert deposit (pending)
    W-->>G: ticket_id
    G-->>U: 200 ticket_id
    Note over W,DB: sweeper confirms tx and credits balance
```

### Trading

Signal to execution — match analyzer signals to active contracts, verify each signature, then execute.

```mermaid
%%{init: {"theme":"base","themeVariables":{"actorBkg":"transparent","actorBorder":"#888","actorTextColor":"#fff","signalColor":"#888","signalTextColor":"#fff","labelBoxBkgColor":"transparent","labelBoxBorderColor":"#888","labelTextColor":"#fff","noteBkgColor":"transparent","noteBorderColor":"#888","noteTextColor":"#fff"}}}%%
sequenceDiagram
    participant TE as trade-engine
    participant R as Redis
    participant AN as analyzer
    participant DB as PostgreSQL
    participant KC as KuCoin

    TE->>R: PUBLISH tickers:analyze
    R-->>AN: tickers
    AN->>KC: fetch klines
    KC-->>AN: OHLCV
    AN->>AN: RSI + SMA + ATR
    AN->>R: PUBLISH signals:result
    R-->>TE: signals
    TE->>DB: load active contracts
    TE->>TE: re-verify signatures (fail-closed)
    TE-->>KC: place orders (WIP)
```
