# Analyzer Hyperparameter Screening Report

Provisional screening results. This report does not claim profitability.

## 1. Run metadata

| Field | Value |
|---|---|
| Generated at | 2026-10-22T00:00:00Z |
| As of (ms) | 1792661878840 |
| Months | 24 |
| Symbols | BTC-USDT, ETH-USDT, SOL-USDT |
| Timeframes | 1hour, 4hour |
| Seed | 20261022 |
| Base costs | 10 bps fee + 5 bps slippage per side |
| Stress costs | 20 bps fee + 10 bps slippage per side |
| Init fund | 1000 |

## 1b. Baseline configuration

Source: pinned study baseline; matches process/analyzer-rs/.env values (`baseline`: RSI 14/o5 age 12 SMA on (20) SL 1.5x TP 2.0x).

| Parameter | Values | Role |
|---|---|---|
| RSI period | 10, 14, 21 | varied |
| RSI order | 3, 5, 7 | varied |
| Max age | 4, 8, 12, uncapped | varied |
| SMA enabled | true, false | varied |
| SMA period | 20 | held constant |
| ATR SL multiplier | 1.5, 2.0, 2.5 | varied |
| ATR TP multiplier | 2.0, 3.0, 4.0 | varied |
| Price diff threshold | 0.05 | held constant |
| ATR period | 14 | held constant |
| ATR pct high | 5.0 | held constant |
| ATR high adjust | 1.5 | held constant |
| ATR SL max pct | 0 | held constant |
| Window | 100 | held constant |

## 2. Data quality

| Symbol | Timeframe | Requested range (ms) | Effective range (ms) | Candles | Requested | Expected | Missing | Duplicates | Pages | SHA-256 |
|---|---|---|---|---|---|---|---|---|---|---|
| BTC-USDT | 1hour | 1730453878840..1792661878840 | 1730455200000..1792656000000 | 17279 | 17281 | 17279 | 0 | 0 | 12 | 80c8ce754a1cd8fa396db61e0892415766586de1dec45f27fe8efa84ce222912 |
| ETH-USDT | 1hour | 1730453878840..1792661878840 | 1730455200000..1792656000000 | 17279 | 17281 | 17279 | 0 | 0 | 12 | 49227307b5cbd514ee2e7c175793f2d081edc1fe1f3d7cb24c9f4c15f32a2bd2 |
| SOL-USDT | 1hour | 1730453878840..1792661878840 | 1730455200000..1792656000000 | 17279 | 17281 | 17279 | 0 | 0 | 12 | cbb57eed5f416e7d4de8d8331522f1e8dd4947370ea9a0eb751159bbfbb11996 |
| BTC-USDT | 4hour | 1730453878840..1792661878840 | 1730462400000..1792641600000 | 4319 | 4321 | 4319 | 0 | 0 | 3 | 377d832f66e43de90b42dfe4af015f78d81f4d9f33940bedb0faf202180cb10f |
| ETH-USDT | 4hour | 1730453878840..1792661878840 | 1730462400000..1792641600000 | 4319 | 4321 | 4319 | 0 | 0 | 3 | 28cc2368ac0cbea635e8c2df9977cd5544b8236538cdd93abc2f21f9e8ca4e3a |
| SOL-USDT | 4hour | 1730453878840..1792661878840 | 1730462400000..1792641600000 | 4319 | 4321 | 4319 | 0 | 0 | 3 | ea4c6bc7f78d543e6a61c7e575a840cb2da66a512147c7308bcfd7af0588b29e |

### Errors

No errors recorded.

### Warnings

No warnings recorded.

## 2b. Chronological splits

Each split is chronological: 60% train, 20% validation, 20% holdout, plus three expanding folds inside train+validation. Indices are half-open (`start` inclusive, `end` exclusive); timestamps are the open times of the first and last candle in each range.

