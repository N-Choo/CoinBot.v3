# Analyzer Hyperparameter Screening Report

Provisional screening results. This report does not claim profitability.

## 1. Run metadata

| Field        | Value                                  |
| ------------ | -------------------------------------- |
| Generated at | 2026-10-23T00:00:00Z                   |
| As of (ms)   | 1792713600000                          |
| Months       | 34                                     |
| Symbols      | BTC-USDT, BNB-USDT, XRP-USDT, SOL-USDT |
| Timeframes   | 4hour, 1day                            |
| Seed         | 20261023                               |
| Base costs   | 10 bps fee + 5 bps slippage per side   |
| Stress costs | 20 bps fee + 10 bps slippage per side  |
| Init fund    | 1000                                   |

## 1b. Baseline configuration

Source: pinned per screening design decision 5; not read from .env (`baseline`: RSI 21/60:50).

| Parameter           | Values         | Role          |
| ------------------- | -------------- | ------------- |
| RSI period          | 14, 21         | varied        |
| RSI long threshold  | 55, 60, 65, 70 | varied        |
| RSI short threshold | 40, 45, 50     | varied        |
| Window              | 100            | held constant |

## 2. Data quality

| Symbol   | Timeframe | Requested range (ms)         | Effective range (ms)         | Candles | Requested | Expected | Missing | Duplicates | Pages | SHA-256                                                          |
| -------- | --------- | ---------------------------- | ---------------------------- | ------- | --------- | -------- | ------- | ---------- | ----- | ---------------------------------------------------------------- |
| BTC-USDT | 4hour     | 1704585600000..1792713600000 | 1778299200000..1792684800000 | 1000    | 6121      | 6119     | 0       | 0          | 5     | 02e46c9500896bf904f85fb3f942e286dd162d3e65c4b5a56138f0dbf25326f1 |
| BNB-USDT | 4hour     | 1704585600000..1792713600000 | 1778299200000..1792684800000 | 1000    | 6121      | 6119     | 0       | 0          | 5     | 6004958f6da16d4699ab59eee7404cb8b71df74fb351b66b3a3abfc600ebbc30 |
| XRP-USDT | 4hour     | 1704585600000..1792713600000 | 1778299200000..1792684800000 | 1000    | 6121      | 6119     | 0       | 0          | 5     | 4d33449a6d1dddb7a31e1401fb256d520e64094d4cc1c0dd2722f36afb465dc4 |
| SOL-USDT | 4hour     | 1704585600000..1792713600000 | 1778299200000..1792684800000 | 1000    | 6121      | 6119     | 0       | 0          | 5     | 04b4e2d7ca0182f1d8eafb5a7b70a9bb8e4f8bc7ab108c0d3f926d43bf9ea647 |
| BTC-USDT | 1day      | 1704585600000..1792713600000 | 1706227200000..1792540800000 | 1000    | 1021      | 1019     | 0       | 0          | 1     | 8c637b79ee316cee7f201e8dc7d6f7a9d342939ffa59c4597ccccde92df037a4 |
| BNB-USDT | 1day      | 1704585600000..1792713600000 | 1706227200000..1792540800000 | 1000    | 1021      | 1019     | 0       | 0          | 1     | d770e25d8bb1d7cfe3fb7d6066f34c25e8b4ee5a5938a7e919e9552be7390e6e |
| XRP-USDT | 1day      | 1704585600000..1792713600000 | 1706227200000..1792540800000 | 1000    | 1021      | 1019     | 0       | 0          | 1     | fc5180d415959abdb2e1fcf0f581912d2d2c0131fb72cc04af103114e31dcd5f |
| SOL-USDT | 1day      | 1704585600000..1792713600000 | 1706227200000..1792540800000 | 1000    | 1021      | 1019     | 0       | 0          | 1     | 3b705853dc26fe0cc4ded2fe534286b5c9dd30f862405a5300957fd77d7fdce4 |

### Errors

No errors recorded.

### Warnings

No warnings recorded.

## 2b. Chronological splits

Each split is chronological: 60% train, 20% validation, 20% holdout, plus three expanding folds inside train+validation. Indices are half-open (`start` inclusive, `end` exclusive); timestamps are the open times of the first and last candle in each range.

