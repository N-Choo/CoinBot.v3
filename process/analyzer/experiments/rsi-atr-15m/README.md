# Fixed RSI 15min Backtest — 1095 Days, No Tuning

One fixed configuration across all symbols: RSI 16, long 60, short 40, no tuning. The baseline has no ATR/SL/TP; the ATR variants use the same signals and add stop-loss/take-profit exits. Generated at `2026-09-23T00:00:00Z`.

| Symbol | Candles | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Win rate % | Verdict | Notes |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| [BTC-USDT](BTC-USDT/report.md) | 105119 | 1909 | -1.5735 | 0.6053 | 307.4849 | 0.2092 | -3003.7850 | -1.3934 | 25.2488 | no base edge | stress expectancy -3.0741 |
| [BNB-USDT](BNB-USDT/report.md) | 105119 | 1903 | -1.5296 | 0.6394 | 293.9022 | 0.2328 | -2910.8914 | -2.5385 | 27.1151 | no base edge | stress expectancy -3.0303 |
| [SOL-USDT](SOL-USDT/report.md) | 105119 | 1860 | -1.1394 | 0.8143 | 217.8674 | 0.3788 | -2119.2113 | -2.8882 | 31.1290 | no base edge | stress expectancy -2.6405 |
| [XRP-USDT](XRP-USDT/report.md) | 105119 | 322 | -3.0599 | 0.4406 | 98.5286 | 0.5832 | -985.2798 | 0 | 25.4658 | no base edge | stress expectancy -4.5606 |

## ATR SL/TP comparison

Signal-at-close, fill-at-next-open. ATR exits can trigger on the entry candle; gap exits use the open, and SL wins when both levels are touched. The high-vol variant multiplies both exits by 1.5 when ATR% > 5.0 and has no stop cap. ATR variants are full-period only.

**Base costs (10 bps fee + 5 bps slippage per side)**