| Symbol | Timeframe | Partition | Start index | End index | Start | End | Fold validation ends <= holdout start |
|---|---|---|---|---|---|---|---|
| BTC-USDT | 1hour | train | 0 | 10367 | 2024-11-01T10:00:00Z | 2026-01-07T08:00:00Z | — |
| BTC-USDT | 1hour | validation | 10367 | 13823 | 2026-01-07T09:00:00Z | 2026-05-31T08:00:00Z | — |
| BTC-USDT | 1hour | holdout | 13823 | 17279 | 2026-05-31T09:00:00Z | 2026-10-22T08:00:00Z | — |
| BTC-USDT | 1hour | fold1_train | 0 | 6911 | 2024-11-01T10:00:00Z | 2025-08-16T08:00:00Z | — |
| BTC-USDT | 1hour | fold1_val | 6911 | 9503 | 2025-08-16T09:00:00Z | 2025-12-02T08:00:00Z | yes |
| BTC-USDT | 1hour | fold2_train | 0 | 9503 | 2024-11-01T10:00:00Z | 2025-12-02T08:00:00Z | — |
| BTC-USDT | 1hour | fold2_val | 9503 | 12095 | 2025-12-02T09:00:00Z | 2026-03-20T08:00:00Z | yes |
| BTC-USDT | 1hour | fold3_train | 0 | 12095 | 2024-11-01T10:00:00Z | 2026-03-20T08:00:00Z | — |
| BTC-USDT | 1hour | fold3_val | 12095 | 13823 | 2026-03-20T09:00:00Z | 2026-05-31T08:00:00Z | yes |
| ETH-USDT | 1hour | train | 0 | 10367 | 2024-11-01T10:00:00Z | 2026-01-07T08:00:00Z | — |
| ETH-USDT | 1hour | validation | 10367 | 13823 | 2026-01-07T09:00:00Z | 2026-05-31T08:00:00Z | — |
| ETH-USDT | 1hour | holdout | 13823 | 17279 | 2026-05-31T09:00:00Z | 2026-10-22T08:00:00Z | — |
| ETH-USDT | 1hour | fold1_train | 0 | 6911 | 2024-11-01T10:00:00Z | 2025-08-16T08:00:00Z | — |
| ETH-USDT | 1hour | fold1_val | 6911 | 9503 | 2025-08-16T09:00:00Z | 2025-12-02T08:00:00Z | yes |
| ETH-USDT | 1hour | fold2_train | 0 | 9503 | 2024-11-01T10:00:00Z | 2025-12-02T08:00:00Z | — |
| ETH-USDT | 1hour | fold2_val | 9503 | 12095 | 2025-12-02T09:00:00Z | 2026-03-20T08:00:00Z | yes |
| ETH-USDT | 1hour | fold3_train | 0 | 12095 | 2024-11-01T10:00:00Z | 2026-03-20T08:00:00Z | — |
| ETH-USDT | 1hour | fold3_val | 12095 | 13823 | 2026-03-20T09:00:00Z | 2026-05-31T08:00:00Z | yes |
| SOL-USDT | 1hour | train | 0 | 10367 | 2024-11-01T10:00:00Z | 2026-01-07T08:00:00Z | — |
| SOL-USDT | 1hour | validation | 10367 | 13823 | 2026-01-07T09:00:00Z | 2026-05-31T08:00:00Z | — |
| SOL-USDT | 1hour | holdout | 13823 | 17279 | 2026-05-31T09:00:00Z | 2026-10-22T08:00:00Z | — |
| SOL-USDT | 1hour | fold1_train | 0 | 6911 | 2024-11-01T10:00:00Z | 2025-08-16T08:00:00Z | — |
| SOL-USDT | 1hour | fold1_val | 6911 | 9503 | 2025-08-16T09:00:00Z | 2025-12-02T08:00:00Z | yes |
| SOL-USDT | 1hour | fold2_train | 0 | 9503 | 2024-11-01T10:00:00Z | 2025-12-02T08:00:00Z | — |
| SOL-USDT | 1hour | fold2_val | 9503 | 12095 | 2025-12-02T09:00:00Z | 2026-03-20T08:00:00Z | yes |
| SOL-USDT | 1hour | fold3_train | 0 | 12095 | 2024-11-01T10:00:00Z | 2026-03-20T08:00:00Z | — |
| SOL-USDT | 1hour | fold3_val | 12095 | 13823 | 2026-03-20T09:00:00Z | 2026-05-31T08:00:00Z | yes |
| BTC-USDT | 4hour | train | 0 | 2591 | 2024-11-01T12:00:00Z | 2026-01-07T04:00:00Z | — |
| BTC-USDT | 4hour | validation | 2591 | 3455 | 2026-01-07T08:00:00Z | 2026-05-31T04:00:00Z | — |
| BTC-USDT | 4hour | holdout | 3455 | 4319 | 2026-05-31T08:00:00Z | 2026-10-22T04:00:00Z | — |
| BTC-USDT | 4hour | fold1_train | 0 | 1727 | 2024-11-01T12:00:00Z | 2025-08-16T04:00:00Z | — |
| BTC-USDT | 4hour | fold1_val | 1727 | 2375 | 2025-08-16T08:00:00Z | 2025-12-02T04:00:00Z | yes |
| BTC-USDT | 4hour | fold2_train | 0 | 2375 | 2024-11-01T12:00:00Z | 2025-12-02T04:00:00Z | — |
| BTC-USDT | 4hour | fold2_val | 2375 | 3023 | 2025-12-02T08:00:00Z | 2026-03-20T04:00:00Z | yes |
| BTC-USDT | 4hour | fold3_train | 0 | 3023 | 2024-11-01T12:00:00Z | 2026-03-20T04:00:00Z | — |
| BTC-USDT | 4hour | fold3_val | 3023 | 3455 | 2026-03-20T08:00:00Z | 2026-05-31T04:00:00Z | yes |
| ETH-USDT | 4hour | train | 0 | 2591 | 2024-11-01T12:00:00Z | 2026-01-07T04:00:00Z | — |
| ETH-USDT | 4hour | validation | 2591 | 3455 | 2026-01-07T08:00:00Z | 2026-05-31T04:00:00Z | — |
| ETH-USDT | 4hour | holdout | 3455 | 4319 | 2026-05-31T08:00:00Z | 2026-10-22T04:00:00Z | — |
| ETH-USDT | 4hour | fold1_train | 0 | 1727 | 2024-11-01T12:00:00Z | 2025-08-16T04:00:00Z | — |
| ETH-USDT | 4hour | fold1_val | 1727 | 2375 | 2025-08-16T08:00:00Z | 2025-12-02T04:00:00Z | yes |
| ETH-USDT | 4hour | fold2_train | 0 | 2375 | 2024-11-01T12:00:00Z | 2025-12-02T04:00:00Z | — |
| ETH-USDT | 4hour | fold2_val | 2375 | 3023 | 2025-12-02T08:00:00Z | 2026-03-20T04:00:00Z | yes |
| ETH-USDT | 4hour | fold3_train | 0 | 3023 | 2024-11-01T12:00:00Z | 2026-03-20T04:00:00Z | — |
| ETH-USDT | 4hour | fold3_val | 3023 | 3455 | 2026-03-20T08:00:00Z | 2026-05-31T04:00:00Z | yes |
| SOL-USDT | 4hour | train | 0 | 2591 | 2024-11-01T12:00:00Z | 2026-01-07T04:00:00Z | — |
| SOL-USDT | 4hour | validation | 2591 | 3455 | 2026-01-07T08:00:00Z | 2026-05-31T04:00:00Z | — |
| SOL-USDT | 4hour | holdout | 3455 | 4319 | 2026-05-31T08:00:00Z | 2026-10-22T04:00:00Z | — |
| SOL-USDT | 4hour | fold1_train | 0 | 1727 | 2024-11-01T12:00:00Z | 2025-08-16T04:00:00Z | — |
| SOL-USDT | 4hour | fold1_val | 1727 | 2375 | 2025-08-16T08:00:00Z | 2025-12-02T04:00:00Z | yes |
| SOL-USDT | 4hour | fold2_train | 0 | 2375 | 2024-11-01T12:00:00Z | 2025-12-02T04:00:00Z | — |
| SOL-USDT | 4hour | fold2_val | 2375 | 3023 | 2025-12-02T08:00:00Z | 2026-03-20T04:00:00Z | yes |
| SOL-USDT | 4hour | fold3_train | 0 | 3023 | 2024-11-01T12:00:00Z | 2026-03-20T04:00:00Z | — |
| SOL-USDT | 4hour | fold3_val | 3023 | 3455 | 2026-03-20T08:00:00Z | 2026-05-31T04:00:00Z | yes |

Every `fold*_val` row above ends at or before the holdout start index, so no fold validation overlaps the holdout.

## 3. Baseline vs candidates

Validation is the selection partition; holdout is the one-shot evaluation partition, scored only for the baseline and selected winners, and is never used for selection. Metrics are pooled across symbols under base costs. A missing holdout is shown as —.

Footnote: *pooled max DD* is the maximum per-symbol drawdown, not a portfolio drawdown.

### 1hour