| Symbol   | Timeframe | Partition   | Start index | End index | Start                | End                  | Fold validation ends <= holdout start |
| -------- | --------- | ----------- | ----------- | --------- | -------------------- | -------------------- | ------------------------------------- |
| BTC-USDT | 4hour     | train       | 0           | 600       | 2026-05-09T04:00:00Z | 2026-08-17T00:00:00Z | —                                     |
| BTC-USDT | 4hour     | validation  | 600         | 800       | 2026-08-17T04:00:00Z | 2026-09-19T08:00:00Z | —                                     |
| BTC-USDT | 4hour     | holdout     | 800         | 1000      | 2026-09-19T12:00:00Z | 2026-10-22T16:00:00Z | —                                     |
| BTC-USDT | 4hour     | fold1_train | 0           | 400       | 2026-05-09T04:00:00Z | 2026-07-14T16:00:00Z | —                                     |
| BTC-USDT | 4hour     | fold1_val   | 400         | 550       | 2026-07-14T20:00:00Z | 2026-08-08T16:00:00Z | yes                                   |
| BTC-USDT | 4hour     | fold2_train | 0           | 550       | 2026-05-09T04:00:00Z | 2026-08-08T16:00:00Z | —                                     |
| BTC-USDT | 4hour     | fold2_val   | 550         | 700       | 2026-08-08T20:00:00Z | 2026-09-02T16:00:00Z | yes                                   |
| BTC-USDT | 4hour     | fold3_train | 0           | 700       | 2026-05-09T04:00:00Z | 2026-09-02T16:00:00Z | —                                     |
| BTC-USDT | 4hour     | fold3_val   | 700         | 800       | 2026-09-02T20:00:00Z | 2026-09-19T08:00:00Z | yes                                   |
| BNB-USDT | 4hour     | train       | 0           | 600       | 2026-05-09T04:00:00Z | 2026-08-17T00:00:00Z | —                                     |
| BNB-USDT | 4hour     | validation  | 600         | 800       | 2026-08-17T04:00:00Z | 2026-09-19T08:00:00Z | —                                     |
| BNB-USDT | 4hour     | holdout     | 800         | 1000      | 2026-09-19T12:00:00Z | 2026-10-22T16:00:00Z | —                                     |
| BNB-USDT | 4hour     | fold1_train | 0           | 400       | 2026-05-09T04:00:00Z | 2026-07-14T16:00:00Z | —                                     |
| BNB-USDT | 4hour     | fold1_val   | 400         | 550       | 2026-07-14T20:00:00Z | 2026-08-08T16:00:00Z | yes                                   |
| BNB-USDT | 4hour     | fold2_train | 0           | 550       | 2026-05-09T04:00:00Z | 2026-08-08T16:00:00Z | —                                     |
| BNB-USDT | 4hour     | fold2_val   | 550         | 700       | 2026-08-08T20:00:00Z | 2026-09-02T16:00:00Z | yes                                   |
| BNB-USDT | 4hour     | fold3_train | 0           | 700       | 2026-05-09T04:00:00Z | 2026-09-02T16:00:00Z | —                                     |
| BNB-USDT | 4hour     | fold3_val   | 700         | 800       | 2026-09-02T20:00:00Z | 2026-09-19T08:00:00Z | yes                                   |
| XRP-USDT | 4hour     | train       | 0           | 600       | 2026-05-09T04:00:00Z | 2026-08-17T00:00:00Z | —                                     |
| XRP-USDT | 4hour     | validation  | 600         | 800       | 2026-08-17T04:00:00Z | 2026-09-19T08:00:00Z | —                                     |
| XRP-USDT | 4hour     | holdout     | 800         | 1000      | 2026-09-19T12:00:00Z | 2026-10-22T16:00:00Z | —                                     |
| XRP-USDT | 4hour     | fold1_train | 0           | 400       | 2026-05-09T04:00:00Z | 2026-07-14T16:00:00Z | —                                     |
| XRP-USDT | 4hour     | fold1_val   | 400         | 550       | 2026-07-14T20:00:00Z | 2026-08-08T16:00:00Z | yes                                   |
| XRP-USDT | 4hour     | fold2_train | 0           | 550       | 2026-05-09T04:00:00Z | 2026-08-08T16:00:00Z | —                                     |
| XRP-USDT | 4hour     | fold2_val   | 550         | 700       | 2026-08-08T20:00:00Z | 2026-09-02T16:00:00Z | yes                                   |
| XRP-USDT | 4hour     | fold3_train | 0           | 700       | 2026-05-09T04:00:00Z | 2026-09-02T16:00:00Z | —                                     |
| XRP-USDT | 4hour     | fold3_val   | 700         | 800       | 2026-09-02T20:00:00Z | 2026-09-19T08:00:00Z | yes                                   |
| SOL-USDT | 4hour     | train       | 0           | 600       | 2026-05-09T04:00:00Z | 2026-08-17T00:00:00Z | —                                     |
| SOL-USDT | 4hour     | validation  | 600         | 800       | 2026-08-17T04:00:00Z | 2026-09-19T08:00:00Z | —                                     |
| SOL-USDT | 4hour     | holdout     | 800         | 1000      | 2026-09-19T12:00:00Z | 2026-10-22T16:00:00Z | —                                     |
| SOL-USDT | 4hour     | fold1_train | 0           | 400       | 2026-05-09T04:00:00Z | 2026-07-14T16:00:00Z | —                                     |
| SOL-USDT | 4hour     | fold1_val   | 400         | 550       | 2026-07-14T20:00:00Z | 2026-08-08T16:00:00Z | yes                                   |
| SOL-USDT | 4hour     | fold2_train | 0           | 550       | 2026-05-09T04:00:00Z | 2026-08-08T16:00:00Z | —                                     |
| SOL-USDT | 4hour     | fold2_val   | 550         | 700       | 2026-08-08T20:00:00Z | 2026-09-02T16:00:00Z | yes                                   |
| SOL-USDT | 4hour     | fold3_train | 0           | 700       | 2026-05-09T04:00:00Z | 2026-09-02T16:00:00Z | —                                     |
| SOL-USDT | 4hour     | fold3_val   | 700         | 800       | 2026-09-02T20:00:00Z | 2026-09-19T08:00:00Z | yes                                   |
| BTC-USDT | 1day      | train       | 0           | 600       | 2024-01-26T00:00:00Z | 2025-09-16T00:00:00Z | —                                     |
| BTC-USDT | 1day      | validation  | 600         | 800       | 2025-09-17T00:00:00Z | 2026-04-04T00:00:00Z | —                                     |
| BTC-USDT | 1day      | holdout     | 800         | 1000      | 2026-04-05T00:00:00Z | 2026-10-21T00:00:00Z | —                                     |
| BTC-USDT | 1day      | fold1_train | 0           | 400       | 2024-01-26T00:00:00Z | 2025-02-28T00:00:00Z | —                                     |
| BTC-USDT | 1day      | fold1_val   | 400         | 550       | 2025-03-01T00:00:00Z | 2025-07-28T00:00:00Z | yes                                   |
| BTC-USDT | 1day      | fold2_train | 0           | 550       | 2024-01-26T00:00:00Z | 2025-07-28T00:00:00Z | —                                     |
| BTC-USDT | 1day      | fold2_val   | 550         | 700       | 2025-07-29T00:00:00Z | 2025-12-25T00:00:00Z | yes                                   |
| BTC-USDT | 1day      | fold3_train | 0           | 700       | 2024-01-26T00:00:00Z | 2025-12-25T00:00:00Z | —                                     |
| BTC-USDT | 1day      | fold3_val   | 700         | 800       | 2025-12-26T00:00:00Z | 2026-04-04T00:00:00Z | yes                                   |
| BNB-USDT | 1day      | train       | 0           | 600       | 2024-01-26T00:00:00Z | 2025-09-16T00:00:00Z | —                                     |
| BNB-USDT | 1day      | validation  | 600         | 800       | 2025-09-17T00:00:00Z | 2026-04-04T00:00:00Z | —                                     |
| BNB-USDT | 1day      | holdout     | 800         | 1000      | 2026-04-05T00:00:00Z | 2026-10-21T00:00:00Z | —                                     |
| BNB-USDT | 1day      | fold1_train | 0           | 400       | 2024-01-26T00:00:00Z | 2025-02-28T00:00:00Z | —                                     |
| BNB-USDT | 1day      | fold1_val   | 400         | 550       | 2025-03-01T00:00:00Z | 2025-07-28T00:00:00Z | yes                                   |
| BNB-USDT | 1day      | fold2_train | 0           | 550       | 2024-01-26T00:00:00Z | 2025-07-28T00:00:00Z | —                                     |
| BNB-USDT | 1day      | fold2_val   | 550         | 700       | 2025-07-29T00:00:00Z | 2025-12-25T00:00:00Z | yes                                   |
| BNB-USDT | 1day      | fold3_train | 0           | 700       | 2024-01-26T00:00:00Z | 2025-12-25T00:00:00Z | —                                     |
| BNB-USDT | 1day      | fold3_val   | 700         | 800       | 2025-12-26T00:00:00Z | 2026-04-04T00:00:00Z | yes                                   |
| XRP-USDT | 1day      | train       | 0           | 600       | 2024-01-26T00:00:00Z | 2025-09-16T00:00:00Z | —                                     |
| XRP-USDT | 1day      | validation  | 600         | 800       | 2025-09-17T00:00:00Z | 2026-04-04T00:00:00Z | —                                     |
| XRP-USDT | 1day      | holdout     | 800         | 1000      | 2026-04-05T00:00:00Z | 2026-10-21T00:00:00Z | —                                     |
| XRP-USDT | 1day      | fold1_train | 0           | 400       | 2024-01-26T00:00:00Z | 2025-02-28T00:00:00Z | —                                     |
| XRP-USDT | 1day      | fold1_val   | 400         | 550       | 2025-03-01T00:00:00Z | 2025-07-28T00:00:00Z | yes                                   |
| XRP-USDT | 1day      | fold2_train | 0           | 550       | 2024-01-26T00:00:00Z | 2025-07-28T00:00:00Z | —                                     |
| XRP-USDT | 1day      | fold2_val   | 550         | 700       | 2025-07-29T00:00:00Z | 2025-12-25T00:00:00Z | yes                                   |
| XRP-USDT | 1day      | fold3_train | 0           | 700       | 2024-01-26T00:00:00Z | 2025-12-25T00:00:00Z | —                                     |
| XRP-USDT | 1day      | fold3_val   | 700         | 800       | 2025-12-26T00:00:00Z | 2026-04-04T00:00:00Z | yes                                   |
| SOL-USDT | 1day      | train       | 0           | 600       | 2024-01-26T00:00:00Z | 2025-09-16T00:00:00Z | —                                     |
| SOL-USDT | 1day      | validation  | 600         | 800       | 2025-09-17T00:00:00Z | 2026-04-04T00:00:00Z | —                                     |
| SOL-USDT | 1day      | holdout     | 800         | 1000      | 2026-04-05T00:00:00Z | 2026-10-21T00:00:00Z | —                                     |
| SOL-USDT | 1day      | fold1_train | 0           | 400       | 2024-01-26T00:00:00Z | 2025-02-28T00:00:00Z | —                                     |
| SOL-USDT | 1day      | fold1_val   | 400         | 550       | 2025-03-01T00:00:00Z | 2025-07-28T00:00:00Z | yes                                   |
| SOL-USDT | 1day      | fold2_train | 0           | 550       | 2024-01-26T00:00:00Z | 2025-07-28T00:00:00Z | —                                     |
| SOL-USDT | 1day      | fold2_val   | 550         | 700       | 2025-07-29T00:00:00Z | 2025-12-25T00:00:00Z | yes                                   |
| SOL-USDT | 1day      | fold3_train | 0           | 700       | 2024-01-26T00:00:00Z | 2025-12-25T00:00:00Z | —                                     |
| SOL-USDT | 1day      | fold3_val   | 700         | 800       | 2025-12-26T00:00:00Z | 2026-04-04T00:00:00Z | yes                                   |

