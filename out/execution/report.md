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

| Candidate | Model | Exec fill rate | Capital fill rate | Filled/Signals | Expired | Skipped funds | Net expectancy | SE | 95% CI | PF | Trades | Max DD % |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| baseline | market | 100% | 82.31% | 107/130 | 0 | 23 | -0.3436 | 1.7995 | -3.8707..3.1835 | 0.96 | 107 | 20.58 |
| baseline | limit_1c_b2 | 95.24% | 76.92% | 100/130 | 5 | 25 | 0.0094 | 1.9230 | -3.7597..3.7785 | 1.00 | 100 | 18.72 |
| baseline | limit_2c_b2 | 97.20% | 80.00% | 104/130 | 3 | 23 | 0.3507 | 1.8702 | -3.3149..4.0164 | 1.04 | 104 | 18.28 |
| baseline | retest_2c_b2 | 75.63% | 69.23% | 90/130 | 29 | 11 | 0.9516 | 2.0522 | -3.0706..4.9739 | 1.12 | 90 | 16.48 |
| joint-13 | market | 100% | 38.71% | 60/155 | 0 | 95 | 5.1544 | 4.4355 | -3.5392..13.8480 | 1.41 | 60 | 18.20 |
| joint-13 | limit_1c_b2 | 90.32% | 36.13% | 56/155 | 6 | 93 | 5.1559 | 4.7425 | -4.1394..14.4513 | 1.40 | 56 | 22.90 |
| joint-13 | limit_2c_b2 | 92.19% | 38.06% | 59/155 | 5 | 91 | 5.1317 | 4.5305 | -3.7481..14.0114 | 1.40 | 59 | 21.50 |
| joint-13 | retest_2c_b2 | 65.66% | 41.94% | 65/155 | 34 | 56 | 5.6512 | 4.1737 | -2.5292..13.8316 | 1.48 | 65 | 29.48 |
| max_age=4 | market | 100% | 87.63% | 85/97 | 0 | 12 | 1.6239 | 2.0437 | -2.3817..5.6295 | 1.22 | 85 | 12.85 |
| max_age=4 | limit_1c_b2 | 94.12% | 82.47% | 80/97 | 5 | 12 | 1.9830 | 2.1907 | -2.3107..6.2767 | 1.27 | 80 | 12.32 |
| max_age=4 | limit_2c_b2 | 96.47% | 84.54% | 82/97 | 3 | 12 | 2.3138 | 2.1465 | -1.8933..6.5209 | 1.32 | 82 | 12.32 |
| max_age=4 | retest_2c_b2 | 75.82% | 71.13% | 69/97 | 22 | 6 | 2.4941 | 2.4387 | -2.2857..7.2739 | 1.34 | 69 | 11.43 |

## 4. Holdout (cautionary, pooled across symbols)

| Candidate | Model | Exec fill rate | Capital fill rate | Filled/Signals | Expired | Skipped funds | Net expectancy | SE | 95% CI | PF | Trades | Max DD % |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| baseline | market | 99.12% | 80.00% | 112/140 | 1 | 27 | -0.9988 | 1.3910 | -3.7252..1.7275 | 0.86 | 112 | 17.50 |
| baseline | limit_1c_b2 | 96.52% | 79.29% | 111/140 | 4 | 25 | -0.3740 | 1.4450 | -3.2062..2.4581 | 0.95 | 111 | 13.54 |
| baseline | limit_2c_b2 | 99.12% | 80.71% | 113/140 | 1 | 26 | -0.4315 | 1.4236 | -3.2217..2.3587 | 0.94 | 113 | 13.05 |
| baseline | retest_2c_b2 | 72.65% | 60.71% | 85/140 | 32 | 23 | -0.0679 | 1.6597 | -3.3208..3.1850 | 0.99 | 85 | 9.35 |
| joint-13 | market | 98.04% | 33.56% | 50/149 | 1 | 98 | -2.6785 | 3.8591 | -10.2423..4.8853 | 0.81 | 50 | 15.99 |
| joint-13 | limit_1c_b2 | 92.59% | 33.56% | 50/149 | 4 | 95 | -1.7975 | 3.8990 | -9.4395..5.8445 | 0.87 | 50 | 15.19 |
| joint-13 | limit_2c_b2 | 98.04% | 33.56% | 50/149 | 1 | 98 | -1.7975 | 3.8990 | -9.4395..5.8445 | 0.87 | 50 | 15.19 |
| joint-13 | retest_2c_b2 | 60.67% | 36.24% | 54/149 | 35 | 60 | -1.1773 | 3.5774 | -8.1889..5.8343 | 0.90 | 54 | 16.52 |
| max_age=4 | market | 100% | 83.47% | 101/121 | 0 | 20 | -0.8142 | 1.4823 | -3.7195..2.0911 | 0.89 | 101 | 14.28 |
| max_age=4 | limit_1c_b2 | 97.06% | 81.82% | 99/121 | 3 | 19 | -0.1483 | 1.5560 | -3.1980..2.9014 | 0.98 | 99 | 12.43 |
| max_age=4 | limit_2c_b2 | 100% | 83.47% | 101/121 | 0 | 20 | -0.2171 | 1.5296 | -3.2151..2.7810 | 0.97 | 101 | 12.43 |
| max_age=4 | retest_2c_b2 | 74.77% | 66.12% | 80/121 | 27 | 14 | 0.2242 | 1.7319 | -3.1703..3.6187 | 1.03 | 80 | 8.93 |

