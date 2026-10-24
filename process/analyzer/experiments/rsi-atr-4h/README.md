# Fixed RSI 4hour Backtest — 1095 Days, No Tuning

One fixed configuration across all symbols: RSI 16, long 60, short 40, no tuning. The baseline has no ATR/SL/TP; the ATR variants use the same signals and add stop-loss/take-profit exits. Generated at `2026-09-23T00:00:00Z`.

| Symbol | Candles | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Win rate % | Verdict | Notes |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| [BTC-USDT](BTC-USDT/report.md) | 6569 | 121 | 1.8173 | 1.1505 | 20.1466 | 4.0426 | 219.8975 | 50.5383 | 36.3636 | positive base expectancy | stress expectancy 0.3083 |
| [BNB-USDT](BNB-USDT/report.md) | 6569 | 111 | -0.6762 | 0.9542 | 29.0109 | 3.6894 | -75.0538 | 31.0665 | 36.9369 | no base edge | stress expectancy -2.1870 |
| [SOL-USDT](SOL-USDT/report.md) | 6569 | 109 | 9.4465 | 1.4528 | 25.8269 | 8.7681 | 1029.6717 | 63.4951 | 36.6972 | positive base expectancy | stress expectancy 7.9265 |
| [XRP-USDT](XRP-USDT/report.md) | 6569 | 32 | -17.4973 | 0.2142 | 57.8175 | 5.7462 | -559.9150 | 0 | 18.7500 | no base edge | stress expectancy -19.0007 |

## ATR SL/TP comparison

Signal-at-close, fill-at-next-open. ATR exits can trigger on the entry candle; gap exits use the open, and SL wins when both levels are touched. The high-vol variant multiplies both exits by 1.5 when ATR% > 5.0 and has no stop cap. ATR variants are full-period only.

**Base costs (10 bps fee + 5 bps slippage per side)**

| Symbol | Variant | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Win rate % |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| [BTC-USDT](BTC-USDT/report.md) | ATR off | 121 | 1.8173 | 1.1505 | 20.1466 | 4.0426 | 219.8975 | 50.5383 | 36.3636 |
| [BTC-USDT](BTC-USDT/report.md) | ATR14 1.5x/2.0x | 122 | -1.3329 | 0.7973 | 24.3010 | 1.1255 | -162.6084 | 0 | 45.0820 |
| [BTC-USDT](BTC-USDT/report.md) | ATR14 2.0x/3.0x | 122 | 0.1447 | 1.0178 | 14.1944 | 1.5910 | 17.6478 | 0 | 45.0820 |
| [BTC-USDT](BTC-USDT/report.md) | ATR14 2.5x/4.0x | 122 | 0.2662 | 1.0273 | 21.6856 | 1.9706 | 32.4807 | 0 | 42.6230 |
| [BTC-USDT](BTC-USDT/report.md) | ATR14 2.0x/3.0x high-vol x1.5 | 122 | 0.1447 | 1.0178 | 14.1944 | 1.5910 | 17.6478 | 0 | 45.0820 |
| [BNB-USDT](BNB-USDT/report.md) | ATR off | 111 | -0.6762 | 0.9542 | 29.0109 | 3.6894 | -75.0538 | 31.0665 | 36.9369 |
| [BNB-USDT](BNB-USDT/report.md) | ATR14 1.5x/2.0x | 112 | -2.8203 | 0.6634 | 34.8182 | 1.4356 | -315.8690 | 0 | 40.1786 |
| [BNB-USDT](BNB-USDT/report.md) | ATR14 2.0x/3.0x | 112 | -4.8588 | 0.5806 | 55.9698 | 1.8506 | -544.1824 | 0 | 34.8214 |
| [BNB-USDT](BNB-USDT/report.md) | ATR14 2.5x/4.0x | 112 | -2.2708 | 0.8163 | 28.2383 | 2.3623 | -254.3350 | 0 | 39.2857 |
| [BNB-USDT](BNB-USDT/report.md) | ATR14 2.0x/3.0x high-vol x1.5 | 112 | -5.1129 | 0.5681 | 58.8164 | 1.9325 | -572.6482 | 0 | 34.8214 |
| [SOL-USDT](SOL-USDT/report.md) | ATR off | 109 | 9.4465 | 1.4528 | 25.8269 | 8.7681 | 1029.6717 | 63.4951 | 36.6972 |
| [SOL-USDT](SOL-USDT/report.md) | ATR14 1.5x/2.0x | 110 | -2.7795 | 0.7657 | 39.1852 | 2.0778 | -305.7442 | 0 | 40.9091 |
| [SOL-USDT](SOL-USDT/report.md) | ATR14 2.0x/3.0x | 110 | 1.5902 | 1.1129 | 26.9786 | 3.0226 | 174.9207 | 0 | 44.5455 |
| [SOL-USDT](SOL-USDT/report.md) | ATR14 2.5x/4.0x | 110 | 1.0966 | 1.0648 | 27.4110 | 3.5740 | 120.6222 | 0 | 42.7273 |
| [SOL-USDT](SOL-USDT/report.md) | ATR14 2.0x/3.0x high-vol x1.5 | 110 | 0.9379 | 1.0666 | 28.2662 | 2.9240 | 103.1710 | 0 | 44.5455 |
| [XRP-USDT](XRP-USDT/report.md) | ATR off | 32 | -17.4973 | 0.2142 | 57.8175 | 5.7462 | -559.9150 | 0 | 18.7500 |
| [XRP-USDT](XRP-USDT/report.md) | ATR14 1.5x/2.0x | 123 | 1.2587 | 1.1454 | 15.7933 | 1.8371 | 154.8216 | 0 | 49.5935 |
| [XRP-USDT](XRP-USDT/report.md) | ATR14 2.0x/3.0x | 123 | 1.6521 | 1.1433 | 16.6958 | 2.4505 | 203.2085 | 0 | 44.7154 |
| [XRP-USDT](XRP-USDT/report.md) | ATR14 2.5x/4.0x | 123 | -1.8885 | 0.8741 | 33.9980 | 2.8433 | -232.2892 | 0 | 34.9593 |
| [XRP-USDT](XRP-USDT/report.md) | ATR14 2.0x/3.0x high-vol x1.5 | 123 | 1.3395 | 1.1131 | 24.3893 | 2.5200 | 164.7583 | 0 | 44.7154 |