Every `fold*_val` row above ends at or before the holdout start index, so no fold validation overlaps the holdout.

## 3. Baseline vs candidates

Validation is the selection partition; holdout is the one-shot evaluation partition, scored only for the baseline and selected winners, and is never used for selection. Metrics are pooled across symbols under base costs. A missing holdout is shown as —.

Footnote: _pooled max DD_ is the maximum per-symbol drawdown, not a portfolio drawdown.

### 4hour

| Label                | Config       | Val trades | Val net expectancy              | Val PF                         | Val max DD %                   | Val stderr        | Holdout trades | Holdout net expectancy        | Holdout PF                     | Holdout max DD %               | Status       |
| -------------------- | ------------ | ---------- | ------------------------------- | ------------------------------ | ------------------------------ | ----------------- | -------------- | ----------------------------- | ------------------------------ | ------------------------------ | ------------ |
| baseline             | RSI 21/60:50 | 22         | 3.4116100030678284210661605195  | 1.1927367497192062596292396816 | 16.525144415930723204791623600 | 12.90026036998438 | 15             | -6.08434566420156395269329888 | 0.5151412550990901091238190428 | 7.7282666015436221851287534300 | not selected |
| rsi14-long55-short40 | RSI 14/55:40 | 26         | 1.3224191246650607879823305598  | 1.0796415116856449528562496854 | 13.796690291714156996147538660 | 10.83613417801533 | —              | —                             | —                              | —                              | not selected |
| rsi14-long55-short45 | RSI 14/55:45 | 34         | 0.8294740085380326582036062012  | 1.0611080642072909748608334    | 19.689768087030641521917433820 | 8.32913325197479  | —              | —                             | —                              | —                              | not selected |
| rsi14-long55-short50 | RSI 14/55:50 | 42         | 2.9090011253023293949724158614  | 1.2889610519675126328946619711 | 15.617910051278871796542504490 | 7.13368269240908  | —              | —                             | —                              | —                              | not selected |
| rsi14-long60-short40 | RSI 14/60:40 | 22         | 2.6970274008152052908816717348  | 1.15565996261723743097810172   | 14.606806332400809474118900120 | 12.38789265810405 | —              | —                             | —                              | —                              | not selected |
| rsi14-long60-short45 | RSI 14/60:45 | 24         | 4.31158300442756910697739812    | 1.2870636120801642717247243201 | 16.263717131820461198182370070 | 11.50428315100259 | —              | —                             | —                              | —                              | not selected |
| rsi14-long60-short50 | RSI 14/60:50 | 29         | 5.2014453642905989267151287648  | 1.4099563820494389672200465188 | 12.478087432606737190818396530 | 10.24306525296989 | —              | —                             | —                              | —                              | not selected |
| rsi14-long65-short40 | RSI 14/65:40 | 22         | -1.3818119565834074921706458864 | 0.9350537956790805696960633091 | 19.275164906466278932040318650 | 12.72673045226678 | —              | —                             | —                              | —                              | not selected |
| rsi14-long65-short45 | RSI 14/65:45 | 22         | 2.0505361855538442251057126682  | 1.1084609537559444265924853295 | 18.051056992252450113757024510 | 12.99317463712069 | —              | —                             | —                              | —                              | not selected |
| rsi14-long65-short50 | RSI 14/65:50 | 23         | 6.8290813654727943709281170287  | 1.4445052767131943175606560813 | 14.656645127374184741003060580 | 13.19886637204803 | —              | —                             | —                              | —                              | not selected |
| rsi14-long70-short40 | RSI 14/70:40 | 17         | 0.3875673420815930489224552417  | 1.0173257489358270337679915824 | 20.091144692529294797408592870 | 15.09210383601596 | —              | —                             | —                              | —                              | not selected |
| rsi14-long70-short45 | RSI 14/70:45 | 17         | 3.8927514130682818819495661612  | 1.1926930013387520565116162353 | 18.857996161078242605053894540 | 15.33428176614403 | —              | —                             | —                              | —                              | not selected |
| rsi14-long70-short50 | RSI 14/70:50 | 18         | 7.2856215107735568678897597111  | 1.4081527287330841436918863556 | 15.443240439168657566849620750 | 16.12268908202407 | —              | —                             | —                              | —                              | not selected |
| rsi21-long55-short40 | RSI 21/55:40 | 17         | 4.5476811735920372667418769065  | 1.2116142108930042807067140345 | 17.077558112518753121107433700 | 16.42094395437182 | —              | —                             | —                              | —                              | not selected |
| rsi21-long55-short45 | RSI 21/55:45 | 26         | 0.4291780673775512420854536315  | 1.02544909726282528499374939   | 17.408140740487530153717300430 | 10.54575377197667 | —              | —                             | —                              | —                              | not selected |
| rsi21-long55-short50 | RSI 21/55:50 | 32         | 1.6625126888395768966992694725  | 1.1235762654120705979837846467 | 19.75554516738486963845234900  | 8.6424671874318   | —              | —                             | —                              | —                              | not selected |
| rsi21-long60-short40 | RSI 21/60:40 | 13         | 5.6969316305901952923532804562  | 1.2051141566790802479744422338 | 22.128682812289547679686938190 | 20.94726113547208 | —              | —                             | —                              | —                              | not selected |
| rsi21-long60-short45 | RSI 21/60:45 | 20         | 0.173429938064800172183521723   | 1.0080889650741434718961660056 | 18.719721534391987425392061970 | 13.90766086968967 | —              | —                             | —                              | —                              | not selected |
| rsi21-long65-short40 | RSI 21/65:40 | 12         | 2.8441644477334188519403031117  | 1.0979587460293209866455505449 | 22.956156668634516112461228620 | 22.45935727887867 | —              | —                             | —                              | —                              | not selected |
| rsi21-long65-short45 | RSI 21/65:45 | 17         | -1.8059374851322044171091646332 | 0.9241074862916199024248200701 | 19.531613426098303323630171460 | 15.42168857314334 | —              | —                             | —                              | —                              | not selected |
| rsi21-long65-short50 | RSI 21/65:50 | 17         | 2.3853955089098486814407782684  | 1.1126950828888485860752073046 | 17.307139531603688960474938250 | 16.02574348241264 | —              | —                             | —                              | —                              | not selected |
| rsi21-long70-short40 | RSI 21/70:40 | 7          | 32.446256719092216220428964519  | 2.839910127952109542781733717  | 5.3394619686167941091263103700 | 40.97318959378863 | —              | —                             | —                              | —                              | not selected |
| rsi21-long70-short45 | RSI 21/70:45 | 12         | 9.489500696543223737927298287   | 1.4991132630042854130282153351 | 7.3800204513487721696112713400 | 19.56876207748544 | —              | —                             | —                              | —                              | not selected |
| rsi21-long70-short50 | RSI 21/70:50 | 12         | 13.096843994774996415645494587  | 1.7446115905500430704493269881 | 5.692800480971081994604661200  | 17.2877557915485  | —              | —                             | —                              | —                              | not selected |

