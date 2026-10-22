# Analyzer Execution Modelling (Experiment 4)

Provisional research result. This report does not claim profitability.

## 1. Run metadata

| Field | Value |
|---|---|
| Generated at | 2026-10-22T00:00:00Z |
| As of (ms) | 1792661878840 |
| Symbols | BTC-USDT, ETH-USDT, SOL-USDT |
| Data dir | data/sweep |
| Settle (ms) | 5000 |
| Init fund | 1000 |
| Warm-up window | 100 |
| Partitions | validation, holdout |
| Candidates | baseline, joint-13, max_age=4 |

## 2. Models

| Model | Entry | Validity | Buffer (bps) | Offset (bps) | Entry cost | TP exit | SL/gap/EOD exit |
|---|---|---|---|---|---|---|---|
| market | next candle open | 1 candle | - | - | taker 15 bps | taker 15 bps | taker 15 bps |
| limit_1c_b2 | limit at signal entry | 1 candle | 2 | 0 | maker 2 bps | maker 2 bps | taker 15 bps |
| limit_2c_b2 | limit at signal entry | 2 candles | 2 | 0 | maker 2 bps | maker 2 bps | taker 15 bps |
| retest_2c_b2 | limit 25 bps better than signal entry | 2 candles | 2 | 25 | maker 2 bps | maker 2 bps | taker 15 bps |

Costs apply once per side on the actual notional; the metrics are already net of them.

## 3. Validation (model selection, pooled across symbols)

| Candidate | Model | Fill rate | Filled/Signals | Expired | Skipped funds | Net expectancy | PF | Trades | Max DD % |
|---|---|---|---|---|---|---|---|---|---|
| baseline | market | 82.31% | 107/130 | 0 | 23 | -0.3436 | 0.96 | 107 | 20.58 |
| baseline | limit_1c_b2 | 77.69% | 101/130 | 5 | 24 | 0.0720 | 1.01 | 101 | 17.56 |
| baseline | limit_2c_b2 | 80.77% | 105/130 | 3 | 22 | 0.3649 | 1.05 | 105 | 17.56 |
| baseline | retest_2c_b2 | 69.23% | 90/130 | 29 | 11 | 1.0907 | 1.14 | 90 | 15.73 |
| joint-13 | market | 38.71% | 60/155 | 0 | 95 | 5.1544 | 1.41 | 60 | 18.20 |
| joint-13 | limit_1c_b2 | 36.13% | 56/155 | 6 | 93 | 5.1509 | 1.40 | 56 | 22.90 |
| joint-13 | limit_2c_b2 | 38.06% | 59/155 | 5 | 91 | 5.1268 | 1.40 | 59 | 21.50 |
| joint-13 | retest_2c_b2 | 41.94% | 65/155 | 34 | 56 | 5.6423 | 1.48 | 65 | 29.48 |
| max_age=4 | market | 87.63% | 85/97 | 0 | 12 | 1.6239 | 1.22 | 85 | 12.85 |
| max_age=4 | limit_1c_b2 | 82.47% | 80/97 | 5 | 12 | 2.2224 | 1.30 | 80 | 10.41 |
| max_age=4 | limit_2c_b2 | 84.54% | 82/97 | 3 | 12 | 2.5473 | 1.36 | 82 | 10.41 |
| max_age=4 | retest_2c_b2 | 71.13% | 69/97 | 22 | 6 | 2.7528 | 1.38 | 69 | 10.68 |

## 4. Holdout (cautionary, pooled across symbols)

| Candidate | Model | Fill rate | Filled/Signals | Expired | Skipped funds | Net expectancy | PF | Trades | Max DD % |
|---|---|---|---|---|---|---|---|---|---|
| baseline | market | 80.00% | 112/140 | 1 | 27 | -0.9988 | 0.86 | 112 | 17.50 |
| baseline | limit_1c_b2 | 79.29% | 111/140 | 4 | 25 | -0.5087 | 0.93 | 111 | 13.57 |
| baseline | limit_2c_b2 | 80.71% | 113/140 | 1 | 26 | -0.5638 | 0.92 | 113 | 13.08 |
| baseline | retest_2c_b2 | 60.71% | 85/140 | 32 | 23 | -0.2435 | 0.97 | 85 | 9.38 |
| joint-13 | market | 33.56% | 50/149 | 1 | 98 | -2.6785 | 0.81 | 50 | 15.99 |
| joint-13 | limit_1c_b2 | 33.56% | 50/149 | 4 | 95 | -1.7975 | 0.87 | 50 | 15.19 |
| joint-13 | limit_2c_b2 | 33.56% | 50/149 | 1 | 98 | -1.7975 | 0.87 | 50 | 15.19 |
| joint-13 | retest_2c_b2 | 36.24% | 54/149 | 35 | 60 | -1.1773 | 0.90 | 54 | 16.52 |
| max_age=4 | market | 83.47% | 101/121 | 0 | 20 | -0.8142 | 0.89 | 101 | 14.28 |
| max_age=4 | limit_1c_b2 | 81.82% | 99/121 | 3 | 19 | -0.2993 | 0.96 | 99 | 12.43 |
| max_age=4 | limit_2c_b2 | 83.47% | 101/121 | 0 | 20 | -0.3650 | 0.95 | 101 | 12.43 |
| max_age=4 | retest_2c_b2 | 64.46% | 78/121 | 27 | 16 | 0.2772 | 1.04 | 78 | 8.93 |