| Label | Config | Val trades | Val net expectancy | Val PF | Val max DD % | Val stderr | Holdout trades | Holdout net expectancy | Holdout PF | Holdout max DD % | Status |
|---|---|---|---|---|---|---|---|---|---|---|---|
| atr_sl=2.0 | RSI 14/o5 age 12 SMA on (20) SL 2.0x TP 2.0x | 314 | -2.5571694707147252613604654389 | 0.5709465730665623940826788867 | 39.367497298159221841673947060 | 0.589757608499826 | — | — | — | — | not selected |
| atr_sl=2.5 | RSI 14/o5 age 12 SMA on (20) SL 2.5x TP 2.0x | 295 | -2.9122325145967907251436111115 | 0.5603469933797797294793257359 | 50.995083486900626331310529800 | 0.6954208188327 | — | — | — | — | not selected |
| atr_tp=3.0 | RSI 14/o5 age 12 SMA on (20) SL 1.5x TP 3.0x | 355 | -1.8736724239718110091809321424 | 0.6654830435468961095141110455 | 35.379374742680898435492742470 | 0.541211228718798 | — | — | — | — | not selected |
| atr_tp=4.0 | RSI 14/o5 age 12 SMA on (20) SL 1.5x TP 4.0x | 325 | -2.0421589053294709102468788283 | 0.6659584488776687625976310111 | 34.368194141417745031347053250 | 0.632299053509797 | — | — | — | — | not selected |
| baseline | RSI 14/o5 age 12 SMA on (20) SL 1.5x TP 2.0x | 349 | -2.2756389093198688645788789298 | 0.5593911474733819241689210305 | 41.238775049464617936561525180 | 0.463269338375174 | 452 | -0.8497291995507805785405017629 | 0.766355045749604537501338421 | 30.838689745079694158573258470 | not selected |
| joint-13 | RSI 10/o5 age uncapped SMA on (20) SL 2.5x TP 4.0x | 277 | -2.2608412480278265517104560648 | 0.7323979633096239056328320331 | 48.283846200779269137785066880 | 0.968954714733916 | — | — | — | — | not selected |
| joint-14 | RSI 21/o3 age 8 SMA on (20) SL 1.5x TP 4.0x | 382 | -1.2229419628785363810836129575 | 0.7881984706804089079352077047 | 34.997606061423596539204795510 | 0.603725457883028 | — | — | — | — | not selected |
| joint-15 | RSI 10/o5 age 4 SMA off (20) SL 2.5x TP 2.0x | 382 | -2.099122075553174588182925827 | 0.6661784068804430340525432163 | 44.680922267616889202963780600 | 0.620413680282169 | — | — | — | — | not selected |
| joint-16 | RSI 10/o3 age 12 SMA off (20) SL 2.0x TP 2.0x | 555 | -0.8366349525103096680518275622 | 0.8263740416330190766511371478 | 38.442182166651203361417654650 | 0.4146170511725794 | — | — | — | — | not selected |
| joint-17 | RSI 10/o7 age 8 SMA on (20) SL 1.5x TP 2.0x | 380 | -1.7573439924048155417515780315 | 0.6355180799266365351403670211 | 28.276651724531891365560866770 | 0.4344043173338806 | — | — | — | — | not selected |
| joint-18 | RSI 21/o7 age 4 SMA on (20) SL 2.0x TP 3.0x | 234 | -1.9099164152233453044626640696 | 0.7089967807796233099270992683 | 25.115179635304358424567747060 | 0.800648885844032 | — | — | — | — | not selected |
| joint-19 | RSI 14/o7 age uncapped SMA off (20) SL 1.5x TP 3.0x | 432 | -1.5585864541621072536860518582 | 0.7244222950933977579623612152 | 30.181980419185410248125663170 | 0.515821158338258 | — | — | — | — | not selected |
| joint-20 | RSI 14/o5 age 4 SMA off (20) SL 2.5x TP 3.0x | 324 | -1.8425156933767581300010080824 | 0.7549486178761338772372397469 | 37.502909468656976755779273080 | 0.817369526958437 | — | — | — | — | not selected |
| joint-21 | RSI 14/o5 age 12 SMA off (20) SL 2.5x TP 2.0x | 436 | -1.8020775782815828282691382368 | 0.6968597205005871355923251894 | 44.253294744027244511410609240 | 0.561823850338386 | — | — | — | — | not selected |
| joint-22 | RSI 14/o3 age uncapped SMA off (20) SL 2.5x TP 3.0x | 410 | -1.5924156619619692833044697851 | 0.7781537523334309931365583035 | 54.944106495320278477579786760 | 0.694037320961768 | — | — | — | — | not selected |
| joint-23 | RSI 10/o3 age 12 SMA off (20) SL 1.5x TP 4.0x | 481 | -0.730153378693618110237360129 | 0.8680550278355575499662076056 | 23.338991544356644587753728980 | 0.555699230216584 | — | — | — | — | not selected |
| joint-24 | RSI 21/o5 age 12 SMA on (20) SL 2.0x TP 4.0x | 269 | -2.0574086300899814465309970586 | 0.720208825125200013295476557 | 36.757008747910587506513636470 | 0.849101476238594 | — | — | — | — | not selected |
| joint-25 | RSI 21/o3 age 8 SMA off (20) SL 2.5x TP 2.0x | 474 | -1.6489487798408433705644109776 | 0.7186611966893347085602189464 | 58.660587070734015903543692080 | 0.520798966875855 | — | — | — | — | not selected |
| joint-26 | RSI 21/o7 age uncapped SMA off (20) SL 2.0x TP 4.0x | 325 | -1.5732776723229107833005165151 | 0.7781361998042185619385286328 | 28.890775616355923392162469460 | 0.761294596775427 | — | — | — | — | not selected |
| joint-27 | RSI 10/o3 age uncapped SMA on (20) SL 1.5x TP 2.0x | 509 | -1.2968214858324318224822597012 | 0.7106221312704952289936610083 | 38.835340728040808865725076660 | 0.369454446607217 | — | — | — | — | not selected |
| joint-28 | RSI 10/o7 age 8 SMA off (20) SL 1.5x TP 4.0x | 342 | -2.1722610851662954290438821699 | 0.654980995650629147213701732 | 34.844158167048196795851707460 | 0.642502031548914 | — | — | — | — | not selected |
| joint-29 | RSI 10/o5 age uncapped SMA on (20) SL 1.5x TP 2.0x | 410 | -1.7969432995729604995905275241 | 0.6263972995946476380281488574 | 32.177435291928234072195865330 | 0.4204359674383971 | — | — | — | — | not selected |
| max_age=4 | RSI 14/o5 age 4 SMA on (20) SL 1.5x TP 2.0x | 309 | -2.1356910182530670175189803374 | 0.5781582674024795125522618099 | 34.429904138227105848619198170 | 0.493659440479588 | — | — | — | — | not selected |
| max_age=8 | RSI 14/o5 age 8 SMA on (20) SL 1.5x TP 2.0x | 349 | -2.1473536482918505163116811815 | 0.5695709332808311105625169408 | 38.527050251300844123159845440 | 0.452556784390994 | — | — | — | — | not selected |
| max_age=uncapped | RSI 14/o5 age uncapped SMA on (20) SL 1.5x TP 2.0x | 379 | -2.1070606508012960762470125873 | 0.5795741784804160536081277933 | 39.634030830938405135261884380 | 0.4386251187248131 | — | — | — | — | not selected |
| rsi_order=3 | RSI 14/o3 age 12 SMA on (20) SL 1.5x TP 2.0x | 456 | -1.5527603848318875667144086238 | 0.6684232502110554726256780008 | 40.850145732140565768762457220 | 0.3961013970491795 | — | — | — | — | not selected |
| rsi_order=7 | RSI 14/o7 age 12 SMA on (20) SL 1.5x TP 2.0x | 369 | -1.7568494869048482052144442439 | 0.6451415036980639995015236664 | 27.503844714497336847888169330 | 0.458872569992684 | — | — | — | — | not selected |
| rsi_period=10 | RSI 10/o5 age 12 SMA on (20) SL 1.5x TP 2.0x | 389 | -1.9328187477392137292896010777 | 0.607103498066566739268548589 | 34.213112069557082142612848080 | 0.4350707761394098 | — | — | — | — | not selected |
| rsi_period=21 | RSI 21/o5 age 12 SMA on (20) SL 1.5x TP 2.0x | 380 | -1.8260773514074847063532695215 | 0.6232577753597165877325713371 | 35.402678516045039767002076810 | 0.4325676120022438 | — | — | — | — | not selected |
| sma_enabled=false | RSI 14/o5 age 12 SMA off (20) SL 1.5x TP 2.0x | 498 | -1.9430900671694931695905807145 | 0.6035477706091692279564276667 | 40.579853439257747469065265560 | 0.3810686914464747 | — | — | — | — | not selected |