### 1day

| Label                | Config       | Val trades | Val net expectancy              | Val PF                         | Val max DD %                   | Val stderr        | Holdout trades | Holdout net expectancy          | Holdout PF                   | Holdout max DD %               | Status       |
| -------------------- | ------------ | ---------- | ------------------------------- | ------------------------------ | ------------------------------ | ----------------- | -------------- | ------------------------------- | ---------------------------- | ------------------------------ | ------------ |
| baseline             | RSI 21/60:50 | 9          | 2.5472226546202620322261888738  | 1.1014559165758931765997730305 | 7.0159901762649063529564266900 | 23.26339971139709 | 16             | -3.8082218882598542869738490718 | 0.85564631393535714856167586 | 17.345231497570099272972425160 | not selected |
| rsi14-long55-short40 | RSI 14/55:40 | 23         | 1.8208921697653091529671552713  | 1.0562340693458974158945654256 | 21.068680212826792307405427580 | 17.35786104757396 | —              | —                               | —                            | —                              | not selected |
| rsi14-long55-short45 | RSI 14/55:45 | 26         | 10.823443496435544289949294822  | 1.459247411465486955816557936  | 15.200819995083404602993909980 | 15.14862819411549 | —              | —                               | —                            | —                              | not selected |
| rsi14-long55-short50 | RSI 14/55:50 | 26         | 25.15701135129127003981358967   | 2.6972091569435197481334466517 | 8.064710379657172780571508280  | 14.70513256066279 | —              | —                               | —                            | —                              | not selected |
| rsi14-long60-short40 | RSI 14/60:40 | 17         | 13.553734687257987524181995333  | 1.5282203751177919102268431507 | 11.638544543193152251693311070 | 19.76743288064924 | —              | —                               | —                            | —                              | not selected |
| rsi14-long60-short45 | RSI 14/60:45 | 18         | 19.496159454323929949963712517  | 1.9198371139734474532523383256 | 7.4154030229967623692937909700 | 18.85549483564358 | —              | —                               | —                            | —                              | not selected |
| rsi14-long60-short50 | RSI 14/60:50 | 18         | 30.582025347807484420879154902  | 2.9500600268153354804072042128 | 7.4154030229967623692937909700 | 18.47826293381374 | —              | —                               | —                            | —                              | not selected |
| rsi14-long65-short40 | RSI 14/65:40 | 11         | -2.1105073379507682314262492822 | 0.9311332910521943941669010021 | 9.691134778428669248591804520  | 24.71086541854685 | —              | —                               | —                            | —                              | not selected |
| rsi14-long65-short45 | RSI 14/65:45 | 11         | 7.3637514134415880713903280818  | 1.2891052367568951379227461713 | 7.2133482454412401579707569900 | 25.59235071077072 | —              | —                               | —                            | —                              | not selected |
| rsi14-long65-short50 | RSI 14/65:50 | 11         | 19.018261604903407676325781204  | 1.8731551132050720836763523748 | 7.0159901762649063529564266900 | 26.06141922240727 | —              | —                               | —                            | —                              | not selected |
| rsi14-long70-short40 | RSI 14/70:40 | 7          | -29.386181806991472715735428859 | 0.4110371502529996648147816012 | 12.146279291534285167420881290 | 33.49229926067797 | —              | —                               | —                            | —                              | not selected |
| rsi14-long70-short45 | RSI 14/70:45 | 7          | -20.858269989697406817322730674 | 0.5162861895987635800134479949 | 9.447877754609563800565671760  | 31.41891386952431 | —              | —                               | —                            | —                              | not selected |
| rsi14-long70-short50 | RSI 14/70:50 | 7          | -13.641419166356024734371005223 | 0.644140450874291297236072393  | 9.447877754609563800565671760  | 33.20271976065007 | —              | —                               | —                            | —                              | not selected |
| rsi21-long55-short40 | RSI 21/55:40 | 12         | -24.394427924362560427896784721 | 0.4413612936301413649194372273 | 13.132201030460418233806651870 | 23.50612533656957 | —              | —                               | —                            | —                              | not selected |
| rsi21-long55-short45 | RSI 21/55:45 | 15         | -9.140112223822193155395569418  | 0.7508527681479191118295106805 | 21.068680212826792307405427580 | 20.92783765693146 | —              | —                               | —                            | —                              | not selected |
| rsi21-long55-short50 | RSI 21/55:50 | 17         | 4.5236795424724510621229184378  | 1.175613197503566991895379962  | 15.200819995083404602993909980 | 17.29790170770924 | —              | —                               | —                            | —                              | not selected |
| rsi21-long60-short40 | RSI 21/60:40 | 8          | -43.624705743293725927469933391 | 0.1170410159084255159923076177 | 13.892230309294978411391364860 | 22.30095749343925 | —              | —                               | —                            | —                              | not selected |
| rsi21-long60-short45 | RSI 21/60:45 | 9          | -15.054273393663004130265449101 | 0.5525512665102605488556428536 | 9.691134778428669248591804520  | 22.62191286397608 | —              | —                               | —                            | —                              | not selected |
| rsi21-long65-short40 | RSI 21/65:40 | 3          | -83.1326688389265041328509176   | 0                              | 13.857660499877481009556481250 | 0                 | —              | —                               | —                            | —                              | not selected |
| rsi21-long65-short45 | RSI 21/65:45 | 4          | -60.416843703045957927043560255 | 0                              | 12.146279291534285167420881290 | 7.3400748748007   | —              | —                               | —                            | —                              | not selected |
| rsi21-long65-short50 | RSI 21/65:50 | 4          | -33.715294202469818498610221822 | 0.159158087848282893736744785  | 9.447877754609563800565671760  | 2.37762130425619  | —              | —                               | —                            | —                              | not selected |
| rsi21-long70-short40 | RSI 21/70:40 | 1          | -28.284722362499293003563262056 | 0                              | 2.8284722362499293003563262100 | 0                 | —              | —                               | —                            | —                              | not selected |
| rsi21-long70-short45 | RSI 21/70:45 | 1          | 0.4282136018034469106275704267  | -                              | 0                              | 0                 | —              | —                               | —                            | —                              | not selected |
| rsi21-long70-short50 | RSI 21/70:50 | 1          | 39.414030243287573830627893650  | -                              | 0                              | 0                 | —              | —                               | —                            | —                              | not selected |

