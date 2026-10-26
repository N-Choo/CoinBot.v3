# analyzer

Rust RSI momentum analyzer. **Unvalidated: no tested configuration has shown a profitable edge.**

**Strategy:** Long when RSI(21) on 4h candles crosses above 60; exit/short when it crosses below 50. No stop-loss or take-profit. Signals strictly alternate long/short.

**Layers:** `src/strategy/` (pure core), `src/harness/` (replay and sweeps), `src/live/` (gRPC client to `trade-engine`).

## Commands

```bash
make analyzer-check    # fmt + clippy + tests
make analyzer-replay   # HTML replay report
make analyzer-sweep    # hyperparameter screening

cp process/analyzer/.env.example process/analyzer/.env   # configure
cargo run -p analyzer --features grpc --bin analyzer-serve   # run live
```

## Config (`.env`)

```dotenv
KUCOIN_TIMEFRAME=4hour
RSI_PERIOD=21
RSI_LONG_THRESHOLD=60
RSI_SHORT_THRESHOLD=50
ANALYZER_WINDOW_CANDLES=100
```

Other options: `LIVE_SYMBOLS` (fallback symbols), `ANALYZER_ONCE=1` (single scan, then exit), `ANALYZER_ACTIVATION_POLL_MS`, `KUCOIN_BASE_URL`.

## Replay

```bash
cargo run -p analyzer --features harness --bin analyzer-replay -- \
  --symbol BTC-USDT --timeframe 4hour --limit 500 --out out/replay.html
```

Use `--data SYMBOL=path.csv` for offline runs. Fees and slippage are excluded.

## Results

Experiments (see [`experiments/README.md`](experiments/README.md)) found **insufficient evidence** for any RSI configuration. Baseline holdout results were negative. The earlier RSI-divergence strategy failed confirmation and is closed.