### 4hour

| Label | Config | Val trades | Val net expectancy | Val PF | Val max DD % | Val stderr | Holdout trades | Holdout net expectancy | Holdout PF | Holdout max DD % | Status |
|---|---|---|---|---|---|---|---|---|---|---|---|
| atr_sl=2.0 | RSI 14/o5 age 12 SMA on (20) SL 2.0x TP 2.0x | 94 | 0.7493886721371158159011275036 | 1.0854841704332374484640203709 | 16.271505118437684816224061740 | 2.077136948523779 | — | — | — | — | not selected |
| atr_sl=2.5 | RSI 14/o5 age 12 SMA on (20) SL 2.5x TP 2.0x | 87 | 0.4338097468925774789572704139 | 1.0437329115304262910460021252 | 16.893305713363231953592635790 | 2.39935473541493 | — | — | — | — | not selected |
| atr_tp=3.0 | RSI 14/o5 age 12 SMA on (20) SL 1.5x TP 3.0x | 96 | 0.0621041473413860206256060998 | 1.0064590568067215103082860105 | 17.186687111086739988429576150 | 2.317277143173072 | — | — | — | — | not selected |
| atr_tp=4.0 | RSI 14/o5 age 12 SMA on (20) SL 1.5x TP 4.0x | 89 | 0.3944570430708520966338561564 | 1.0363602111560926769436434099 | 17.000516988007658469286741990 | 2.861388443675215 | 63 | -5.078658921718099057610802569 | 0.5327462001743584586193379677 | 14.609257669532246809702365760 | selected |
| baseline | RSI 14/o5 age 12 SMA on (20) SL 1.5x TP 2.0x | 106 | -0.3297681368638819673672562759 | 0.9603597535951916446428764108 | 20.579867000202170273738499980 | 1.816740811645981 | 111 | -1.0404840172951392282469820971 | 0.8586490441863132459905632911 | 17.503495901373234494194306140 | not selected |
| joint-13 | RSI 10/o5 age uncapped SMA on (20) SL 2.5x TP 4.0x | 57 | 4.9963629646896502938864068795 | 1.3797982013681379426871417104 | 18.201787006085755708658271600 | 4.67459872397945 | 47 | -3.0424747826336476022237138843 | 0.7919939347693540467087625813 | 15.989963734785018953244315420 | selected |
| joint-14 | RSI 21/o3 age 8 SMA on (20) SL 1.5x TP 4.0x | 84 | -1.4024878266485900397421274542 | 0.8762762312362350143079856154 | 23.485886607239477358473548560 | 2.850933488834778 | — | — | — | — | not selected |
| joint-15 | RSI 10/o5 age 4 SMA off (20) SL 2.5x TP 2.0x | 107 | 0.0546362728277884868254831391 | 1.0054950682038354624496607683 | 18.064617968743183116129621130 | 2.131394957838514 | — | — | — | — | not selected |
| joint-16 | RSI 10/o3 age 12 SMA off (20) SL 2.0x TP 2.0x | 149 | 1.2188128288184766504573215937 | 1.1476277369391726169977430967 | 14.703378280239423031801251320 | 1.594230748130059 | — | — | — | — | not selected |
| joint-17 | RSI 10/o7 age 8 SMA on (20) SL 1.5x TP 2.0x | 86 | 0.8967935146880126886629378647 | 1.1131304291766579467789948547 | 11.340711892984336140688686890 | 2.046632603118035 | — | — | — | — | not selected |
| joint-18 | RSI 21/o7 age 4 SMA on (20) SL 2.0x TP 3.0x | 37 | 0.8460379435951455885146220815 | 1.0696991346592629091327774549 | 11.093856530108771114154972070 | 4.82458459136208 | — | — | — | — | not selected |
| joint-19 | RSI 14/o7 age uncapped SMA off (20) SL 1.5x TP 3.0x | 99 | -2.9259282329677132788248018142 | 0.7336322205227747491859516592 | 23.533726933230182046986019960 | 2.174212514755826 | — | — | — | — | not selected |
| joint-20 | RSI 14/o5 age 4 SMA off (20) SL 2.5x TP 3.0x | 86 | -0.6438729878973568798116604113 | 0.9511505216428655379304800775 | 20.751732191475608876612874870 | 3.076633897770187 | — | — | — | — | not selected |
| joint-21 | RSI 14/o5 age 12 SMA off (20) SL 2.5x TP 2.0x | 106 | -2.8875149828828357161005336103 | 0.753638085446851435000978652 | 25.096417896974659839200386310 | 2.188182285775757 | — | — | — | — | not selected |
| joint-22 | RSI 14/o3 age uncapped SMA off (20) SL 2.5x TP 3.0x | 80 | -3.4981756916738636717533105798 | 0.7567136905665636701318582894 | 28.251973715481880983085341440 | 3.067504878315702 | — | — | — | — | not selected |
| joint-23 | RSI 10/o3 age 12 SMA off (20) SL 1.5x TP 4.0x | 132 | -0.033195853441758664913581889 | 0.9968021265390219412582861198 | 22.742208153417056603958733410 | 2.167052822981022 | — | — | — | — | not selected |
| joint-24 | RSI 21/o5 age 12 SMA on (20) SL 2.0x TP 4.0x | 73 | 4.6193629509363201989891794023 | 1.4363300742995956173527302993 | 13.942721226263134510032097470 | 3.478832891819053 | 60 | -3.9043150207849903147032745025 | 0.6922820255552837907444450868 | 14.398000976078659072862482290 | selected |
| joint-25 | RSI 21/o3 age 8 SMA off (20) SL 2.5x TP 2.0x | 117 | -0.5193204832220861980952717938 | 0.9506087399230016918609903029 | 17.739071952935547938825554500 | 2.101722638900523 | — | — | — | — | not selected |
| joint-26 | RSI 21/o7 age uncapped SMA off (20) SL 2.0x TP 4.0x | 65 | 6.2239702369496102901872541205 | 1.6002345324019538680984048873 | 12.542152420101327181663738120 | 3.65172336866485 | — | — | — | — | not selected |
| joint-27 | RSI 10/o3 age uncapped SMA on (20) SL 1.5x TP 2.0x | 138 | 0.0106395657960088033437472141 | 1.0013263063952946536526563978 | 15.722317076536552686779049940 | 1.539674427379141 | — | — | — | — | not selected |
| joint-28 | RSI 10/o7 age 8 SMA off (20) SL 1.5x TP 4.0x | 110 | -0.4011097580767227915791912797 | 0.9638321888870341130621358894 | 28.488595792919374735602924590 | 2.439584392276764 | — | — | — | — | not selected |
| joint-29 | RSI 10/o5 age uncapped SMA on (20) SL 1.5x TP 2.0x | 125 | -0.0528471070204967697173102466 | 0.9934883583121279005034205461 | 16.253690661330367412216099220 | 1.643420219584867 | — | — | — | — | not selected |
| max_age=4 | RSI 14/o5 age 4 SMA on (20) SL 1.5x TP 2.0x | 85 | 1.6239219350009629295744950153 | 1.2214181945431479923947603154 | 12.847498364384181139178986440 | 2.043670242398436 | 101 | -0.814170014744723530881009295 | 0.8882122921418198190183868919 | 14.275309785426725819500507730 | selected |
| max_age=8 | RSI 14/o5 age 8 SMA on (20) SL 1.5x TP 2.0x | 103 | 0.0139317296511954309703472406 | 1.0017226057583695892810717433 | 19.247652172601314480591616770 | 1.837544045647045 | — | — | — | — | not selected |
| max_age=uncapped | RSI 14/o5 age uncapped SMA on (20) SL 1.5x TP 2.0x | 108 | -0.5668860284850826311667773842 | 0.9325793201055525418275268431 | 20.579867000202170273738499980 | 1.791872916471305 | — | — | — | — | not selected |
| rsi_order=3 | RSI 14/o3 age 12 SMA on (20) SL 1.5x TP 2.0x | 126 | -0.7737511434583780609421093854 | 0.9086568994717049339456005369 | 22.980294733221312899386638640 | 1.624252289749509 | — | — | — | — | not selected |
| rsi_order=7 | RSI 14/o7 age 12 SMA on (20) SL 1.5x TP 2.0x | 71 | 0.3002697605673949279146968277 | 1.03500586154926903310385992 | 18.173239636796865377173994600 | 2.242571716006875 | — | — | — | — | not selected |
| rsi_period=10 | RSI 10/o5 age 12 SMA on (20) SL 1.5x TP 2.0x | 123 | 0.1598567495224011334180407434 | 1.0198970600539529962989372464 | 16.253690661330367412216099220 | 1.662920962096941 | — | — | — | — | not selected |
| rsi_period=21 | RSI 21/o5 age 12 SMA on (20) SL 1.5x TP 2.0x | 98 | 0.5344876416472138861583478666 | 1.067954297167724524265012608 | 18.490406114933161986349510850 | 1.905596032009842 | — | — | — | — | not selected |
| sma_enabled=false | RSI 14/o5 age 12 SMA off (20) SL 1.5x TP 2.0x | 143 | -1.3824124257297142964788717519 | 0.8426050093925050997772043767 | 20.984221418835305596966035560 | 1.525311355088506 | — | — | — | — | not selected |