## 4. Exact configurations tested and reasons

The `origin` column records how each candidate entered the study: `baseline` for the pinned profile, or `grid` for the full 24-point RSI grid.

### 4hour

| Label                | Origin   | RSI period | Long threshold | Short threshold |
| -------------------- | -------- | ---------- | -------------- | --------------- |
| baseline             | baseline | 21         | 60             | 50              |
| rsi14-long55-short40 | grid     | 14         | 55             | 40              |
| rsi14-long55-short45 | grid     | 14         | 55             | 45              |
| rsi14-long55-short50 | grid     | 14         | 55             | 50              |
| rsi14-long60-short40 | grid     | 14         | 60             | 40              |
| rsi14-long60-short45 | grid     | 14         | 60             | 45              |
| rsi14-long60-short50 | grid     | 14         | 60             | 50              |
| rsi14-long65-short40 | grid     | 14         | 65             | 40              |
| rsi14-long65-short45 | grid     | 14         | 65             | 45              |
| rsi14-long65-short50 | grid     | 14         | 65             | 50              |
| rsi14-long70-short40 | grid     | 14         | 70             | 40              |
| rsi14-long70-short45 | grid     | 14         | 70             | 45              |
| rsi14-long70-short50 | grid     | 14         | 70             | 50              |
| rsi21-long55-short40 | grid     | 21         | 55             | 40              |
| rsi21-long55-short45 | grid     | 21         | 55             | 45              |
| rsi21-long55-short50 | grid     | 21         | 55             | 50              |
| rsi21-long60-short40 | grid     | 21         | 60             | 40              |
| rsi21-long60-short45 | grid     | 21         | 60             | 45              |
| rsi21-long65-short40 | grid     | 21         | 65             | 40              |
| rsi21-long65-short45 | grid     | 21         | 65             | 45              |
| rsi21-long65-short50 | grid     | 21         | 65             | 50              |
| rsi21-long70-short40 | grid     | 21         | 70             | 40              |
| rsi21-long70-short45 | grid     | 21         | 70             | 45              |
| rsi21-long70-short50 | grid     | 21         | 70             | 50              |