## 5. Per-symbol net expectancy (validation)

| Candidate | Model | Symbol | Fill rate | Filled/Signals | Expired | Net expectancy | PF | Trades |
|---|---|---|---|---|---|---|---|---|
| baseline | market | BTC-USDT | 82.05% | 32/39 | 0 | -2.6947 | 0.64 | 32 |
| baseline | market | ETH-USDT | 79.17% | 38/48 | 0 | 2.5502 | 1.35 | 38 |
| baseline | market | SOL-USDT | 86.05% | 37/43 | 0 | -1.2822 | 0.87 | 37 |
| baseline | limit_1c_b2 | BTC-USDT | 82.05% | 32/39 | 1 | -2.3579 | 0.69 | 32 |
| baseline | limit_1c_b2 | ETH-USDT | 75.00% | 36/48 | 2 | 2.7415 | 1.38 | 36 |
| baseline | limit_1c_b2 | SOL-USDT | 76.74% | 33/43 | 2 | -0.4841 | 0.95 | 33 |
| baseline | limit_2c_b2 | BTC-USDT | 82.05% | 32/39 | 1 | -2.3579 | 0.69 | 32 |
| baseline | limit_2c_b2 | ETH-USDT | 77.08% | 37/48 | 1 | 3.1266 | 1.44 | 37 |
| baseline | limit_2c_b2 | SOL-USDT | 83.72% | 36/43 | 1 | -0.0534 | 0.99 | 36 |
| baseline | retest_2c_b2 | BTC-USDT | 69.23% | 27/39 | 10 | -1.2741 | 0.82 | 27 |
| baseline | retest_2c_b2 | ETH-USDT | 66.67% | 32/48 | 12 | 2.8791 | 1.40 | 32 |
| baseline | retest_2c_b2 | SOL-USDT | 72.09% | 31/43 | 7 | 1.3043 | 1.15 | 31 |
| joint-13 | market | BTC-USDT | 28.57% | 14/49 | 0 | -2.0198 | 0.84 | 14 |
| joint-13 | market | ETH-USDT | 44.44% | 24/54 | 0 | 4.9818 | 1.36 | 24 |
| joint-13 | market | SOL-USDT | 42.31% | 22/52 | 0 | 9.9080 | 1.90 | 22 |
| joint-13 | limit_1c_b2 | BTC-USDT | 26.53% | 13/49 | 1 | -2.7057 | 0.79 | 13 |
| joint-13 | limit_1c_b2 | ETH-USDT | 38.89% | 21/54 | 3 | 3.9953 | 1.26 | 21 |
| joint-13 | limit_1c_b2 | SOL-USDT | 42.31% | 22/52 | 2 | 10.8964 | 2.01 | 22 |
| joint-13 | limit_2c_b2 | BTC-USDT | 26.53% | 13/49 | 1 | -2.7057 | 0.79 | 13 |
| joint-13 | limit_2c_b2 | ETH-USDT | 44.44% | 24/54 | 2 | 4.0807 | 1.28 | 24 |
| joint-13 | limit_2c_b2 | SOL-USDT | 42.31% | 22/52 | 2 | 10.8964 | 2.01 | 22 |
| joint-13 | retest_2c_b2 | BTC-USDT | 40.82% | 20/49 | 11 | 3.5971 | 1.38 | 20 |
| joint-13 | retest_2c_b2 | ETH-USDT | 44.44% | 24/54 | 14 | 2.6969 | 1.18 | 24 |
| joint-13 | retest_2c_b2 | SOL-USDT | 40.38% | 21/52 | 9 | 10.9562 | 2.03 | 21 |
| max_age=4 | market | BTC-USDT | 82.35% | 28/34 | 0 | -1.7613 | 0.76 | 28 |
| max_age=4 | market | ETH-USDT | 90.62% | 29/32 | 0 | 6.0764 | 2.04 | 29 |
| max_age=4 | market | SOL-USDT | 90.32% | 28/31 | 0 | 0.3977 | 1.04 | 28 |
| max_age=4 | limit_1c_b2 | BTC-USDT | 79.41% | 27/34 | 1 | -1.0563 | 0.85 | 27 |
| max_age=4 | limit_1c_b2 | ETH-USDT | 84.38% | 27/32 | 2 | 6.3413 | 2.06 | 27 |
| max_age=4 | limit_1c_b2 | SOL-USDT | 83.87% | 26/31 | 2 | 1.3500 | 1.15 | 26 |
| max_age=4 | limit_2c_b2 | BTC-USDT | 79.41% | 27/34 | 1 | -1.0563 | 0.85 | 27 |
| max_age=4 | limit_2c_b2 | ETH-USDT | 87.50% | 28/32 | 1 | 6.7217 | 2.16 | 28 |
| max_age=4 | limit_2c_b2 | SOL-USDT | 87.10% | 27/31 | 1 | 1.8220 | 1.21 | 27 |
| max_age=4 | retest_2c_b2 | BTC-USDT | 70.59% | 24/34 | 8 | -0.3964 | 0.94 | 24 |
| max_age=4 | retest_2c_b2 | ETH-USDT | 65.62% | 21/32 | 9 | 7.3559 | 2.21 | 21 |
| max_age=4 | retest_2c_b2 | SOL-USDT | 77.42% | 24/31 | 5 | 1.8742 | 1.22 | 24 |