## 4. Exact configurations tested and reasons

The `origin` column records how each candidate entered the study: `baseline`, `ofat:<parameter>=<value>` (one factor at a time), or `joint` (deterministic seeded draw).

### 1hour

| Label | Origin | RSI period | RSI order | Max age | SMA enabled | SMA period | ATR SL | ATR TP |
|---|---|---|---|---|---|---|---|---|
| atr_sl=2.0 | ofat:atr_sl=2.0 | 14 | 5 | 12 | true | 20 | 2.0 | 2.0 |
| atr_sl=2.5 | ofat:atr_sl=2.5 | 14 | 5 | 12 | true | 20 | 2.5 | 2.0 |
| atr_tp=3.0 | ofat:atr_tp=3.0 | 14 | 5 | 12 | true | 20 | 1.5 | 3.0 |
| atr_tp=4.0 | ofat:atr_tp=4.0 | 14 | 5 | 12 | true | 20 | 1.5 | 4.0 |
| baseline | baseline | 14 | 5 | 12 | true | 20 | 1.5 | 2.0 |
| joint-13 | joint | 10 | 5 | uncapped | true | 20 | 2.5 | 4.0 |
| joint-14 | joint | 21 | 3 | 8 | true | 20 | 1.5 | 4.0 |
| joint-15 | joint | 10 | 5 | 4 | false | 20 | 2.5 | 2.0 |
| joint-16 | joint | 10 | 3 | 12 | false | 20 | 2.0 | 2.0 |
| joint-17 | joint | 10 | 7 | 8 | true | 20 | 1.5 | 2.0 |
| joint-18 | joint | 21 | 7 | 4 | true | 20 | 2.0 | 3.0 |
| joint-19 | joint | 14 | 7 | uncapped | false | 20 | 1.5 | 3.0 |
| joint-20 | joint | 14 | 5 | 4 | false | 20 | 2.5 | 3.0 |
| joint-21 | joint | 14 | 5 | 12 | false | 20 | 2.5 | 2.0 |
| joint-22 | joint | 14 | 3 | uncapped | false | 20 | 2.5 | 3.0 |
| joint-23 | joint | 10 | 3 | 12 | false | 20 | 1.5 | 4.0 |
| joint-24 | joint | 21 | 5 | 12 | true | 20 | 2.0 | 4.0 |
| joint-25 | joint | 21 | 3 | 8 | false | 20 | 2.5 | 2.0 |
| joint-26 | joint | 21 | 7 | uncapped | false | 20 | 2.0 | 4.0 |
| joint-27 | joint | 10 | 3 | uncapped | true | 20 | 1.5 | 2.0 |
| joint-28 | joint | 10 | 7 | 8 | false | 20 | 1.5 | 4.0 |
| joint-29 | joint | 10 | 5 | uncapped | true | 20 | 1.5 | 2.0 |
| max_age=4 | ofat:max_age=4 | 14 | 5 | 4 | true | 20 | 1.5 | 2.0 |
| max_age=8 | ofat:max_age=8 | 14 | 5 | 8 | true | 20 | 1.5 | 2.0 |
| max_age=uncapped | ofat:max_age=uncapped | 14 | 5 | uncapped | true | 20 | 1.5 | 2.0 |
| rsi_order=3 | ofat:rsi_order=3 | 14 | 3 | 12 | true | 20 | 1.5 | 2.0 |
| rsi_order=7 | ofat:rsi_order=7 | 14 | 7 | 12 | true | 20 | 1.5 | 2.0 |
| rsi_period=10 | ofat:rsi_period=10 | 10 | 5 | 12 | true | 20 | 1.5 | 2.0 |
| rsi_period=21 | ofat:rsi_period=21 | 21 | 5 | 12 | true | 20 | 1.5 | 2.0 |
| sma_enabled=false | ofat:sma_enabled=false | 14 | 5 | 12 | false | 20 | 1.5 | 2.0 |

### 4hour