### 1day

| Label                | Origin   | RSI period | Long threshold | Short threshold |
| -------------------- | -------- | ---------- | -------------- | --------------- |
| baseline             | baseline | 21         | 60             | 50              |
| rsi14-long55-short40 | grid     | 14         | 55             | 40              |
| rsi14-long55-short45 | grid     | 14         | 55             | 45              |
| rsi14-long55-short50 | grid     | 14         | 55             | 50              |
| rsi14-long60-short40 | grid     | 14         | 60             | 40              |
| rsi14-long60-short45 | grid     | 14         | 60             | 45              |
| rsi14-long60-short50 | grid     | 14         | 60             | 50              |
| rsi14-long65-short40 | grid     | 14         | 65             | 40              |
| rsi14-long65-short45 | grid     | 14         | 65             | 45              |
| rsi14-long65-short50 | grid     | 14         | 65             | 50              |
| rsi14-long70-short40 | grid     | 14         | 70             | 40              |
| rsi14-long70-short45 | grid     | 14         | 70             | 45              |
| rsi14-long70-short50 | grid     | 14         | 70             | 50              |
| rsi21-long55-short40 | grid     | 21         | 55             | 40              |
| rsi21-long55-short45 | grid     | 21         | 55             | 45              |
| rsi21-long55-short50 | grid     | 21         | 55             | 50              |
| rsi21-long60-short40 | grid     | 21         | 60             | 40              |
| rsi21-long60-short45 | grid     | 21         | 60             | 45              |
| rsi21-long65-short40 | grid     | 21         | 65             | 40              |
| rsi21-long65-short45 | grid     | 21         | 65             | 45              |
| rsi21-long65-short50 | grid     | 21         | 65             | 50              |
| rsi21-long70-short40 | grid     | 21         | 70             | 40              |
| rsi21-long70-short45 | grid     | 21         | 70             | 45              |
| rsi21-long70-short50 | grid     | 21         | 70             | 50              |

## 5. Selection

Selection uses validation only under the base cost scenario. Stress results are reported as a robustness check but never used for selection.

### 4hour

Selection: no candidate meets the criteria for this timeframe.