| Symbol | Variant | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Win rate % |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| [BTC-USDT](BTC-USDT/report.md) | ATR off | 1909 | -1.5735 | 0.6053 | 307.4849 | 0.2092 | -3003.7850 | -1.3934 | 25.2488 |
| [BTC-USDT](BTC-USDT/report.md) | ATR14 1.5x/2.0x | 1910 | -1.5533 | 0.2947 | 296.3045 | 0.0676 | -2966.7381 | 0 | 36.3351 |
| [BTC-USDT](BTC-USDT/report.md) | ATR14 2.0x/3.0x | 1910 | -1.5852 | 0.4197 | 301.9029 | 0.0940 | -3027.8273 | 0 | 36.2827 |
| [BTC-USDT](BTC-USDT/report.md) | ATR14 2.5x/4.0x | 1910 | -1.5741 | 0.4923 | 300.5187 | 0.1147 | -3006.6070 | 0 | 35.0785 |
| [BTC-USDT](BTC-USDT/report.md) | ATR14 2.0x/3.0x high-vol x1.5 | 1910 | -1.5852 | 0.4197 | 301.9029 | 0.0940 | -3027.8273 | 0 | 36.2827 |
| [BNB-USDT](BNB-USDT/report.md) | ATR off | 1903 | -1.5296 | 0.6394 | 293.9022 | 0.2328 | -2910.8914 | -2.5385 | 27.1151 |
| [BNB-USDT](BNB-USDT/report.md) | ATR14 1.5x/2.0x | 1904 | -1.5408 | 0.3463 | 293.5967 | 0.0780 | -2933.6962 | 0 | 39.0231 |
| [BNB-USDT](BNB-USDT/report.md) | ATR14 2.0x/3.0x | 1903 | -1.4795 | 0.4888 | 282.9327 | 0.1081 | -2815.5611 | -2.5385 | 39.4640 |
| [BNB-USDT](BNB-USDT/report.md) | ATR14 2.5x/4.0x | 1903 | -1.4549 | 0.5630 | 277.8843 | 0.1307 | -2768.7336 | -2.5385 | 37.3095 |
| [BNB-USDT](BNB-USDT/report.md) | ATR14 2.0x/3.0x high-vol x1.5 | 1903 | -1.4795 | 0.4888 | 282.9327 | 0.1081 | -2815.5611 | -2.5385 | 39.4640 |
| [SOL-USDT](SOL-USDT/report.md) | ATR off | 1860 | -1.1394 | 0.8143 | 217.8674 | 0.3788 | -2119.2113 | -2.8882 | 31.1290 |
| [SOL-USDT](SOL-USDT/report.md) | ATR14 1.5x/2.0x | 1861 | -1.5290 | 0.5502 | 283.7593 | 0.1329 | -2845.3818 | 0 | 42.2891 |
| [SOL-USDT](SOL-USDT/report.md) | ATR14 2.0x/3.0x | 1860 | -1.4266 | 0.6699 | 265.4705 | 0.1857 | -2653.4568 | -2.8882 | 39.0323 |
| [SOL-USDT](SOL-USDT/report.md) | ATR14 2.5x/4.0x | 1860 | -1.4280 | 0.7172 | 265.4618 | 0.2283 | -2656.1215 | -2.8882 | 36.2903 |
| [SOL-USDT](SOL-USDT/report.md) | ATR14 2.0x/3.0x high-vol x1.5 | 1860 | -1.4266 | 0.6699 | 265.4705 | 0.1857 | -2653.4568 | -2.8882 | 39.0323 |
| [XRP-USDT](XRP-USDT/report.md) | ATR off | 322 | -3.0599 | 0.4406 | 98.5286 | 0.5832 | -985.2798 | 0 | 25.4658 |
| [XRP-USDT](XRP-USDT/report.md) | ATR14 1.5x/2.0x | 1838 | -1.6634 | 0.4558 | 306.1958 | 0.1188 | -3057.3898 | 0 | 40.9684 |
| [XRP-USDT](XRP-USDT/report.md) | ATR14 2.0x/3.0x | 1838 | -1.5010 | 0.6014 | 274.4643 | 0.1646 | -2758.7610 | 0 | 39.7171 |
| [XRP-USDT](XRP-USDT/report.md) | ATR14 2.5x/4.0x | 1838 | -1.6133 | 0.6319 | 299.7576 | 0.1980 | -2965.2567 | 0 | 36.0174 |
| [XRP-USDT](XRP-USDT/report.md) | ATR14 2.0x/3.0x high-vol x1.5 | 1838 | -1.5010 | 0.6014 | 274.4643 | 0.1646 | -2758.7610 | 0 | 39.7171 |

**Stress costs (20 bps fee + 10 bps slippage per side)**