## 6. Per-symbol net expectancy (holdout)

| Candidate | Model | Symbol | Fill rate | Filled/Signals | Expired | Net expectancy | PF | Trades |
|---|---|---|---|---|---|---|---|---|
| baseline | market | BTC-USDT | 86.36% | 38/44 | 1 | -1.9775 | 0.68 | 38 |
| baseline | market | ETH-USDT | 60.00% | 27/45 | 0 | -1.4286 | 0.82 | 27 |
| baseline | market | SOL-USDT | 92.16% | 47/51 | 0 | 0.0393 | 1.01 | 47 |
| baseline | limit_1c_b2 | BTC-USDT | 88.64% | 39/44 | 3 | -0.6320 | 0.89 | 39 |
| baseline | limit_1c_b2 | ETH-USDT | 60.00% | 27/45 | 1 | -0.9154 | 0.88 | 27 |
| baseline | limit_1c_b2 | SOL-USDT | 88.24% | 45/51 | 0 | -0.1578 | 0.98 | 45 |
| baseline | limit_2c_b2 | BTC-USDT | 90.91% | 40/44 | 1 | -0.4945 | 0.91 | 40 |
| baseline | limit_2c_b2 | ETH-USDT | 62.22% | 28/45 | 0 | -1.3151 | 0.84 | 28 |
| baseline | limit_2c_b2 | SOL-USDT | 88.24% | 45/51 | 0 | -0.1578 | 0.98 | 45 |
| baseline | retest_2c_b2 | BTC-USDT | 63.64% | 28/44 | 14 | -0.2798 | 0.95 | 28 |
| baseline | retest_2c_b2 | ETH-USDT | 51.11% | 23/45 | 9 | -1.5102 | 0.81 | 23 |
| baseline | retest_2c_b2 | SOL-USDT | 66.67% | 34/51 | 9 | 0.6434 | 1.08 | 34 |
| joint-13 | market | BTC-USDT | 25.53% | 12/47 | 1 | -12.2141 | 0.23 | 12 |
| joint-13 | market | ETH-USDT | 29.17% | 14/48 | 0 | -5.4235 | 0.63 | 14 |
| joint-13 | market | SOL-USDT | 44.44% | 24/54 | 0 | 3.6906 | 1.30 | 24 |
| joint-13 | limit_1c_b2 | BTC-USDT | 25.53% | 12/47 | 3 | -11.4396 | 0.25 | 12 |
| joint-13 | limit_1c_b2 | ETH-USDT | 29.17% | 14/48 | 1 | -4.5696 | 0.68 | 14 |
| joint-13 | limit_1c_b2 | SOL-USDT | 44.44% | 24/54 | 0 | 4.6406 | 1.38 | 24 |
| joint-13 | limit_2c_b2 | BTC-USDT | 25.53% | 12/47 | 1 | -11.4396 | 0.25 | 12 |
| joint-13 | limit_2c_b2 | ETH-USDT | 29.17% | 14/48 | 0 | -4.5696 | 0.68 | 14 |
| joint-13 | limit_2c_b2 | SOL-USDT | 44.44% | 24/54 | 0 | 4.6406 | 1.38 | 24 |
| joint-13 | retest_2c_b2 | BTC-USDT | 34.04% | 16/47 | 15 | -9.3333 | 0.24 | 16 |
| joint-13 | retest_2c_b2 | ETH-USDT | 29.17% | 14/48 | 10 | -3.8876 | 0.71 | 14 |
| joint-13 | retest_2c_b2 | SOL-USDT | 44.44% | 24/54 | 10 | 5.8410 | 1.51 | 24 |
| max_age=4 | market | BTC-USDT | 94.44% | 34/36 | 0 | -1.5119 | 0.75 | 34 |
| max_age=4 | market | ETH-USDT | 61.54% | 24/39 | 0 | -1.5450 | 0.80 | 24 |
| max_age=4 | market | SOL-USDT | 93.48% | 43/46 | 0 | 0.1454 | 1.02 | 43 |
| max_age=4 | limit_1c_b2 | BTC-USDT | 94.44% | 34/36 | 2 | 0.0912 | 1.02 | 34 |
| max_age=4 | limit_1c_b2 | ETH-USDT | 61.54% | 24/39 | 1 | -1.1099 | 0.86 | 24 |
| max_age=4 | limit_1c_b2 | SOL-USDT | 89.13% | 41/46 | 0 | -0.1486 | 0.98 | 41 |
| max_age=4 | limit_2c_b2 | BTC-USDT | 97.22% | 35/36 | 0 | 0.2277 | 1.04 | 35 |
| max_age=4 | limit_2c_b2 | ETH-USDT | 64.10% | 25/39 | 0 | -1.5499 | 0.80 | 25 |
| max_age=4 | limit_2c_b2 | SOL-USDT | 89.13% | 41/46 | 0 | -0.1486 | 0.98 | 41 |
| max_age=4 | retest_2c_b2 | BTC-USDT | 63.89% | 23/36 | 12 | 0.5925 | 1.11 | 23 |
| max_age=4 | retest_2c_b2 | ETH-USDT | 61.54% | 24/39 | 7 | -0.6492 | 0.91 | 24 |
| max_age=4 | retest_2c_b2 | SOL-USDT | 67.39% | 31/46 | 8 | 0.7606 | 1.10 | 31 |