## 5. Per-symbol net expectancy (validation)

| Candidate | Model | Symbol | Exec fill rate | Capital fill rate | Filled/Signals | Expired | Net expectancy | PF | Trades |
|---|---|---|---|---|---|---|---|---|---|
| baseline | market | BTC-USDT | 100% | 82.05% | 32/39 | 0 | -2.6947 | 0.64 | 32 |
| baseline | market | ETH-USDT | 100% | 79.17% | 38/48 | 0 | 2.5502 | 1.35 | 38 |
| baseline | market | SOL-USDT | 100% | 86.05% | 37/43 | 0 | -1.2822 | 0.87 | 37 |
| baseline | limit_1c_b2 | BTC-USDT | 96.97% | 82.05% | 32/39 | 1 | -2.1908 | 0.70 | 32 |
| baseline | limit_1c_b2 | ETH-USDT | 94.74% | 75.00% | 36/48 | 2 | 2.7415 | 1.38 | 36 |
| baseline | limit_1c_b2 | SOL-USDT | 94.12% | 74.42% | 32/43 | 2 | -0.8639 | 0.92 | 32 |
| baseline | limit_2c_b2 | BTC-USDT | 96.97% | 82.05% | 32/39 | 1 | -2.1908 | 0.70 | 32 |
| baseline | limit_2c_b2 | ETH-USDT | 97.37% | 77.08% | 37/48 | 1 | 3.1266 | 1.44 | 37 |
| baseline | limit_2c_b2 | SOL-USDT | 97.22% | 81.40% | 35/43 | 1 | -0.2601 | 0.97 | 35 |
| baseline | retest_2c_b2 | BTC-USDT | 72.97% | 69.23% | 27/39 | 10 | -1.0766 | 0.84 | 27 |
| baseline | retest_2c_b2 | ETH-USDT | 72.73% | 66.67% | 32/48 | 12 | 2.9211 | 1.41 | 32 |
| baseline | retest_2c_b2 | SOL-USDT | 81.58% | 72.09% | 31/43 | 7 | 0.6852 | 1.08 | 31 |
| joint-13 | market | BTC-USDT | 100% | 28.57% | 14/49 | 0 | -2.0198 | 0.84 | 14 |
| joint-13 | market | ETH-USDT | 100% | 44.44% | 24/54 | 0 | 4.9818 | 1.36 | 24 |
| joint-13 | market | SOL-USDT | 100% | 42.31% | 22/52 | 0 | 9.9080 | 1.90 | 22 |
| joint-13 | limit_1c_b2 | BTC-USDT | 92.86% | 26.53% | 13/49 | 1 | -2.6838 | 0.79 | 13 |
| joint-13 | limit_1c_b2 | ETH-USDT | 87.50% | 38.89% | 21/54 | 3 | 3.9953 | 1.26 | 21 |
| joint-13 | limit_1c_b2 | SOL-USDT | 91.67% | 42.31% | 22/52 | 2 | 10.8964 | 2.01 | 22 |
| joint-13 | limit_2c_b2 | BTC-USDT | 92.86% | 26.53% | 13/49 | 1 | -2.6838 | 0.79 | 13 |
| joint-13 | limit_2c_b2 | ETH-USDT | 92.31% | 44.44% | 24/54 | 2 | 4.0807 | 1.28 | 24 |
| joint-13 | limit_2c_b2 | SOL-USDT | 91.67% | 42.31% | 22/52 | 2 | 10.8964 | 2.01 | 22 |
| joint-13 | retest_2c_b2 | BTC-USDT | 64.52% | 40.82% | 20/49 | 11 | 3.6261 | 1.38 | 20 |
| joint-13 | retest_2c_b2 | ETH-USDT | 63.16% | 44.44% | 24/54 | 14 | 2.6969 | 1.18 | 24 |
| joint-13 | retest_2c_b2 | SOL-USDT | 70.00% | 40.38% | 21/52 | 9 | 10.9562 | 2.03 | 21 |
| max_age=4 | market | BTC-USDT | 100% | 82.35% | 28/34 | 0 | -1.7613 | 0.76 | 28 |
| max_age=4 | market | ETH-USDT | 100% | 90.62% | 29/32 | 0 | 6.0764 | 2.04 | 29 |
| max_age=4 | market | SOL-USDT | 100% | 90.32% | 28/31 | 0 | 0.3977 | 1.04 | 28 |
| max_age=4 | limit_1c_b2 | BTC-USDT | 96.43% | 79.41% | 27/34 | 1 | -1.0563 | 0.85 | 27 |
| max_age=4 | limit_1c_b2 | ETH-USDT | 93.10% | 84.38% | 27/32 | 2 | 6.3413 | 2.06 | 27 |
| max_age=4 | limit_1c_b2 | SOL-USDT | 92.86% | 83.87% | 26/31 | 2 | 0.6133 | 1.07 | 26 |
| max_age=4 | limit_2c_b2 | BTC-USDT | 96.43% | 79.41% | 27/34 | 1 | -1.0563 | 0.85 | 27 |
| max_age=4 | limit_2c_b2 | ETH-USDT | 96.55% | 87.50% | 28/32 | 1 | 6.7217 | 2.16 | 28 |
| max_age=4 | limit_2c_b2 | SOL-USDT | 96.43% | 87.10% | 27/31 | 1 | 1.1127 | 1.13 | 27 |
| max_age=4 | retest_2c_b2 | BTC-USDT | 75.00% | 70.59% | 24/34 | 8 | -0.3964 | 0.94 | 24 |
| max_age=4 | retest_2c_b2 | ETH-USDT | 70.00% | 65.62% | 21/32 | 9 | 7.4198 | 2.23 | 21 |
| max_age=4 | retest_2c_b2 | SOL-USDT | 82.76% | 77.42% | 24/31 | 5 | 1.0745 | 1.12 | 24 |