Notes:

- baseline: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- no candidate meets the criteria
- rsi14-long55-short40: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- rsi14-long55-short45: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- rsi14-long55-short50: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- rsi14-long60-short40: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- rsi14-long60-short45: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- rsi14-long60-short50: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- rsi14-long65-short40: rejected: validation net expectancy -1.3818119565834074921706458864 is not positive
- rsi14-long65-short45: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- rsi14-long65-short50: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- rsi14-long70-short40: rejected: XRP-USDT max drawdown 20.091144692529294797408592870% exceeds 20%
- rsi14-long70-short45: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- rsi14-long70-short50: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- rsi21-long55-short40: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- rsi21-long55-short45: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- rsi21-long55-short50: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- rsi21-long60-short40: rejected: XRP-USDT max drawdown 22.128682812289547679686938190% exceeds 20%
- rsi21-long60-short45: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- rsi21-long65-short40: rejected: XRP-USDT max drawdown 22.956156668634516112461228620% exceeds 20%
- rsi21-long65-short45: rejected: validation net expectancy -1.8059374851322044171091646332 is not positive
- rsi21-long65-short50: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- rsi21-long70-short40: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- rsi21-long70-short45: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- rsi21-long70-short50: rejected: only 1 of 3 folds have positive net expectancy (need 2)

### 1day

Selection: no candidate meets the criteria for this timeframe.

Notes:

- baseline: rejected: only 0 of 3 folds have positive net expectancy (need 2)
- no candidate meets the criteria
- rsi14-long55-short40: rejected: SOL-USDT max drawdown 21.068680212826792307405427580% exceeds 20%
- rsi14-long55-short45: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- rsi14-long55-short50: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- rsi14-long60-short40: rejected: only 0 of 3 folds have positive net expectancy (need 2)
- rsi14-long60-short45: rejected: only 0 of 3 folds have positive net expectancy (need 2)
- rsi14-long60-short50: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- rsi14-long65-short40: rejected: validation net expectancy -2.1105073379507682314262492822 is not positive
- rsi14-long65-short45: rejected: only 0 of 3 folds have positive net expectancy (need 2)
- rsi14-long65-short50: rejected: only 0 of 3 folds have positive net expectancy (need 2)
- rsi14-long70-short40: rejected: validation net expectancy -29.386181806991472715735428859 is not positive
- rsi14-long70-short45: rejected: validation net expectancy -20.858269989697406817322730674 is not positive
- rsi14-long70-short50: rejected: validation net expectancy -13.641419166356024734371005223 is not positive
- rsi21-long55-short40: rejected: validation net expectancy -24.394427924362560427896784721 is not positive
- rsi21-long55-short45: rejected: validation net expectancy -9.140112223822193155395569418 is not positive
- rsi21-long55-short50: rejected: only 0 of 3 folds have positive net expectancy (need 2)
- rsi21-long60-short40: rejected: validation net expectancy -43.624705743293725927469933391 is not positive
- rsi21-long60-short45: rejected: validation net expectancy -15.054273393663004130265449101 is not positive
- rsi21-long65-short40: rejected: validation net expectancy -83.1326688389265041328509176 is not positive
- rsi21-long65-short45: rejected: validation net expectancy -60.416843703045957927043560255 is not positive
- rsi21-long65-short50: rejected: validation net expectancy -33.715294202469818498610221822 is not positive
- rsi21-long70-short40: rejected: validation net expectancy -28.284722362499293003563262056 is not positive
- rsi21-long70-short45: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- rsi21-long70-short50: rejected: only 1 of 3 folds have positive net expectancy (need 2)

## 6. Walk-forward

Expanding-fold validation metrics are pooled across symbols; per-symbol validation metrics are shown separately. Only the baseline and selected winners are shown.

### 4hour

#### Pooled folds

| Label    | Fold      | Net expectancy                  | Trades | Max DD %                       |
| -------- | --------- | ------------------------------- | ------ | ------------------------------ |
| baseline | fold1_val | -10.965523029069881347770615541 | 14     | 8.119622025270469685322545670  |
| baseline | fold2_val | 39.740116233292498962486986025  | 10     | 2.9510355374194547949266970400 |
| baseline | fold3_val | -20.334698240231832349198664731 | 12     | 15.334864629176628642697099750 |

#### Per-symbol validation

| Label    | Symbol   | Net expectancy                  | Trades | Max DD %                       |
| -------- | -------- | ------------------------------- | ------ | ------------------------------ |
| baseline | BNB-USDT | 6.7235743746091140467198402478  | 5      | 1.7257830919712078472836469500 |
| baseline | BTC-USDT | 11.944978101920416745168630054  | 4      | 5.0414175842818199317490523500 |
| baseline | SOL-USDT | 5.8784034362387749135805411916  | 7      | 8.166561148819329508249738460  |
| baseline | XRP-USDT | -7.9151980444844060576469963952 | 6      | 16.525144415930723204791623600 |

### 1day

#### Pooled folds

| Label    | Fold      | Net expectancy                  | Trades | Max DD %                       |
| -------- | --------- | ------------------------------- | ------ | ------------------------------ |
| baseline | fold1_val | -32.576738883617391224504061632 | 13     | 13.561592652652886113613942950 |
| baseline | fold2_val | -35.073720399121195735994156844 | 11     | 27.119576196561043752735136780 |
| baseline | fold3_val | -46.16839347293428196204810343  | 5      | 8.170645217025116364839696760  |

#### Per-symbol validation