## 7. Decision

**execution rescues**

Selected model: `max_age=4` / `retest_2c_b2`.

A limit model is a candidate only if pooled validation net expectancy is positive, the validation fill rate is at least 50%, and it beats `market` on validation net expectancy. It rescues execution only if pooled holdout net expectancy is positive, holdout PF is above 1, and at least two of three symbols are positive on the holdout.

| Candidate | Selected model | Validation fill rate | Holdout net expectancy | Holdout PF | Positive symbols | Rescues | Rationale |
|---|---|---|---|---|---|---|---|
| baseline | retest_2c_b2 | 69.23% | -0.2435 | 0.97 | 1/3 | no | retest_2c_b2 passed validation (fill rate 69.23%, net expectancy 1.0907327054573030220969024382) but did not rescue execution: holdout net expectancy -0.2434571617546488307955391197 was not positive; holdout PF 0.97 was not above 1; only 1 of 3 symbols were positive on the holdout. |
| joint-13 | - | - | - | - | 0/3 | no | no limit model met the validation criteria (fill rate >= 50%, positive net expectancy, and better than market) |
| max_age=4 | retest_2c_b2 | 71.13% | 0.2772 | 1.04 | 2/3 | yes | retest_2c_b2 passed validation (fill rate 71.13%, net expectancy 2.752786824416455135066205268) and the holdout held up (net expectancy 0.2772223794830992099283582851, PF 1.04, 2 of 3 symbols positive). |

**Holdout caveat:** The holdout has been scored in several earlier experiments, so it is no longer a clean out-of-sample test. Treat these holdout results as cautionary, and prefer the validation result for model selection.

## 8. Limitations

- Candle-level fill model; no order-book or queue simulation.
- Maker fills assume the limit is resting and filled when touched; adverse selection is approximated by the fill buffer.
- Multiple uses of the same holdout across experiments reduce its evidential value.
- Costs are provisional, not verified KuCoin fees.