## 6. Per-symbol net expectancy (holdout)

| Candidate | Model | Symbol | Exec fill rate | Capital fill rate | Filled/Signals | Expired | Net expectancy | PF | Trades |
|---|---|---|---|---|---|---|---|---|---|
| baseline | market | BTC-USDT | 97.44% | 86.36% | 38/44 | 1 | -1.9775 | 0.68 | 38 |
| baseline | market | ETH-USDT | 100% | 60.00% | 27/45 | 0 | -1.4286 | 0.82 | 27 |
| baseline | market | SOL-USDT | 100% | 92.16% | 47/51 | 0 | 0.0393 | 1.01 | 47 |
| baseline | limit_1c_b2 | BTC-USDT | 92.86% | 88.64% | 39/44 | 3 | -0.6247 | 0.89 | 39 |
| baseline | limit_1c_b2 | ETH-USDT | 96.43% | 60.00% | 27/45 | 1 | -0.4741 | 0.94 | 27 |
| baseline | limit_1c_b2 | SOL-USDT | 100% | 88.24% | 45/51 | 0 | -0.0967 | 0.99 | 45 |
| baseline | limit_2c_b2 | BTC-USDT | 97.56% | 90.91% | 40/44 | 1 | -0.4874 | 0.91 | 40 |
| baseline | limit_2c_b2 | ETH-USDT | 100% | 62.22% | 28/45 | 0 | -0.8897 | 0.88 | 28 |
| baseline | limit_2c_b2 | SOL-USDT | 100% | 88.24% | 45/51 | 0 | -0.0967 | 0.99 | 45 |
| baseline | retest_2c_b2 | BTC-USDT | 66.67% | 63.64% | 28/44 | 14 | -0.2697 | 0.95 | 28 |
| baseline | retest_2c_b2 | ETH-USDT | 71.88% | 51.11% | 23/45 | 9 | -0.9935 | 0.87 | 23 |
| baseline | retest_2c_b2 | SOL-USDT | 79.07% | 66.67% | 34/51 | 9 | 0.7244 | 1.09 | 34 |
| joint-13 | market | BTC-USDT | 92.31% | 25.53% | 12/47 | 1 | -12.2141 | 0.23 | 12 |
| joint-13 | market | ETH-USDT | 100% | 29.17% | 14/48 | 0 | -5.4235 | 0.63 | 14 |
| joint-13 | market | SOL-USDT | 100% | 44.44% | 24/54 | 0 | 3.6906 | 1.30 | 24 |
| joint-13 | limit_1c_b2 | BTC-USDT | 80.00% | 25.53% | 12/47 | 3 | -11.4396 | 0.25 | 12 |
| joint-13 | limit_1c_b2 | ETH-USDT | 93.33% | 29.17% | 14/48 | 1 | -4.5696 | 0.68 | 14 |
| joint-13 | limit_1c_b2 | SOL-USDT | 100% | 44.44% | 24/54 | 0 | 4.6406 | 1.38 | 24 |
| joint-13 | limit_2c_b2 | BTC-USDT | 92.31% | 25.53% | 12/47 | 1 | -11.4396 | 0.25 | 12 |
| joint-13 | limit_2c_b2 | ETH-USDT | 100% | 29.17% | 14/48 | 0 | -4.5696 | 0.68 | 14 |
| joint-13 | limit_2c_b2 | SOL-USDT | 100% | 44.44% | 24/54 | 0 | 4.6406 | 1.38 | 24 |
| joint-13 | retest_2c_b2 | BTC-USDT | 51.61% | 34.04% | 16/47 | 15 | -9.3333 | 0.24 | 16 |
| joint-13 | retest_2c_b2 | ETH-USDT | 58.33% | 29.17% | 14/48 | 10 | -3.8876 | 0.71 | 14 |
| joint-13 | retest_2c_b2 | SOL-USDT | 70.59% | 44.44% | 24/54 | 10 | 5.8410 | 1.51 | 24 |
| max_age=4 | market | BTC-USDT | 100% | 94.44% | 34/36 | 0 | -1.5119 | 0.75 | 34 |
| max_age=4 | market | ETH-USDT | 100% | 61.54% | 24/39 | 0 | -1.5450 | 0.80 | 24 |
| max_age=4 | market | SOL-USDT | 100% | 93.48% | 43/46 | 0 | 0.1454 | 1.02 | 43 |
| max_age=4 | limit_1c_b2 | BTC-USDT | 94.44% | 94.44% | 34/36 | 2 | 0.0997 | 1.02 | 34 |
| max_age=4 | limit_1c_b2 | ETH-USDT | 96.00% | 61.54% | 24/39 | 1 | -0.6136 | 0.92 | 24 |
| max_age=4 | limit_1c_b2 | SOL-USDT | 100% | 89.13% | 41/46 | 0 | -0.0816 | 0.99 | 41 |
| max_age=4 | limit_2c_b2 | BTC-USDT | 100% | 97.22% | 35/36 | 0 | 0.2359 | 1.04 | 35 |
| max_age=4 | limit_2c_b2 | ETH-USDT | 100% | 64.10% | 25/39 | 0 | -1.0734 | 0.86 | 25 |
| max_age=4 | limit_2c_b2 | SOL-USDT | 100% | 89.13% | 41/46 | 0 | -0.0816 | 0.99 | 41 |
| max_age=4 | retest_2c_b2 | BTC-USDT | 65.71% | 63.89% | 23/36 | 12 | 0.6049 | 1.12 | 23 |
| max_age=4 | retest_2c_b2 | ETH-USDT | 78.79% | 66.67% | 26/39 | 7 | -0.8579 | 0.88 | 26 |
| max_age=4 | retest_2c_b2 | SOL-USDT | 79.49% | 67.39% | 31/46 | 8 | 0.8494 | 1.11 | 31 |

