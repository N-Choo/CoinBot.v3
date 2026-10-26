# Analyzer Experiments

> Designed by ChatGPT and built by DeepSeek V4 Flash. Claude handled most of the code review, in both the planning and implementation stages. DeepSeek runs in the **OpenCode** agent system, with specialist sub-agents such as brainstorm, code-review, tech-lead, and architecture. The experiments have also produced a good amount of data, which can be used to build a well-designed strategy. Not financial advice. No tested strategy has shown a robust out-of-sample edge after realistic costs.

## Assumptions

Closed-candle signals, next-open execution, one position per symbol, chronological splits. Base costs: 15 bps/side (10 fee + 5 slippage). Stress costs: 30 bps/side.

## Results

| Family | Experiments | Verdict |
|---|---|---|
| **RSI momentum** | 4h baseline, per-symbol tuning, walk-forward, fixed 3y | Fail or mixed. Only BTC and XRP passed walk-forward OOS checks (2/5 symbols). |
| **RSI + ATR stops** | 4h, 15m, ATR grid | No consistent improvement. Every 15m variant lost money. |
| **Donchian trend** | v1 (15 months), v2 (5 years) | Fail. Only 1/5 symbols passed a single window under stricter validation. |
| **TS momentum (BTC)** | 30d, 7d, walk-forward | Fail. 30d lost less than buy-and-hold but still lost money. Walk-forward windows had too few trades. |
| **Mean reversion** | Percent-based, 1d | Fail. All symbols negative. |
| **RSI divergence (closed)** | Sweep, cost ladder, execution, confirmation | Fail. Best break-even cost of 6.86 bps/side was below realistic costs. Fresh confirmation: net expectancy −2.15, PF 0.80, 0/3 symbols profitable. |

Detailed reports are in each experiment's folder.

## Takeaways

1. Mean reversion and short timeframes did not survive costs.
2. Isolated per-symbol wins did not generalize.
3. ATR stops did not reliably help.
4. Small-sample tuning kept selecting overfit configurations.

**Conclusion:** Positive results on single symbols or periods are not evidence of a tradable edge. 
*Historical simulations only. Use at your own risk.*