| Symbol | Variant | Trades | Net expectancy | PF | Max DD % | Stderr | Closed net PnL | Open mark | Win rate % |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| [BTC-USDT](BTC-USDT/report.md) | ATR off | 1909 | -3.0741 | 0.4026 | 591.5853 | 0.2091 | -5868.4126 | -2.8935 | 20.6915 |
| [BTC-USDT](BTC-USDT/report.md) | ATR14 1.5x/2.0x | 1910 | -3.0532 | 0.0758 | 583.1122 | 0.0676 | -5831.5971 | 0 | 17.4869 |
| [BTC-USDT](BTC-USDT/report.md) | ATR14 2.0x/3.0x | 1910 | -3.0851 | 0.1767 | 588.4515 | 0.0940 | -5892.5631 | 0 | 27.0157 |
| [BTC-USDT](BTC-USDT/report.md) | ATR14 2.5x/4.0x | 1910 | -3.0742 | 0.2509 | 587.1666 | 0.1147 | -5871.6664 | 0 | 30.2618 |
| [BTC-USDT](BTC-USDT/report.md) | ATR14 2.0x/3.0x high-vol x1.5 | 1910 | -3.0851 | 0.1767 | 588.4515 | 0.0940 | -5892.5631 | 0 | 27.0157 |
| [BNB-USDT](BNB-USDT/report.md) | ATR off | 1903 | -3.0303 | 0.4358 | 577.0723 | 0.2328 | -5766.6865 | -4.0369 | 22.5434 |
| [BNB-USDT](BNB-USDT/report.md) | ATR14 1.5x/2.0x | 1904 | -3.0407 | 0.1087 | 578.9498 | 0.0780 | -5789.4980 | 0 | 20.6408 |
| [BNB-USDT](BNB-USDT/report.md) | ATR14 2.0x/3.0x | 1903 | -2.9795 | 0.2271 | 567.6979 | 0.1081 | -5669.9260 | -4.0369 | 30.7409 |
| [BNB-USDT](BNB-USDT/report.md) | ATR14 2.5x/4.0x | 1903 | -2.9551 | 0.3108 | 562.9360 | 0.1307 | -5623.5242 | -4.0369 | 34.1040 |
| [BNB-USDT](BNB-USDT/report.md) | ATR14 2.0x/3.0x high-vol x1.5 | 1903 | -2.9795 | 0.2271 | 567.6979 | 0.1081 | -5669.9260 | -4.0369 | 30.7409 |
| [SOL-USDT](SOL-USDT/report.md) | ATR off | 1860 | -2.6405 | 0.6331 | 481.9946 | 0.3787 | -4911.2894 | -4.3861 | 27.9032 |
| [SOL-USDT](SOL-USDT/report.md) | ATR14 1.5x/2.0x | 1861 | -3.0288 | 0.2940 | 563.6543 | 0.1329 | -5636.5433 | 0 | 38.0978 |
| [SOL-USDT](SOL-USDT/report.md) | ATR14 2.0x/3.0x | 1860 | -2.9264 | 0.4416 | 544.7501 | 0.1857 | -5443.1148 | -4.3861 | 37.9570 |
| [SOL-USDT](SOL-USDT/report.md) | ATR14 2.5x/4.0x | 1860 | -2.9277 | 0.5127 | 544.9896 | 0.2283 | -5445.5097 | -4.3861 | 35.8602 |
| [SOL-USDT](SOL-USDT/report.md) | ATR14 2.0x/3.0x high-vol x1.5 | 1860 | -2.9264 | 0.4416 | 544.7501 | 0.1857 | -5443.1148 | -4.3861 | 37.9570 |
| [XRP-USDT](XRP-USDT/report.md) | ATR off | 322 | -4.5606 | 0.3111 | 146.8502 | 0.5830 | -1468.5018 | 0 | 20.8075 |
| [XRP-USDT](XRP-USDT/report.md) | ATR14 1.5x/2.0x | 1838 | -3.1633 | 0.2112 | 581.4204 | 0.1188 | -5814.2045 | 0 | 30.1959 |
| [XRP-USDT](XRP-USDT/report.md) | ATR14 2.0x/3.0x | 1838 | -3.0007 | 0.3599 | 549.5843 | 0.1646 | -5515.2743 | 0 | 36.2894 |
| [XRP-USDT](XRP-USDT/report.md) | ATR14 2.5x/4.0x | 1838 | -3.1128 | 0.4182 | 571.6796 | 0.1980 | -5721.2501 | 0 | 34.3308 |
| [XRP-USDT](XRP-USDT/report.md) | ATR14 2.0x/3.0x high-vol x1.5 | 1838 | -3.0007 | 0.3599 | 549.5843 | 0.1646 | -5515.2743 | 0 | 36.2894 |

## Notes

- Full-period metrics are net of base costs (10 bps fee + 5 bps slippage per side); stress costs are 20 + 10.
- The requested window spans 1095 days ending at as_of.
- The configuration is fixed and is not tuned on this data.
- Yearly breakdowns cover the ATR-off baseline; ATR variants are reported over the full period only.
- This report is not a profitability claim.

Per-symbol artifacts: `report.md`, `results.json`.
