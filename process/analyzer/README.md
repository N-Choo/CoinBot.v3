# Analyzer

RSI divergence signal engine + backtesting bench for CoinBot.v3.

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
BENCHMARK_INIT_FUND=1000
BENCHMARK_FEE_RATE=0.001
BENCHMARK_TICKERS=BTC-USDT,ETH-USDT,SOL-USDT
```



## Trading pipeline

1. **Trade engine** loads active contracts from the DB
2. Publishes ticker list to Redis `tickers:analyze`
3. **Analyzer** fetches daily klines (KuCoin's shared spot/margin candles — margin exposes no separate price feed), computes RSI divergences
4. Applies SMA trend filter (buy only above, sell only below)
5. Sizes SL/TP via ATR with volatility scaling and max-loss cap
6. Publishes the single most recent signal to `signals:result`

## Signals

| Type         | Condition                         | Direction |
| ------------ | --------------------------------- | --------- |
| Regular Bull | Price lower low, RSI higher low   | BUY       |
| Regular Bear | Price higher high, RSI lower high | SELL      |
| Hidden Bull  | Price higher low, RSI lower low   | BUY       |
| Hidden Bear  | Price lower high, RSI higher high | SELL      |

Only the most recent divergence per ticker is emitted. Confidence: 0.6 regular, 0.8 near overbought/oversold, 0.5 hidden.


## Architecture

```
analyzer/
├── main.py              # live mode — subscribes to Redis, emits signals
├── config.py             # all settings via .env
├── models/types.py       # Signal, OHLCV dataclasses
├── indicators/
│   ├── rsi.py            # calculate_rsi, detect_rsi_divergences
│   ├── sma.py            # calculate_sma  (trend filter)
│   └── atr.py            # calculate_atr  (SL/TP sizing)
├── signals/generator.py  # generate_signals — picks the single most recent divergence
├── fetchers/kucoin.py    # KuCoin REST klines
├── publisher/redis_pub.py
└── benchmark/
    ├── engine.py         # pure trade-engine port + SimEngine
    └── main.py           # replay fetch → signals → engine → ledger
```

## How it works

**Live mode** — subscribes to `tickers:analyze` on Redis. For each ticker batch, fetches daily klines from KuCoin, runs RSI divergence detection, applies SMA trend filter + ATR-based SL/TP, and publishes the single most recent signal to `signals:result`.

**Benchmark** — replays historical candles one at a time through a simulation of the **trade engine**. It fetches the history once, then for each candle evaluates `generate_signals` on the data available up to that point (causal) and feeds the signal to a simulated contract: a market entry of `init_fund / 2` whenever funds allow, and an exit when the candle's range reaches the signal's TP or SL (TP checked first). There is no time cap — a position is held until TP/SL, exactly as the live engine does. Fees are netted into realized PnL and losses are capped at the committed fund. The ledger reports closed round trips plus open positions and an account summary (win rate, realized/unrealized PnL, equity).

## Quick start

```bash
# Run backtest
make benchmark

# Run unit tests
make benchmark-test   # benchmark only
make analyzer-test    # everything

# Live mode (needs Redis)
make trade
```

## Bench output

Each ticker prints a round-trip ledger — one row per closed trade — plus a
footer for positions still open and an account summary:

```
  BTC-USDT
  ──────────────────────────────────────────────────────────────────
  entry      dir   entry$     exit       exit$      pnl%     pnl$  why
  2026-05-12 BUY   91,200     2026-05-19 96,000     +5.0%   +50.00  TP
  2026-05-28 BUY   88,000     2026-06-02 85,000    -10.0%   -50.00  SL
  ──────────────────────────────────────────────────────────────────
  open: 1  |  trades 2  wins 1/2 (50%)  realized +0.00  equity 1000.00 (init 1000)
```

`why` is `TP` or `SL`. Positions still open at the end of data are listed in the
table with `open` in the exit columns (they are never force-closed) and counted
in the `open:` footer; the summary reports win rate, realized PnL, and equity.

## Core rules

| Signal type  | Condition                                        |
| ------------ | ------------------------------------------------ |
| Regular Bull | Price lower low, RSI higher low                  |
| Regular Bear | Price higher high, RSI lower high                |
| Hidden Bull  | Price higher low, RSI lower low (continuation)   |
| Hidden Bear  | Price lower high, RSI higher high (continuation) |

Regular divergences get 0.6–0.8 confidence (boosted near oversold/overbought). Hidden get 0.5.

**SMA filter** — when enabled, buy signals only if price > SMA (uptrend), sell only if price < SMA (downtrend).

**ATR scaling** — SL and TP are sized as multiples of ATR. If ATR% exceeds `ATR_PCT_HIGH`, the multipliers are scaled up by `ATR_HIGH_ADJUST` to handle volatile coins. SL is also clamped at `ATR_SL_MAX_PCT` as an absolute floor.
