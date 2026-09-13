# CoinBot.v3

Automated KuCoin futures trading with RSI divergence signals, ATR-based risk management, and backtesting.

## Quick start

```bash
make dev          # start all services (API + worker + DB + Redis + frontend)
make trade        # run trade-engine + analyzer against KuCoin (needs DB)
make benchmark    # backtest all tickers (no DB needed, just .env)
make test         # Rust + Python tests
make ci           # full pipeline (fmt → clippy → test → build)
```

## Configuration

All settings in `.env` at project root:

```ini
# ── Required ──────────────────────────────────
DATABASE_URL=postgresql://user:pass@host:port/coinbot
REDIS_URL=redis://redis:6379

# ── KuCoin API Keys ───────────────────────────
TRADE_ENGINE_KC_KEY=your_key
TRADE_ENGINE_KC_SECRET=your_secret
TRADE_ENGINE_KC_PASSPHRASE=your_passphrase

# ── Analyzer ──────────────────────────────────
KUCOIN_TIMEFRAME=1day
SMA_ENABLED=true
SMA_PERIOD=200
RSI_PERIOD=14
RSI_DIVERGENCE_ORDER=8

# ── Risk Management ───────────────────────────
ATR_SL_MULTIPLIER=2.8
ATR_TP_MULTIPLIER=3.5
ATR_SL_MAX_PCT=0.10

# ── Benchmark ─────────────────────────────────
BENCHMARK_DAYS=360
BENCHMARK_TICKERS=BTC-USDT,ETH-USDT,SOL-USDT
ALLOW_PYRAMIDING=false
```

Full config reference: see `analyzer/README.md`.

## Architecture

```
Trade Engine (Rust) ──publish tickers──→ Redis
                                          │
Analyzer (Python) ◄──subscribe────────────┘
  │  reads KuCoin klines, computes RSI + SMA + ATR
  │  publishes single most-recent signal with SL/TP
  └──→ Redis ──consume signals──→ Trade Engine

API Gateway (Rust) ──→ PostgreSQL (users, deposits, contracts)
                    ──→ gRPC ──→ Deposit Worker (KuCoin sweep)
React SPA ◄────────────── HTTP API
```

## Project layout

```
analyzer/            Python signal engine + backtesting bench
  main.py              Live mode (Redis subscriber)
  benchmark/main.py    Day-by-day historical simulation
  indicators/          RSI, SMA, ATR
  signals/             Divergence → Signal generation
process/
  trade-engine/        Publishes contracts → Redis, consumes signals
  api_gateway/         HTTP API — auth, routing, deposit validation
  deposit-worker/      gRPC + background KuCoin sweeper
  migrations/          SQL migrations
lib/
  common/              Shared config, errors, proto stubs
  share/               DB models, Redis cache, contract model
react/                 SPA dashboard + trading interface
docs/                  Architecture docs, sequence diagrams
```

## Trading pipeline

1. **Trade engine** loads active KuCoin futures contracts from DB
2. Publishes ticker list to Redis `tickers:analyze`
3. **Analyzer** fetches daily klines, computes RSI divergences
4. Applies SMA trend filter (buy only above, sell only below)
5. Sizes SL/TP via ATR with volatility scaling and max-loss cap
6. Publishes the single most recent signal to `signals:result`

## Signals

| Type          | Condition                              | Direction |
|---------------|----------------------------------------|-----------|
| Regular Bull  | Price lower low, RSI higher low        | BUY       |
| Regular Bear  | Price higher high, RSI lower high       | SELL      |
| Hidden Bull   | Price higher low, RSI lower low         | BUY       |
| Hidden Bear   | Price lower high, RSI higher high       | SELL      |

Only the most recent divergence per ticker is emitted. Confidence: 0.6 regular, 0.8 near overbought/oversold, 0.5 hidden.

## Testing

```bash
make test             # Rust cargo test + Python pytest (51 tests)
make analyzer-test    # Python only
make benchmark-test   # benchmark tests only
```

See `docs/sequence-diagrams.md` for data flows. See `docs/README_PROJECT.md` for full stack details.