## 7. Decision

**nominal rescue — holdout edge is within noise**

Pre-registered rule result: **execution rescues**. Selected model: `max_age=4` / `retest_2c_b2`.

A limit model is a candidate only if pooled validation net expectancy is positive, the validation execution fill rate is at least 50%, and it beats `market` on validation net expectancy. The execution fill rate divides by executable orders (signals minus skipped funds and invalid levels); the capital fill rate still divides by all signals. It rescues execution only if pooled holdout net expectancy is positive, holdout PF is above 1, and at least two of three symbols are positive on the holdout.

| Candidate | Selected model | Validation net expectancy | Val SE | Val 95% CI | Validation exec fill rate | Validation capital fill rate | Validation skipped funds | Holdout net expectancy | Holdout SE | Holdout 95% CI | Holdout PF | Positive symbols | Rescues | Rationale |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| baseline | retest_2c_b2 | 0.9516 | 2.0522 | -3.0706..4.9739 | 75.63% | 69.23% | 11 | -0.0679 | 1.6597 | -3.3208..3.1850 | 0.99 | 1/3 | no | retest_2c_b2 passed validation (execution fill rate 75.63%, capital fill rate 69.23%, net expectancy 0.9516) but did not rescue execution: holdout net expectancy -0.0679 was not positive; holdout PF 0.99 was not above 1; only 1 of 3 symbols were positive on the holdout. |
| joint-13 | retest_2c_b2 | 5.6512 | 4.1737 | -2.5292..13.8316 | 65.66% | 41.94% | 56 | -1.1773 | 3.5774 | -8.1889..5.8343 | 0.90 | 1/3 | no | retest_2c_b2 passed validation (execution fill rate 65.66%, capital fill rate 41.94%, net expectancy 5.6512) but did not rescue execution: holdout net expectancy -1.1773 was not positive; holdout PF 0.90 was not above 1; only 1 of 3 symbols were positive on the holdout. |
| max_age=4 | retest_2c_b2 | 2.4941 | 2.4387 | -2.2857..7.2739 | 75.82% | 71.13% | 6 | 0.2242 | 1.7319 | -3.1703..3.6187 | 1.03 | 2/3 | yes | retest_2c_b2 passed validation (execution fill rate 75.82%, capital fill rate 71.13%, net expectancy 2.4941) and the holdout held up (net expectancy 0.2242, PF 1.03, 2 of 3 symbols positive). |

