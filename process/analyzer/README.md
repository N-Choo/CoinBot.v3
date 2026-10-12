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
BENCHMARK_TICKERS=BTC-USDT,ETH-USDT,SOL-USDT
ALLOW_PYRAMIDING=false
```



## Trading pipeline

1. **Trade engine** loads active contracts from the DB
2. Publishes ticker list to Redis `tickers:analyze`
3. **Analyzer** fetches daily klines, computes RSI divergences
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
    └── main.py           # backtesting — day-by-day simulation with SL/TP
```

## How it works

**Live mode** — subscribes to `tickers:analyze` on Redis. For each ticker batch, fetches daily klines from KuCoin, runs RSI divergence detection, applies SMA trend filter + ATR-based SL/TP, and publishes the single most recent signal to `signals:result`.

**Benchmark** — simulates the analyzer day-by-day through historical data. On each day, it runs the same `generate_signals` function using only data available up to that point. Each signal is scored against forward price with SL/TP exit rules. No 7-day cap — trades run until SL/TP is hit or end of data.

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

Each ticker prints a compact table — one row per signal:

```
  date       dir        entry       exit         SL         TP      pct  hit
  2026-05-12 BUY      $91,200 $96,000 TP    $87,600    $96,000    +5.3%  ✓
  2026-05-28 BUY      $88,000 $85,000 SL    $85,000    $92,000   -10.0%  ✗
  BUY  1/2 (50%) avg -2.3%  |  SELL 3/4 (75%) avg +5.1%
```

Exit column: `price TP` = take-profit hit, `price SL` = stop-loss hit, just price = held to end of data.

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
