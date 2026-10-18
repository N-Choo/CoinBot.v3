# Benchmark: 4h vs 15min, four markets (2025)

- **Date:** 2026-10-18
- **Scope:** in-sample only (2025-01-01 to 2025-12-31). Nothing here touches the 2026 OOS window.
- **Status:** exploratory. Not a confirmatory verdict.
- **Parameters:** current `Config::default_for` defaults for both timeframes. The count-based
  parameters (lookback, expiry, pivots, grace, ATR period) were not re-tuned for 15min.

## 1. Method

Markets: BTC-USDT, ETH-USDT, SOL-USDT, XRP-USDT (the four markets the live analyzer runs).

Data: KuCoin candles fetched with `backtest fetch-data` for 2025-01-01 to 2025-12-31, at `4hour` and
`15min`. Each market has about 2,190 4h candles and 35,040 15min candles. Price scale per market
(decimal places): BTC 1, ETH 2, SOL 3, XRP 4.

Costs (`harness::sim` defaults): 5 bps fee per side, 5 bps entry slippage, 15 bps stop slippage,
2 bps per day borrow.

Commands:

```bash
backtest fetch-data --symbol <SYM>-USDT --interval <4hour|15min> \
    --start-s 1735689600 --end-s 1767225599 --out /tmp/bench4/<SYM>_<iv>.csv
backtest benchmark --symbol <SYM>-USDT --price-scale <SYM>-USDT=<dp> --intervals 4hour,15min \
    --data 4hour=/tmp/bench4/<SYM>_4hour.csv --data 15min=/tmp/bench4/<SYM>_15min.csv --no-fetch
backtest dump-signals --data <SYM>-USDT=/tmp/bench4/<SYM>_<iv>.csv \
    --price-scale <SYM>-USDT=<dp> --candle-interval <iv>
```

Stop and target distances are measured from `reference_entry_price`, `stop_loss` and `take_profit`
in each dumped signal. Net % per trade is approximated as mean net R multiplied by mean stop
distance.

## 2. Results

| market | tf | trades | per day | win % | net R / trade | cost R / trade | total net R | median stop % | median target % | net % / trade |
|---|---|---|---|---|---|---|---|---|---|---|
| BTC | 4h | 18 | 0.05 | 66.7 | +1.14 | 0.53 | +20.4 | 0.69 | 2.07 | +0.66 |
| BTC | 15m | 239 | 0.68 | 46.4 | -1.08 | 2.33 | -258.2 | 0.12 | 0.37 | -0.16 |
| ETH | 4h | 10 | 0.03 | 70.0 | +1.47 | 0.33 | +14.7 | 0.88 | 2.65 | +1.29 |
| ETH | 15m | 250 | 0.72 | 50.4 | -0.14 | 1.60 | -34.9 | 0.18 | 0.54 | -0.03 |
| SOL | 4h | 12 | 0.03 | 58.3 | +0.90 | 0.43 | +10.8 | 1.31 | 3.92 | +0.93 |
| SOL | 15m | 256 | 0.72 | 58.6 | +0.44 | 1.11 | +113.5 | 0.28 | 0.82 | +0.14 |
| XRP | 4h | 28 | 0.08 | 50.0 | +0.71 | 0.86 | +19.9 | 0.40 | 1.19 | +0.40 |
| XRP | 15m | 242 | 0.66 | 54.5 | +0.25 | 1.08 | +60.3 | 0.25 | 0.75 | +0.08 |

Note: XRP 15min shows 242 trades. An earlier run on the on-disk file showed 240, because that file
was re-fetched for this report.

## 3. Findings

1. **4h is positive on all four markets.** Net R per trade runs from +0.71 to +1.47. Stops are
   0.4 to 1.3% of price and targets 1.2 to 3.9%, so the TP/SL range is wide and profit per trade is
   +0.4% to +1.3%.
2. **15min is cost-bound.** Median stops are only 0.12 to 0.28% of price. The 15 bps stop slippage
   takes most of the risk, and costs run 1.1 to 2.3R per trade. On BTC the median stop (0.12%) is
   smaller than the round-trip cost of being stopped out, which is why BTC 15min loses −1.08R per
   trade.
3. **15min is positive only on SOL and XRP**, and only in total R. Volume drives the totals
   (+113.5R and +60.3R). Per trade the edge is thin, at +0.08% to +0.14%.
4. **The 4h edge is consistent but thinly sampled.** 10 to 28 trades per market per year is not
   enough to call the edge statistically reliable, even though every market is positive.

## 4. Interpretation

- At the current costs, 4h has the better per-trade economics and is the defensible base timeframe.
- 15min would need a cost-aware re-tune before it could be considered. Options are wider targets
  relative to the cost floor (`target_r` or `min_gross_rr`), or filtering out signals with stops
  below a cost threshold.
- Any tuned 15min result should be checked on a holdout period, not only on this 2025 window, to
  avoid overfitting.

## 5. Limitations

- One year of data (2025). Results may not hold in other regimes.
- The 15min count parameters are still the 4h-era values, so 15min is not a tuned configuration.
- The cost model is fixed. Real fills, maker or taker mix and actual slippage were not measured.
- Profit per trade in % is an approximation (mean net R times mean stop distance), not a per-trade
  realised percentage.
- Benchmark numbers are in-sample and exploratory. They are not a confirmatory verdict.
