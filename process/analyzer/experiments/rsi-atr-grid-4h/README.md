# ATR SL/TP Grid — 4hour, 365 Days

Focused experiment across BTC-USDT, BNB-USDT, SOL-USDT, XRP-USDT. Fixed RSI 16 / long 60 / short 40; ATR(14) SL/TP grid with 64 variants; train = first 11/12, month-12 OOS = final 1/12. Base costs 10 bps fee + 5 bps slippage per side; stress costs 20 + 10. Generated at `2026-09-23T00:00:00Z`.

## Per-symbol top-3 month-12 OOS

| Symbol | Rank | Variant | Trades B | Net expectancy B | PF B | Max DD % B | Stderr B | Trades S | Net expectancy S | PF S | Max DD % S | Stderr S | Closed net PnL B | Open mark B |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| [BTC-USDT](BTC-USDT/report.md) | top-1 | SL5.0/TP3.0 | 7 | -13.0299 | 0.1343 | 10.5359 | 5.2439 | 7 | -14.5321 | 0.1104 | 11.4351 | 5.2418 | -91.2091 | 0 |
| [BTC-USDT](BTC-USDT/report.md) | top-2 | SL4.5/TP3.0 | 7 | -13.2968 | 0.1320 | 10.7228 | 5.1839 | 7 | -14.7974 | 0.1087 | 11.6209 | 5.1802 | -93.0779 | 0 |
| [BTC-USDT](BTC-USDT/report.md) | top-3 | SL2.0/TP3.0 | 7 | -8.7477 | 0.1877 | 7.5384 | 3.8773 | 7 | -10.2500 | 0.1496 | 8.4377 | 3.8733 | -61.2338 | 0 |
| [BNB-USDT](BNB-USDT/report.md) | top-1 | SL1.5/TP4.5 | 5 | 2.3089 | 1.3332 | 2.2098 | 8.5239 | 5 | 0.7922 | 1.1011 | 2.5092 | 8.5154 | 11.5447 | 0 |
| [BNB-USDT](BNB-USDT/report.md) | top-2 | SL1.5/TP1.5 | 5 | -4.2549 | 0.3861 | 2.8307 | 4.5291 | 5 | -5.7618 | 0.2644 | 3.4329 | 4.5265 | -21.2745 | 0 |
| [BNB-USDT](BNB-USDT/report.md) | top-3 | SL1.5/TP5.0 | 5 | 3.4029 | 1.4910 | 2.2098 | 9.1921 | 5 | 1.8845 | 1.2406 | 2.5092 | 9.1827 | 17.0146 | 0 |
| [SOL-USDT](SOL-USDT/report.md) | top-1 | SL2.5/TP3.0 | 5 | -2.6212 | 0.8270 | 7.2993 | 14.0210 | 5 | -4.1473 | 0.7417 | 7.7476 | 14.0108 | -13.1062 | 0 |
| [SOL-USDT](SOL-USDT/report.md) | top-2 | SL2.5/TP1.5 | 5 | -9.2151 | 0.3918 | 7.4405 | 9.8895 | 5 | -10.7312 | 0.3317 | 7.8974 | 9.8855 | -46.0753 | 0 |
| [SOL-USDT](SOL-USDT/report.md) | top-3 | SL2.5/TP4.5 | 5 | -10.0215 | 0.4317 | 8.8166 | 12.2840 | 5 | -11.5364 | 0.3875 | 9.4181 | 12.2738 | -50.1074 | 0 |
| [XRP-USDT](XRP-USDT/report.md) | top-1 | SL4.0/TP5.0 | 6 | -24.8495 | 0.2852 | 20.8599 | 16.9107 | 6 | -26.3748 | 0.2679 | 21.6159 | 16.8976 | -149.0968 | 0 |
| [XRP-USDT](XRP-USDT/report.md) | top-2 | SL4.5/TP5.0 | 6 | -25.4604 | 0.2803 | 21.2265 | 17.0220 | 6 | -26.9871 | 0.2634 | 21.9833 | 17.0096 | -152.7626 | 0 |
| [XRP-USDT](XRP-USDT/report.md) | top-3 | SL5.0/TP5.0 | 6 | -25.9771 | 0.2763 | 21.5365 | 17.1604 | 6 | -27.5022 | 0.2598 | 22.2923 | 17.1468 | -155.8626 | 0 |

## Best top-3 variant per symbol (month-12 base)

| Symbol | Best variant | Trades | Net expectancy | PF | Max DD % | Total net PnL | ATR-off net expectancy | ATR-off PF | ATR-off total net PnL | Beat ATR-off (exp+PF)? | Beat ATR-off (total PnL)? |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| [BTC-USDT](BTC-USDT/report.md) | SL2.0/TP3.0 | 7 | -8.7477 | 0.1877 | 7.5384 | -61.2338 | -17.7712 | 0 | -56.0891 | yes | no |
| [BNB-USDT](BNB-USDT/report.md) | SL1.5/TP5.0 | 5 | 3.4029 | 1.4910 | 2.2098 | 17.0146 | -14.1443 | 0.0643 | -25.5108 | yes | yes |
| [SOL-USDT](SOL-USDT/report.md) | SL2.5/TP3.0 | 5 | -2.6212 | 0.8270 | 7.2993 | -13.1062 | -23.5043 | 0 | -30.5222 | yes | yes |
| [XRP-USDT](XRP-USDT/report.md) | SL4.0/TP5.0 | 6 | -24.8495 | 0.2852 | 20.8599 | -149.0968 | -45.0736 | 0 | -164.3340 | yes | yes |

Best is the top-3 config with the highest month-12 base net expectancy (PF then label break ties). This is descriptive only; selection was frozen on the first 11/12. Total net PnL is closed net PnL plus the open mark.

## Notes

- Ranking uses train base costs only. The drawdown gate is applied when it leaves at least 3 variants; otherwise the gate failure is reported and ranking falls back to all variants.
- Execution: signal at close, fill at next open, one position, reversal exits, ATR SL/TP with gap/SL-first ordering.
- Per-symbol artifacts: `report.md`, `leaderboard.csv`, `results.json`.
- This report is not a profitability claim.