Both 95% CIs include zero; the validation→holdout decay is ~10×. This is not evidence of a durable edge.

Multiple comparisons: 3 candidates × 3 limit models were tested, and the holdout has been reused across experiments; the nominal rescue should be read against that search.

**Holdout caveat:** The holdout has been scored in several earlier experiments, so it is no longer a clean out-of-sample test. Treat these holdout results as cautionary, and prefer the validation result for model selection.

## 8. Relative to market (selected model)

Selected model: `max_age=4` / `retest_2c_b2`. Deltas are selected minus `market` on the same execution simulator.

| Partition | Symbol | Market net expectancy | retest_2c_b2 net expectancy | Delta |
|---|---|---|---|---|
| validation | BTC-USDT | -1.7613 | -0.3964 | 1.3649 |
| validation | ETH-USDT | 6.0764 | 7.4198 | 1.3434 |
| validation | SOL-USDT | 0.3977 | 1.0745 | 0.6768 |
| holdout | BTC-USDT | -1.5119 | 0.6049 | 2.1168 |
| holdout | ETH-USDT | -1.5450 | -0.8579 | 0.6870 |
| holdout | SOL-USDT | 0.1454 | 0.8494 | 0.7039 |

## 9. Capacity / capital constraint

- `joint-13` produced 155 validation signals but only 60 were funded (`E = init_fund / 2`, max two concurrent positions); 95 were skipped for funds.
- This is a strategy-capacity issue, not an execution-model failure.
- A larger `--init-fund` or smaller `E` would change the comparison; not tested here.

## 10. Market control

The execution simulator closes open positions at partition end (`eod`, taker), while the cost ladder marks them; absolute `market` metrics can differ slightly from the cost-ladder report. Model comparisons are within the same execution simulator.

## 11. Limitations

- Candle-level fill model; no order-book or queue simulation.
- Maker fills assume the limit is resting and filled when touched; adverse selection is approximated by the fill buffer.
- Open-through limit fills use the open price (price improvement) with no placement latency or queue risk; this is optimistic on gap opens.
- Costs are deducted from PnL but not from available capital for sizing; capacity-constrained configs are slightly optimistic.
- Multiple uses of the same holdout across experiments reduce its evidential value.
- Costs are provisional, not verified KuCoin fees.