| Label | Origin | RSI period | RSI order | Max age | SMA enabled | SMA period | ATR SL | ATR TP |
|---|---|---|---|---|---|---|---|---|
| atr_sl=2.0 | ofat:atr_sl=2.0 | 14 | 5 | 12 | true | 20 | 2.0 | 2.0 |
| atr_sl=2.5 | ofat:atr_sl=2.5 | 14 | 5 | 12 | true | 20 | 2.5 | 2.0 |
| atr_tp=3.0 | ofat:atr_tp=3.0 | 14 | 5 | 12 | true | 20 | 1.5 | 3.0 |
| atr_tp=4.0 | ofat:atr_tp=4.0 | 14 | 5 | 12 | true | 20 | 1.5 | 4.0 |
| baseline | baseline | 14 | 5 | 12 | true | 20 | 1.5 | 2.0 |
| joint-13 | joint | 10 | 5 | uncapped | true | 20 | 2.5 | 4.0 |
| joint-14 | joint | 21 | 3 | 8 | true | 20 | 1.5 | 4.0 |
| joint-15 | joint | 10 | 5 | 4 | false | 20 | 2.5 | 2.0 |
| joint-16 | joint | 10 | 3 | 12 | false | 20 | 2.0 | 2.0 |
| joint-17 | joint | 10 | 7 | 8 | true | 20 | 1.5 | 2.0 |
| joint-18 | joint | 21 | 7 | 4 | true | 20 | 2.0 | 3.0 |
| joint-19 | joint | 14 | 7 | uncapped | false | 20 | 1.5 | 3.0 |
| joint-20 | joint | 14 | 5 | 4 | false | 20 | 2.5 | 3.0 |
| joint-21 | joint | 14 | 5 | 12 | false | 20 | 2.5 | 2.0 |
| joint-22 | joint | 14 | 3 | uncapped | false | 20 | 2.5 | 3.0 |
| joint-23 | joint | 10 | 3 | 12 | false | 20 | 1.5 | 4.0 |
| joint-24 | joint | 21 | 5 | 12 | true | 20 | 2.0 | 4.0 |
| joint-25 | joint | 21 | 3 | 8 | false | 20 | 2.5 | 2.0 |
| joint-26 | joint | 21 | 7 | uncapped | false | 20 | 2.0 | 4.0 |
| joint-27 | joint | 10 | 3 | uncapped | true | 20 | 1.5 | 2.0 |
| joint-28 | joint | 10 | 7 | 8 | false | 20 | 1.5 | 4.0 |
| joint-29 | joint | 10 | 5 | uncapped | true | 20 | 1.5 | 2.0 |
| max_age=4 | ofat:max_age=4 | 14 | 5 | 4 | true | 20 | 1.5 | 2.0 |
| max_age=8 | ofat:max_age=8 | 14 | 5 | 8 | true | 20 | 1.5 | 2.0 |
| max_age=uncapped | ofat:max_age=uncapped | 14 | 5 | uncapped | true | 20 | 1.5 | 2.0 |
| rsi_order=3 | ofat:rsi_order=3 | 14 | 3 | 12 | true | 20 | 1.5 | 2.0 |
| rsi_order=7 | ofat:rsi_order=7 | 14 | 7 | 12 | true | 20 | 1.5 | 2.0 |
| rsi_period=10 | ofat:rsi_period=10 | 10 | 5 | 12 | true | 20 | 1.5 | 2.0 |
| rsi_period=21 | ofat:rsi_period=21 | 21 | 5 | 12 | true | 20 | 1.5 | 2.0 |
| sma_enabled=false | ofat:sma_enabled=false | 14 | 5 | 12 | false | 20 | 1.5 | 2.0 |

## 5. Selection

Selection uses validation only under the base cost scenario. Stress results are reported as a robustness check but never used for selection.

### 1hour

Selection: no candidate meets the criteria for this timeframe.

Notes:
- atr_sl=2.0: rejected: validation net expectancy -2.5571694707147252613604654389 is not positive
- atr_sl=2.5: rejected: validation net expectancy -2.9122325145967907251436111115 is not positive
- atr_tp=3.0: rejected: validation net expectancy -1.8736724239718110091809321424 is not positive
- atr_tp=4.0: rejected: validation net expectancy -2.0421589053294709102468788283 is not positive
- baseline: rejected: validation net expectancy -2.2756389093198688645788789298 is not positive
- joint-13: rejected: validation net expectancy -2.2608412480278265517104560648 is not positive
- joint-14: rejected: validation net expectancy -1.2229419628785363810836129575 is not positive
- joint-15: rejected: validation net expectancy -2.099122075553174588182925827 is not positive
- joint-16: rejected: validation net expectancy -0.8366349525103096680518275622 is not positive
- joint-17: rejected: validation net expectancy -1.7573439924048155417515780315 is not positive
- joint-18: rejected: validation net expectancy -1.9099164152233453044626640696 is not positive
- joint-19: rejected: validation net expectancy -1.5585864541621072536860518582 is not positive
- joint-20: rejected: validation net expectancy -1.8425156933767581300010080824 is not positive
- joint-21: rejected: validation net expectancy -1.8020775782815828282691382368 is not positive
- joint-22: rejected: validation net expectancy -1.5924156619619692833044697851 is not positive
- joint-23: rejected: validation net expectancy -0.730153378693618110237360129 is not positive
- joint-24: rejected: validation net expectancy -2.0574086300899814465309970586 is not positive
- joint-25: rejected: validation net expectancy -1.6489487798408433705644109776 is not positive
- joint-26: rejected: validation net expectancy -1.5732776723229107833005165151 is not positive
- joint-27: rejected: validation net expectancy -1.2968214858324318224822597012 is not positive
- joint-28: rejected: validation net expectancy -2.1722610851662954290438821699 is not positive
- joint-29: rejected: validation net expectancy -1.7969432995729604995905275241 is not positive
- max_age=4: rejected: validation net expectancy -2.1356910182530670175189803374 is not positive
- max_age=8: rejected: validation net expectancy -2.1473536482918505163116811815 is not positive
- max_age=uncapped: rejected: validation net expectancy -2.1070606508012960762470125873 is not positive
- no candidate meets the criteria
- rsi_order=3: rejected: validation net expectancy -1.5527603848318875667144086238 is not positive
- rsi_order=7: rejected: validation net expectancy -1.7568494869048482052144442439 is not positive
- rsi_period=10: rejected: validation net expectancy -1.9328187477392137292896010777 is not positive
- rsi_period=21: rejected: validation net expectancy -1.8260773514074847063532695215 is not positive
- sma_enabled=false: rejected: validation net expectancy -1.9430900671694931695905807145 is not positive

### 4hour

Winners (in rank order): `joint-13`, `joint-24`, `max_age=4`, `atr_tp=4.0`.

Notes:
- atr_sl=2.0: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- atr_sl=2.5: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- atr_tp=3.0: rejected: only 1 of 3 symbols have positive net expectancy (need 2)
- atr_tp=4.0: flagged: negative stress validation net expectancy (-1.1059984411069772516764508266)
- baseline: rejected: validation net expectancy -0.3297681368638819673672562759 is not positive
- joint-14: rejected: validation net expectancy -1.4024878266485900397421274542 is not positive
- joint-15: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- joint-16: rejected: only 0 of 3 folds have positive net expectancy (need 2)
- joint-17: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- joint-18: rejected: only 1 of 3 symbols have positive net expectancy (need 2)
- joint-19: rejected: validation net expectancy -2.9259282329677132788248018142 is not positive
- joint-20: rejected: validation net expectancy -0.6438729878973568798116604113 is not positive
- joint-21: rejected: validation net expectancy -2.8875149828828357161005336103 is not positive
- joint-22: rejected: validation net expectancy -3.4981756916738636717533105798 is not positive
- joint-23: rejected: validation net expectancy -0.033195853441758664913581889 is not positive
- joint-25: rejected: validation net expectancy -0.5193204832220861980952717938 is not positive
- joint-26: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- joint-27: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- joint-28: rejected: validation net expectancy -0.4011097580767227915791912797 is not positive
- joint-29: rejected: validation net expectancy -0.0528471070204967697173102466 is not positive
- max_age=8: rejected: only 0 of 3 folds have positive net expectancy (need 2)
- max_age=uncapped: rejected: validation net expectancy -0.5668860284850826311667773842 is not positive
- rsi_order=3: rejected: validation net expectancy -0.7737511434583780609421093854 is not positive
- rsi_order=7: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- rsi_period=10: rejected: only 0 of 3 folds have positive net expectancy (need 2)
- rsi_period=21: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- sma_enabled=false: rejected: validation net expectancy -1.3824124257297142964788717519 is not positive