**Stress costs (20 bps fee + 10 bps slippage per side)**

| Symbol | Variant | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Win rate % |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| [BTC-USDT](BTC-USDT/report.md) | ATR off | 121 | 0.3083 | 1.0236 | 27.4313 | 4.0390 | 37.3052 | 48.9601 | 31.4050 |
| [BTC-USDT](BTC-USDT/report.md) | ATR14 1.5x/2.0x | 122 | -2.8321 | 0.6172 | 38.4025 | 1.1255 | -345.5205 | 0 | 45.0820 |
| [BTC-USDT](BTC-USDT/report.md) | ATR14 2.0x/3.0x | 122 | -1.3551 | 0.8486 | 27.7697 | 1.5911 | -165.3210 | 0 | 45.0820 |
| [BTC-USDT](BTC-USDT/report.md) | ATR14 2.5x/4.0x | 122 | -1.2328 | 0.8840 | 29.2687 | 1.9709 | -150.4071 | 0 | 41.8033 |
| [BTC-USDT](BTC-USDT/report.md) | ATR14 2.0x/3.0x high-vol x1.5 | 122 | -1.3551 | 0.8486 | 27.7697 | 1.5911 | -165.3210 | 0 | 45.0820 |
| [BNB-USDT](BNB-USDT/report.md) | ATR off | 111 | -2.1870 | 0.8610 | 36.6759 | 3.6885 | -242.7580 | 29.5175 | 36.0360 |
| [BNB-USDT](BNB-USDT/report.md) | ATR14 1.5x/2.0x | 112 | -4.3205 | 0.5343 | 49.3102 | 1.4361 | -483.9008 | 0 | 40.1786 |
| [BNB-USDT](BNB-USDT/report.md) | ATR14 2.0x/3.0x | 112 | -6.3607 | 0.4937 | 72.6386 | 1.8510 | -712.3959 | 0 | 34.8214 |
| [BNB-USDT](BNB-USDT/report.md) | ATR14 2.5x/4.0x | 112 | -3.7763 | 0.7156 | 44.4342 | 2.3624 | -422.9497 | 0 | 39.2857 |
| [BNB-USDT](BNB-USDT/report.md) | ATR14 2.0x/3.0x high-vol x1.5 | 112 | -6.6152 | 0.4839 | 75.4895 | 1.9331 | -740.9043 | 0 | 34.8214 |
| [SOL-USDT](SOL-USDT/report.md) | ATR off | 109 | 7.9265 | 1.3633 | 29.6809 | 8.7596 | 863.9936 | 61.8974 | 35.7798 |
| [SOL-USDT](SOL-USDT/report.md) | ATR14 1.5x/2.0x | 110 | -4.2839 | 0.6640 | 51.0406 | 2.0779 | -471.2262 | 0 | 40.9091 |
| [SOL-USDT](SOL-USDT/report.md) | ATR14 2.0x/3.0x | 110 | 0.0885 | 1.0059 | 34.6884 | 3.0230 | 9.7341 | 0 | 44.5455 |
| [SOL-USDT](SOL-USDT/report.md) | ATR14 2.5x/4.0x | 110 | -0.4049 | 0.9772 | 34.2689 | 3.5742 | -44.5444 | 0 | 42.7273 |
| [SOL-USDT](SOL-USDT/report.md) | ATR14 2.0x/3.0x high-vol x1.5 | 110 | -0.5648 | 0.9621 | 36.4210 | 2.9242 | -62.1231 | 0 | 44.5455 |
| [XRP-USDT](XRP-USDT/report.md) | ATR off | 32 | -19.0007 | 0.1910 | 62.2591 | 5.7444 | -608.0219 | 0 | 18.7500 |
| [XRP-USDT](XRP-USDT/report.md) | ATR14 1.5x/2.0x | 123 | -0.2378 | 0.9747 | 24.3097 | 1.8377 | -29.2538 | 0 | 49.5935 |
| [XRP-USDT](XRP-USDT/report.md) | ATR14 2.0x/3.0x | 123 | 0.1561 | 1.0126 | 22.4356 | 2.4511 | 19.2041 | 0 | 44.7154 |
| [XRP-USDT](XRP-USDT/report.md) | ATR14 2.5x/4.0x | 123 | -3.3816 | 0.7883 | 46.1741 | 2.8441 | -415.9417 | 0 | 34.9593 |
| [XRP-USDT](XRP-USDT/report.md) | ATR14 2.0x/3.0x high-vol x1.5 | 123 | -0.1569 | 0.9876 | 23.1368 | 2.5208 | -19.3037 | 0 | 44.7154 |

## Notes

- Full-period metrics are net of base costs (10 bps fee + 5 bps slippage per side); stress costs are 20 + 10.
- The requested window spans 1095 days ending at as_of.
- The configuration is fixed and is not tuned on this data.
- Yearly breakdowns cover the ATR-off baseline; ATR variants are reported over the full period only.
- This report is not a profitability claim.

Per-symbol artifacts: `report.md`, `results.json`.