| Label    | Symbol   | Net expectancy                  | Trades | Max DD %                       |
| -------- | -------- | ------------------------------- | ------ | ------------------------------ |
| baseline | BNB-USDT | 14.542881069389324439192433438  | 3      | 3.0721761454056477365617863100 |
| baseline | BTC-USDT | -11.699900828555999680781884762 | 4      | 7.0159901762649063529564266900 |
| baseline | SOL-USDT | 0                               | 0      | 0                              |
| baseline | XRP-USDT | 13.0479819988191918477929693    | 2      | 7.0103761403630189567976394700 |

## 7. Parameter sensitivity

Each row groups candidates by a tested parameter value and reports the validation net expectancy range (pooled across symbols, base costs). Joint draws change several parameters at once, so these ranges are descriptive rather than causal.

### 4hour

| Parameter           | Value | Candidates | Mean validation net expectancy | Min                             | Max                            |
| ------------------- | ----- | ---------- | ------------------------------ | ------------------------------- | ------------------------------ |
| RSI period          | 14    | 12         | 3.0278913240337882556964596806 | -1.3818119565834074921706458864 | 7.2856215107735568678897597111 |
| RSI period          | 21    | 12         | 6.1981306152877907317836030708 | -1.8059374851322044171091646332 | 32.446256719092216220428964519 |
| RSI long threshold  | 55    | 6          | 1.950044364719098041114158772  | 0.4291780673775512420854536315  | 4.5476811735920372667418769065 |
| RSI long threshold  | 60    | 6          | 3.5820045568760328683628602197 | 0.173429938064800172183521723   | 5.6969316305901952923532804562 |
| RSI long threshold  | 65    | 6          | 1.8202380109923823700225167595 | -1.8059374851322044171091646332 | 6.8290813654727943709281170287 |
| RSI long threshold  | 70    | 6          | 11.099756946055644695460589751 | 0.3875673420815930489224552417  | 32.446256719092216220428964519 |
| RSI short threshold | 40    | 8          | 6.0700294852482899083850295805 | -1.3818119565834074921706458864 | 32.446256719092216220428964519 |
| RSI short threshold | 45    | 8          | 2.4213144785551373259154240199 | -1.8059374851322044171091646332 | 9.489500696543223737927298287  |
| RSI short threshold | 50    | 8          | 5.3476889451789412469196405266 | 1.6625126888395768966992694725  | 13.096843994774996415645494587 |

### 1day

| Parameter           | Value | Candidates | Mean validation net expectancy  | Min                             | Max                            |
| ------------------- | ----- | ---------- | ------------------------------- | ------------------------------- | ------------------------------ |
| RSI period          | 14    | 12         | 5.1515751020192373855512998136  | -29.386181806991472715735428859 | 30.582025347807484420879154902 |
| RSI period          | 21    | 12         | -20.904158529158276947290927248 | -83.1326688389265041328509176   | 39.414030243287573830627893650 |
| RSI long threshold  | 55    | 6          | 1.465081068629970160260100677   | -24.394427924362560427896784721 | 25.15701135129127003981358967  |
| RSI long threshold  | 60    | 6          | 1.250027167842155644919278189   | -43.624705743293725927469933391 | 30.582025347807484420879154902 |
| RSI long threshold  | 65    | 6          | -25.498883510674675507035806612 | -83.1326688389265041328509176   | 19.018261604903407676325781204 |
| RSI long threshold  | 70    | 6          | -8.721391580075529421622827122  | -29.386181806991472715735428859 | 39.414030243287573830627893650 |
| RSI short threshold | 40    | 8          | -24.444823394625128470224178164 | -83.1326688389265041328509176   | 13.553734687257987524181995333 |
| RSI short threshold | 45    | 8          | -8.419741418028006601012050450  | -60.416843703045957927043560255 | 19.496159454323929949963712517 |
| RSI short threshold | 50    | 8          | 9.235689671944575728626787462   | -33.715294202469818498610221822 | 39.414030243287573830627893650 |

Positions are reversal-only and close at end of data; there are no protective levels.

## 8. Cost assumptions

- Base scenario: 10 bps fee + 5 bps slippage per side (15 bps per side in total).
- Stress scenario: 20 bps fee + 10 bps slippage per side (30 bps per side in total).
- Costs are applied once to each side of the actual simulated fill notional. Every reported metric is net of costs; results are gross of nothing.
- The simulator caps a position's gross loss at its notional `E`; a net loss may slightly exceed `E` because costs are real.

## 9. Command (synthesized from run configuration)

The actual process argv was not captured; this invocation is synthesized from the recorded run configuration.

```bash
make analyzer-sweep ARGS="--symbols BTC-USDT,BNB-USDT,XRP-USDT,SOL-USDT --timeframes 4hour,1day --months 34 --as-of 1792713600000 --generated-at 2026-10-23T00:00:00Z --seed 20261023 --fee-bps 10 --slip-bps 5 --stress-fee-bps 20 --stress-slip-bps 10 --init-fund 1000"
cargo fmt -p analyzer
cargo test -p analyzer --features harness
cargo clippy -p analyzer --features harness --all-targets --no-deps -- -D warnings
cargo build -p analyzer --no-default-features
```

## 9b. Limitations

The neighbor-median-within-one-standard-error stability check from the study spec is approximated by fold and symbol consistency: the full 24-point grid is evaluated, so a candidate's one-step grid neighbors are present. No bootstrap confidence interval is reported; only the pooled standard error of validation net expectancy is shown. The single chronological 60/20/20 split is regime-sensitive, and the holdout is a one-shot evaluation rather than a rolling out-of-sample test.

## 10. Recommendation

**insufficient evidence**

No candidate met the validation criteria for every requested timeframe, so the screening produced no evidence strong enough to change the study baseline. This is a provisional screening result, not a profitability claim; a larger or differently scoped experiment would be needed before revising the study baseline.