## 6. Walk-forward

Expanding-fold validation metrics are pooled across symbols; per-symbol validation metrics are shown separately. Only the baseline and selected winners are shown.

### 1hour

#### Pooled folds

| Label | Fold | Net expectancy | Trades | Max DD % |
|---|---|---|---|---|
| baseline | fold1_val | -1.8925379999018226886038040938 | 300 | 29.551651652582331232956780090 |
| baseline | fold2_val | -1.7445700954130305436614866374 | 292 | 30.055910597500778247723475200 |
| baseline | fold3_val | -2.3354752385296287182718908675 | 174 | 16.364387815649111573050605930 |

#### Per-symbol validation

| Label | Symbol | Net expectancy | Trades | Max DD % |
|---|---|---|---|---|
| baseline | BTC-USDT | -1.6794388082174720800378164456 | 124 | 21.814956959751997262591650490 |
| baseline | ETH-USDT | -3.0514124536429869053152456636 | 116 | 41.238775049464617936561525180 |
| baseline | SOL-USDT | -2.1282910322117542641905597272 | 109 | 23.984154484199828087755216080 |

### 4hour

#### Pooled folds

| Label | Fold | Net expectancy | Trades | Max DD % |
|---|---|---|---|---|
| baseline | fold1_val | 1.7687643754482301274746918735 | 81 | 10.717922217609814304360708160 |
| baseline | fold2_val | -1.9179216352580324205013267861 | 71 | 14.526926509838916722396285900 |
| baseline | fold3_val | -1.6004697361397600138522910723 | 53 | 10.759918350491478834404959160 |
| joint-13 | fold1_val | 1.8090132424751863952753685042 | 55 | 22.408113805233686187095928720 |
| joint-13 | fold2_val | 4.6283011776702707499489480225 | 44 | 15.06638750877823722058798400 |
| joint-13 | fold3_val | 4.0541538913539012432378764938 | 37 | 13.508599716497664002638259530 |
| joint-24 | fold1_val | 2.245248797476758105920657272 | 46 | 17.649323242806632112471265460 |
| joint-24 | fold2_val | 1.650543655379553675897469607 | 45 | 12.259921851771915482378571430 |
| joint-24 | fold3_val | 4.4431431829520874704934223707 | 44 | 8.113462603412853321917218110 |
| max_age=4 | fold1_val | 1.0289038116924160208012861679 | 68 | 12.191074857325265651139089960 |
| max_age=4 | fold2_val | -0.8829521356009900740433539856 | 64 | 12.094427656259255058536751030 |
| max_age=4 | fold3_val | 0.6980141623406680643240445233 | 41 | 6.1782320299530953362356007900 |
| atr_tp=4.0 | fold1_val | 0.9989527949991999322104087985 | 61 | 14.418603043861516580361925060 |
| atr_tp=4.0 | fold2_val | 0.9902558219232403354222942726 | 60 | 10.315124221422693526958870830 |
| atr_tp=4.0 | fold3_val | -1.2795473668926705725773537276 | 47 | 11.887228724714917084933241430 |

#### Per-symbol validation

| Label | Symbol | Net expectancy | Trades | Max DD % |
|---|---|---|---|---|
| baseline | BTC-USDT | -2.7232281686158113173542414939 | 31 | 10.632388454443739842957857520 |
| baseline | ETH-USDT | 2.5501977675523131856069826968 | 38 | 10.759918350491478834404959160 |
| baseline | SOL-USDT | -1.2822395796613307771895411194 | 37 | 20.579867000202170273738499980 |
| joint-13 | BTC-USDT | -2.0358451696697733489714634408 | 13 | 11.452910310619487415087827920 |
| joint-13 | ETH-USDT | 4.9817641572518319293507746692 | 24 | 18.201787006085755708658271600 |
| joint-13 | SOL-USDT | 9.58481682094865769918678124 | 20 | 7.927979080772525946418389280 |
| joint-24 | BTC-USDT | 4.5873001985980185036020607688 | 25 | 7.6509259309681730175244191600 |
| joint-24 | ETH-USDT | 4.8776433678813306764841658661 | 28 | 12.144079760116165691164330510 |
| joint-24 | SOL-USDT | 4.297848807636182649730096645 | 20 | 13.942721226263134510032097470 |
| max_age=4 | BTC-USDT | -1.7613102985039565726212728731 | 28 | 8.852942817118416049489391130 |
| max_age=4 | ETH-USDT | 6.0763694131626354065645356897 | 29 | 6.1782320299530953362356007900 |
| max_age=4 | SOL-USDT | 0.3976907089812930806020064909 | 28 | 12.847498364384181139178986440 |
| atr_tp=4.0 | BTC-USDT | -3.4713277412314052140239639016 | 22 | 15.131267252058122244257701700 |
| atr_tp=4.0 | ETH-USDT | 1.8815447230608733256324961204 | 33 | 13.392821654669747941434064690 |
| atr_tp=4.0 | SOL-USDT | 1.4524973905702332812667068171 | 34 | 17.000516988007658469286741990 |

## 7. Parameter sensitivity

Each row groups candidates by a tested parameter value and reports the validation net expectancy range (pooled across symbols, base costs). Joint draws change several parameters at once, so these ranges are descriptive rather than causal.

### 1hour

| Parameter | Value | Candidates | Mean validation net expectancy | Min | Max |
|---|---|---|---|---|---|
| RSI period | 10 | 9 | -1.6536600295000717711489353431 | -2.2608412480278265517104560648 | -0.730153378693618110237360129 |
| RSI period | 14 | 15 | -2.0066181911978351211579028068 | -2.9122325145967907251436111115 | -1.5527603848318875667144086238 |
| RSI period | 21 | 6 | -1.7064284686271836653825785167 | -2.0574086300899814465309970586 | -1.2229419628785363810836129575 |
| RSI order | 3 | 7 | -1.2686680866499423146340499623 | -1.6489487798408433705644109776 | -0.730153378693618110237360129 |
| RSI order | 5 | 17 | -2.0948160136763150605945261567 | -2.9122325145967907251436111115 | -1.7969432995729604995905275241 |
| RSI order | 7 | 6 | -1.788039184364053752909856148 | -2.1722610851662954290438821699 | -1.5585864541621072536860518582 |
| Max age | 4 | 4 | -1.996811300601586260041394579 | -2.1356910182530670175189803374 | -1.8425156933767581300010080824 |
| Max age | 8 | 5 | -1.7897698937164682477510330636 | -2.1722610851662954290438821699 | -1.2229419628785363810836129575 |
| Max age | 12 | 14 | -1.8641959143972204429115995443 | -2.9122325145967907251436111115 | -0.730153378693618110237360129 |
| Max age | uncapped | 7 | -1.7408494960973574671887562907 | -2.2608412480278265517104560648 | -1.2968214858324318224822597012 |
| SMA enabled | false | 11 | -1.6180984908217329649301974416 | -2.1722610851662954290438821699 | -0.730153378693618110237360129 |
| SMA enabled | true | 19 | -1.9695631866416953755669610227 | -2.9122325145967907251436111115 | -1.2229419628785363810836129575 |
| ATR SL multiplier | 1.5 | 18 | -1.7849012918183920449512189199 | -2.2756389093198688645788789298 | -0.730153378693618110237360129 |
| ATR SL multiplier | 2.0 | 5 | -1.7868814281722544927412941288 | -2.5571694707147252613604654389 | -0.8366349525103096680518275622 |
| ATR SL multiplier | 2.5 | 7 | -2.0225933645198493538822885837 | -2.9122325145967907251436111115 | -1.5924156619619692833044697851 |
| ATR TP multiplier | 2.0 | 18 | -1.910257467445924675400866757 | -2.9122325145967907251436111115 | -0.8366349525103096680518275622 |
| ATR TP multiplier | 3.0 | 5 | -1.7554213297391981961270251876 | -1.9099164152233453044626640696 | -1.5585864541621072536860518582 |
| ATR TP multiplier | 4.0 | 7 | -1.7227204117869485160219576747 | -2.2608412480278265517104560648 | -0.730153378693618110237360129 |

### 4hour

| Parameter | Value | Candidates | Mean validation net expectancy | Min | Max |
|---|---|---|---|---|---|
| RSI period | 10 | 9 | 0.761105464200373314487094891 | -0.4011097580767227915791912797 | 4.9963629646896502938864068795 |
| RSI period | 14 | 15 | -0.6286951063531561201245281304 | -3.4981756916738636717533105798 | 1.6239219350009629295744950153 |
| RSI period | 21 | 6 | 1.7170084105429356210020007038 | -1.4024878266485900397421274542 | 6.2239702369496102901872541205 |
| RSI order | 3 | 7 | -0.7139255148328844545207617563 | -3.4981756916738636717533105798 | 1.2188128288184766504573215937 |
| RSI order | 5 | 17 | 0.4575893049904762065469265165 | -2.8875149828828357161005336103 | 4.9963629646896502938864068795 |
| RSI order | 7 | 6 | 0.8233389107926212374792529668 | -2.9259282329677132788248018142 | 6.2239702369496102901872541205 |
| Max age | 4 | 4 | 0.4701807908816350312757349562 | -0.6438729878973568798116604113 | 1.6239219350009629295744950153 |
| Max age | 8 | 5 | -0.2824385647216381819566610845 | -1.4024878266485900397421274542 | 0.8967935146880126886629378647 |
| Max age | 12 | 14 | 0.2189933570397978216609352639 | -2.8875149828828357161005336103 | 4.6193629509363201989891794023 |
| Max age | uncapped | 7 | 0.5981622438983018622793154556 | -3.4981756916738636717533105798 | 6.2239702369496102901872541205 |
| SMA enabled | false | 11 | -0.4358282797541978245533785706 | -3.4981756916738636717533105798 | 6.2239702369496102901872541205 |
| SMA enabled | true | 19 | 0.6587202170031477802429000297 | -1.4024878266485900397421274542 | 4.9963629646896502938864068795 |
| ATR SL multiplier | 1.5 | 18 | -0.2151069125226061435238862474 | -2.9259282329677132788248018142 | 1.6239219350009629295744950153 |
| ATR SL multiplier | 2.0 | 5 | 2.7315145264873337088099009404 | 0.7493886721371158159011275036 | 6.2239702369496102901872541205 |
| ATR SL multiplier | 2.5 | 7 | -0.2948678801808751722988022804 | -3.4981756916738636717533105798 | 4.9963629646896502938864068795 |
| ATR TP multiplier | 2.0 | 18 | -0.0286639938951848559824619459 | -2.8875149828828357161005336103 | 1.6239219350009629295744950153 |
| ATR TP multiplier | 3.0 | 5 | -1.2319669643204804442499089248 | -3.4981756916738636717533105798 | 0.8460379435951455885146220815 |
| ATR TP multiplier | 4.0 | 7 | 2.056765679639908769065970848 | -1.4024878266485900397421274542 | 6.2239702369496102901872541205 |

`SMA_PERIOD` was fixed at 20 for every candidate, so this study cannot rank 50/100/200.

## 8. Cost assumptions

- Base scenario: 10 bps fee + 5 bps slippage per side (15 bps per side in total).
- Stress scenario: 20 bps fee + 10 bps slippage per side (30 bps per side in total).
- Costs are applied once to each side of the actual simulated fill notional. Every reported metric is net of costs; results are gross of nothing.
- The simulator caps a position's gross loss at its notional `E`; a net loss may slightly exceed `E` because costs are real.

## 9. Command (synthesized from run configuration)

The actual process argv was not captured; this invocation is synthesized from the recorded run configuration.

```bash
make analyzer-sweep ARGS="--symbols BTC-USDT,ETH-USDT,SOL-USDT --timeframes 1hour,4hour --months 24 --as-of 1792661878840 --generated-at 2026-10-22T00:00:00Z --seed 20261022 --fee-bps 10 --slip-bps 5 --stress-fee-bps 20 --stress-slip-bps 10 --init-fund 1000"
cargo fmt -p analyzer-rs
cargo test -p analyzer-rs --features harness
cargo clippy -p analyzer-rs --features harness --all-targets --no-deps -- -D warnings
cargo build -p analyzer-rs --no-default-features
```

## 9b. Limitations

The neighbor-median-within-one-standard-error stability check from the study spec is approximated by fold and symbol consistency: only 30 of the 648 grid points are sampled, so a candidate's one-step grid neighbors are usually absent. No bootstrap confidence interval is reported; only the pooled standard error of validation net expectancy is shown. The single chronological 60/20/20 split is regime-sensitive, and the holdout is a one-shot evaluation rather than a rolling out-of-sample test.

## 10. Recommendation

**insufficient evidence**

No candidate met the validation criteria for every requested timeframe, so the screening produced no evidence strong enough to change the study baseline. This is a provisional screening result, not a profitability claim; a larger or differently scoped experiment would be needed before revising the study baseline. Selected candidate(s) with non-positive holdout net expectancy: 4hour/atr_tp=4.0 (-5.078658921718099057610802569), 4hour/joint-13 (-3.0424747826336476022237138843), 4hour/joint-24 (-3.9043150207849903147032745025), 4hour/max_age=4 (-0.814170014744723530881009295).

