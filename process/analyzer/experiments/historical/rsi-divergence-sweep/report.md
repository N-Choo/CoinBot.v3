# Analyzer Hyperparameter Screening Report

Provisional screening results. This report does not claim profitability.

## 1. Run metadata

| Field        | Value                                 |
| ------------ | ------------------------------------- |
| Generated at | 2026-10-23T00:00:00Z                  |
| As of (ms)   | 1792713600000                         |
| Months       | 12                                    |
| Symbols      | BTC-USDT, ETH-USDT, SOL-USDT          |
| Timeframes   | 4hour                                 |
| Seed         | 20261023                              |
| Base costs   | 10 bps fee + 5 bps slippage per side  |
| Stress costs | 20 bps fee + 10 bps slippage per side |
| Init fund    | 1000                                  |

## 1b. Baseline configuration

Source: pinned per screening design decision 5; not read from .env (`baseline`: RSI 21/60:50 ATR off SL 2.0x TP 3.0x).

| Parameter           | Values         | Role          |
| ------------------- | -------------- | ------------- |
| RSI period          | 14, 21         | varied        |
| RSI long threshold  | 55, 60, 65, 70 | varied        |
| RSI short threshold | 40, 45, 50     | varied        |
| ATR enabled         | false, true    | varied        |
| ATR SL multiplier   | 1.5, 2.0, 2.5  | varied        |
| ATR TP multiplier   | 2.0, 3.0, 4.0  | varied        |
| ATR period          | 14             | held constant |
| ATR pct high        | 5.0            | held constant |
| ATR high adjust     | 1.5            | held constant |
| ATR SL max pct      | 0              | held constant |
| Window              | 100            | held constant |

## 2. Data quality

| Symbol   | Timeframe | Requested range (ms)         | Effective range (ms)         | Candles | Requested | Expected | Missing | Duplicates | Pages | SHA-256                                                          |
| -------- | --------- | ---------------------------- | ---------------------------- | ------- | --------- | -------- | ------- | ---------- | ----- | ---------------------------------------------------------------- |
| BTC-USDT | 4hour     | 1761609600000..1792713600000 | 1761609600000..1792684800000 | 2159    | 2161      | 2159     | 0       | 0          | 2     | 0b90310252caea1bcb9f1b2ff8e68fa80b6260214c48522f01d718767c2cc34d |
| ETH-USDT | 4hour     | 1761609600000..1792713600000 | 1761609600000..1792684800000 | 2159    | 2161      | 2159     | 0       | 0          | 2     | 4e7db149f849f7a0d4f0ebce0105b923cc5a71945426ccb977be1655623c2dc9 |
| SOL-USDT | 4hour     | 1761609600000..1792713600000 | 1761609600000..1792684800000 | 2159    | 2161      | 2159     | 0       | 0          | 2     | d2c49f957a66c9859180cc5a4f2118d01d409ad37127fccfad5befd708a3b365 |

### Errors

No errors recorded.

### Warnings

No warnings recorded.

## 2b. Chronological splits

Each split is chronological: 60% train, 20% validation, 20% holdout, plus three expanding folds inside train+validation. Indices are half-open (`start` inclusive, `end` exclusive); timestamps are the open times of the first and last candle in each range.

| Symbol   | Timeframe | Partition   | Start index | End index | Start                | End                  | Fold validation ends <= holdout start |
| -------- | --------- | ----------- | ----------- | --------- | -------------------- | -------------------- | ------------------------------------- |
| BTC-USDT | 4hour     | train       | 0           | 1295      | 2025-10-28T00:00:00Z | 2026-05-31T16:00:00Z | —                                     |
| BTC-USDT | 4hour     | validation  | 1295        | 1727      | 2026-05-31T20:00:00Z | 2026-08-11T16:00:00Z | —                                     |
| BTC-USDT | 4hour     | holdout     | 1727        | 2159      | 2026-08-11T20:00:00Z | 2026-10-22T16:00:00Z | —                                     |
| BTC-USDT | 4hour     | fold1_train | 0           | 863       | 2025-10-28T00:00:00Z | 2026-03-20T16:00:00Z | —                                     |
| BTC-USDT | 4hour     | fold1_val   | 863         | 1187      | 2026-03-20T20:00:00Z | 2026-05-13T16:00:00Z | yes                                   |
| BTC-USDT | 4hour     | fold2_train | 0           | 1187      | 2025-10-28T00:00:00Z | 2026-05-13T16:00:00Z | —                                     |
| BTC-USDT | 4hour     | fold2_val   | 1187        | 1511      | 2026-05-13T20:00:00Z | 2026-07-06T16:00:00Z | yes                                   |
| BTC-USDT | 4hour     | fold3_train | 0           | 1511      | 2025-10-28T00:00:00Z | 2026-07-06T16:00:00Z | —                                     |
| BTC-USDT | 4hour     | fold3_val   | 1511        | 1727      | 2026-07-06T20:00:00Z | 2026-08-11T16:00:00Z | yes                                   |
| ETH-USDT | 4hour     | train       | 0           | 1295      | 2025-10-28T00:00:00Z | 2026-05-31T16:00:00Z | —                                     |
| ETH-USDT | 4hour     | validation  | 1295        | 1727      | 2026-05-31T20:00:00Z | 2026-08-11T16:00:00Z | —                                     |
| ETH-USDT | 4hour     | holdout     | 1727        | 2159      | 2026-08-11T20:00:00Z | 2026-10-22T16:00:00Z | —                                     |
| ETH-USDT | 4hour     | fold1_train | 0           | 863       | 2025-10-28T00:00:00Z | 2026-03-20T16:00:00Z | —                                     |
| ETH-USDT | 4hour     | fold1_val   | 863         | 1187      | 2026-03-20T20:00:00Z | 2026-05-13T16:00:00Z | yes                                   |
| ETH-USDT | 4hour     | fold2_train | 0           | 1187      | 2025-10-28T00:00:00Z | 2026-05-13T16:00:00Z | —                                     |
| ETH-USDT | 4hour     | fold2_val   | 1187        | 1511      | 2026-05-13T20:00:00Z | 2026-07-06T16:00:00Z | yes                                   |
| ETH-USDT | 4hour     | fold3_train | 0           | 1511      | 2025-10-28T00:00:00Z | 2026-07-06T16:00:00Z | —                                     |
| ETH-USDT | 4hour     | fold3_val   | 1511        | 1727      | 2026-07-06T20:00:00Z | 2026-08-11T16:00:00Z | yes                                   |
| SOL-USDT | 4hour     | train       | 0           | 1295      | 2025-10-28T00:00:00Z | 2026-05-31T16:00:00Z | —                                     |
| SOL-USDT | 4hour     | validation  | 1295        | 1727      | 2026-05-31T20:00:00Z | 2026-08-11T16:00:00Z | —                                     |
| SOL-USDT | 4hour     | holdout     | 1727        | 2159      | 2026-08-11T20:00:00Z | 2026-10-22T16:00:00Z | —                                     |
| SOL-USDT | 4hour     | fold1_train | 0           | 863       | 2025-10-28T00:00:00Z | 2026-03-20T16:00:00Z | —                                     |
| SOL-USDT | 4hour     | fold1_val   | 863         | 1187      | 2026-03-20T20:00:00Z | 2026-05-13T16:00:00Z | yes                                   |
| SOL-USDT | 4hour     | fold2_train | 0           | 1187      | 2025-10-28T00:00:00Z | 2026-05-13T16:00:00Z | —                                     |
| SOL-USDT | 4hour     | fold2_val   | 1187        | 1511      | 2026-05-13T20:00:00Z | 2026-07-06T16:00:00Z | yes                                   |
| SOL-USDT | 4hour     | fold3_train | 0           | 1511      | 2025-10-28T00:00:00Z | 2026-07-06T16:00:00Z | —                                     |
| SOL-USDT | 4hour     | fold3_val   | 1511        | 1727      | 2026-07-06T20:00:00Z | 2026-08-11T16:00:00Z | yes                                   |

Every `fold*_val` row above ends at or before the holdout start index, so no fold validation overlaps the holdout.

## 3. Baseline vs candidates

Validation is the selection partition; holdout is the one-shot evaluation partition, scored only for the baseline and selected winners, and is never used for selection. Metrics are pooled across symbols under base costs. A missing holdout is shown as —.

Footnote: _pooled max DD_ is the maximum per-symbol drawdown, not a portfolio drawdown.

### 4hour

| Label            | Config                               | Val trades | Val net expectancy              | Val PF                         | Val max DD %                   | Val stderr        | Holdout trades | Holdout net expectancy         | Holdout PF                     | Holdout max DD %              | Status       |
| ---------------- | ------------------------------------ | ---------- | ------------------------------- | ------------------------------ | ------------------------------ | ----------------- | -------------- | ------------------------------ | ------------------------------ | ----------------------------- | ------------ |
| atr_enabled=true | RSI 21/60:50 ATR on SL 2.0x TP 3.0x  | 52         | -5.0932943486493836304669488762 | 0.5397573995861121755811274903 | 13.726074595676766307029335810 | 2.493468777897146 | —              | —                              | —                              | —                             | not selected |
| atr_sl=1.5       | RSI 21/60:50 ATR on SL 1.5x TP 3.0x  | 58         | -3.974843495365881768156530324  | 0.5736791530370006937323883908 | 15.140938088083969871014638620 | 2.032199153955999 | —              | —                              | —                              | —                             | not selected |
| atr_sl=2.5       | RSI 21/60:50 ATR on SL 2.5x TP 3.0x  | 49         | -6.1891824059091910637947740553 | 0.50718955568947474879727059   | 17.331518619075311822763955460 | 2.836702649953416 | —              | —                              | —                              | —                             | not selected |
| atr_tp=2.0       | RSI 21/60:50 ATR on SL 2.0x TP 2.0x  | 62         | -5.0086208901416727337984076692 | 0.4912037363143101461597128018 | 12.803398223702593038687286940 | 1.92017872196578  | —              | —                              | —                              | —                             | not selected |
| atr_tp=4.0       | RSI 21/60:50 ATR on SL 2.0x TP 4.0x  | 42         | -8.330120231843542792859079626  | 0.3061569381983104460202210619 | 14.695765439271051812010759170 | 2.501185132048267 | —              | —                              | —                              | —                             | not selected |
| baseline         | RSI 21/60:50 ATR off SL 2.0x TP 3.0x | 34         | -13.770335279714112040908606072 | 0.0942652757748924780745880734 | 19.867415685752574471802189020 | 2.482255136787885 | 25             | 6.6390391955048502395539701204 | 1.5496111539018269089644331898 | 9.258724973192409032364620340 | not selected |
| joint-12         | RSI 14/55:45 ATR on SL 2.5x TP 4.0x  | 62         | -1.8472122095543049604399669211 | 0.788403693985136121570186953  | 9.833321772706816382182893270  | 2.477021848378633 | —              | —                              | —                              | —                             | not selected |
| joint-13         | RSI 14/65:40 ATR on SL 1.5x TP 4.0x  | 47         | -5.3057984482977189837750921785 | 0.489375938166852273212984385  | 16.325812955185055960241826580 | 2.45140660518758  | —              | —                              | —                              | —                             | not selected |
| joint-14         | RSI 14/60:40 ATR off SL 2.0x TP 2.0x | 33         | -2.0932994105935144525053448147 | 0.8265447397984028445207016193 | 11.881253717951548335818448930 | 5.14338125182008  | —              | —                              | —                              | —                             | not selected |
| joint-15         | RSI 14/65:45 ATR on SL 1.5x TP 2.0x  | 66         | -2.4335903361134865232333601252 | 0.684259064413999998560633463  | 13.529088249083225143189784300 | 1.658183812412718 | —              | —                              | —                              | —                             | not selected |
| joint-16         | RSI 21/70:45 ATR on SL 2.0x TP 3.0x  | 32         | -8.261964941624419475509109215  | 0.3982638096025780258755482788 | 13.756912159384386632120175230 | 3.317463266486442 | —              | —                              | —                              | —                             | not selected |
| joint-17         | RSI 21/70:50 ATR on SL 1.5x TP 3.0x  | 39         | -4.8914775899111698895372310764 | 0.512541331514679910429394759  | 15.374973182396989132728808660 | 2.418069663556696 | —              | —                              | —                              | —                             | not selected |
| joint-18         | RSI 14/65:45 ATR off SL 2.5x TP 3.0x | 32         | -8.720032259975687463676021152  | 0.3715826315333978496564910858 | 15.18398518262345148184377800  | 4.25596739405369  | —              | —                              | —                              | —                             | not selected |
| joint-19         | RSI 21/70:40 ATR off SL 1.5x TP 2.0x | 8          | -28.324359310645724348020385861 | 0.2334565829036682737310098649 | 20.343590164242398071861139320 | 17.52838070409435 | —              | —                              | —                              | —                             | not selected |
| joint-20         | RSI 21/55:40 ATR off SL 1.5x TP 4.0x | 21         | 1.444272181836223834835912553   | 1.1141584978842159030494187158 | 9.649864018895386483503685540  | 7.46891526173764  | —              | —                              | —                              | —                             | not selected |
| joint-21         | RSI 14/65:45 ATR on SL 2.0x TP 4.0x  | 44         | -6.3164707505063405091220363318 | 0.4631018828269924377376326533 | 16.179270639434895865065786100 | 2.810874255081636 | —              | —                              | —                              | —                             | not selected |
| joint-22         | RSI 14/70:40 ATR off SL 2.0x TP 4.0x | 19         | -12.250178990321668992133034133 | 0.4223238279090073197848611735 | 18.962654288230318171347360050 | 8.60149539843592  | —              | —                              | —                              | —                             | not selected |
| joint-23         | RSI 21/60:50 ATR on SL 1.5x TP 2.0x  | 66         | -3.7620721440450479353161272498 | 0.5372180627118044866120934646 | 11.852157138923993762363140950 | 1.574299012047829 | —              | —                              | —                              | —                             | not selected |
| joint-24         | RSI 21/60:40 ATR on SL 1.5x TP 2.0x  | 50         | -6.587430468748732152813818982  | 0.3461747771035610304797488216 | 18.330864339182065845036960290 | 1.791537307598159 | —              | —                              | —                              | —                             | not selected |
| joint-25         | RSI 14/55:50 ATR off SL 2.0x TP 3.0x | 64         | -2.1355441988932522797957253683 | 0.6458331411736526821446661252 | 9.496051137205874071037782670  | 1.745629990370979 | —              | —                              | —                              | —                             | not selected |
| joint-26         | RSI 14/65:45 ATR on SL 2.5x TP 4.0x  | 43         | -6.8467909922550367132231897412 | 0.4896479995094835151237798563 | 17.098634620337544030995216420 | 3.336252131212039 | —              | —                              | —                              | —                             | not selected |
| joint-27         | RSI 14/70:40 ATR on SL 2.0x TP 4.0x  | 38         | -7.8280129780920827667799471726 | 0.4258225472199625941844815457 | 16.812517403692179262434569730 | 3.280981854486126 | —              | —                              | —                              | —                             | not selected |
| joint-28         | RSI 14/65:40 ATR on SL 2.0x TP 3.0x  | 49         | -7.1011671760581385339077246776 | 0.4405332050920555297342742737 | 18.617204423793971177132444030 | 2.631059473283783 | —              | —                              | —                              | —                             | not selected |
| joint-29         | RSI 14/65:40 ATR on SL 2.0x TP 2.0x  | 55         | -5.2302848911694033938377536535 | 0.5112137994560429607403942071 | 15.426854421222467988521582430 | 2.244185316005099 | —              | —                              | —                              | —                             | not selected |
| rsi_long=55      | RSI 21/55:50 ATR off SL 2.0x TP 3.0x | 46         | -6.0201415724668794524262459578 | 0.3106867593949374560904188974 | 15.975269875460320415861413360 | 1.993459405697681 | —              | —                              | —                              | —                             | not selected |
| rsi_long=65      | RSI 21/65:50 ATR off SL 2.0x TP 3.0x | 20         | -25.383019471315889981323223997 | 0.0256853907981010083287201275 | 26.746916606207460436840140160 | 4.84264803783585  | —              | —                              | —                              | —                             | not selected |
| rsi_long=70      | RSI 21/70:50 ATR off SL 2.0x TP 3.0x | 10         | -48.561089987689672931328307441 | 0                              | 25.89445060489845279886374300  | 7.66538257646639  | —              | —                              | —                              | —                             | not selected |
| rsi_period=14    | RSI 14/60:50 ATR off SL 2.0x TP 3.0x | 42         | -3.1798655121747570198932142031 | 0.5904093966138419239726133633 | 10.129543078550913204148647890 | 2.407205539575122 | —              | —                              | —                              | —                             | not selected |
| rsi_short=40     | RSI 21/60:40 ATR off SL 2.0x TP 3.0x | 16         | -2.4328012423554460431602706559 | 0.8356315236034371405829187187 | 14.436299613246398373784591620 | 8.70091468356727  | —              | —                              | —                              | —                             | not selected |
| rsi_short=45     | RSI 21/60:45 ATR off SL 2.0x TP 3.0x | 25         | -8.136511440829356777207241968  | 0.4637087363288062084597934077 | 14.668735857234941201399351020 | 5.37357669727466  | —              | —                              | —                              | —                             | not selected |

## 4. Exact configurations tested and reasons

The `origin` column records how each candidate entered the study: `baseline`, `ofat:<parameter>=<value>` (one factor at a time), or `joint` (deterministic seeded draw).

### 4hour

| Label            | Origin                | RSI period | Long threshold | Short threshold | ATR enabled | ATR SL | ATR TP |
| ---------------- | --------------------- | ---------- | -------------- | --------------- | ----------- | ------ | ------ |
| atr_enabled=true | ofat:atr_enabled=true | 21         | 60             | 50              | true        | 2.0    | 3.0    |
| atr_sl=1.5       | ofat:atr_sl=1.5       | 21         | 60             | 50              | true        | 1.5    | 3.0    |
| atr_sl=2.5       | ofat:atr_sl=2.5       | 21         | 60             | 50              | true        | 2.5    | 3.0    |
| atr_tp=2.0       | ofat:atr_tp=2.0       | 21         | 60             | 50              | true        | 2.0    | 2.0    |
| atr_tp=4.0       | ofat:atr_tp=4.0       | 21         | 60             | 50              | true        | 2.0    | 4.0    |
| baseline         | baseline              | 21         | 60             | 50              | false       | 2.0    | 3.0    |
| joint-12         | joint                 | 14         | 55             | 45              | true        | 2.5    | 4.0    |
| joint-13         | joint                 | 14         | 65             | 40              | true        | 1.5    | 4.0    |
| joint-14         | joint                 | 14         | 60             | 40              | false       | 2.0    | 2.0    |
| joint-15         | joint                 | 14         | 65             | 45              | true        | 1.5    | 2.0    |
| joint-16         | joint                 | 21         | 70             | 45              | true        | 2.0    | 3.0    |
| joint-17         | joint                 | 21         | 70             | 50              | true        | 1.5    | 3.0    |
| joint-18         | joint                 | 14         | 65             | 45              | false       | 2.5    | 3.0    |
| joint-19         | joint                 | 21         | 70             | 40              | false       | 1.5    | 2.0    |
| joint-20         | joint                 | 21         | 55             | 40              | false       | 1.5    | 4.0    |
| joint-21         | joint                 | 14         | 65             | 45              | true        | 2.0    | 4.0    |
| joint-22         | joint                 | 14         | 70             | 40              | false       | 2.0    | 4.0    |
| joint-23         | joint                 | 21         | 60             | 50              | true        | 1.5    | 2.0    |
| joint-24         | joint                 | 21         | 60             | 40              | true        | 1.5    | 2.0    |
| joint-25         | joint                 | 14         | 55             | 50              | false       | 2.0    | 3.0    |
| joint-26         | joint                 | 14         | 65             | 45              | true        | 2.5    | 4.0    |
| joint-27         | joint                 | 14         | 70             | 40              | true        | 2.0    | 4.0    |
| joint-28         | joint                 | 14         | 65             | 40              | true        | 2.0    | 3.0    |
| joint-29         | joint                 | 14         | 65             | 40              | true        | 2.0    | 2.0    |
| rsi_long=55      | ofat:rsi_long=55      | 21         | 55             | 50              | false       | 2.0    | 3.0    |
| rsi_long=65      | ofat:rsi_long=65      | 21         | 65             | 50              | false       | 2.0    | 3.0    |
| rsi_long=70      | ofat:rsi_long=70      | 21         | 70             | 50              | false       | 2.0    | 3.0    |
| rsi_period=14    | ofat:rsi_period=14    | 14         | 60             | 50              | false       | 2.0    | 3.0    |
| rsi_short=40     | ofat:rsi_short=40     | 21         | 60             | 40              | false       | 2.0    | 3.0    |
| rsi_short=45     | ofat:rsi_short=45     | 21         | 60             | 45              | false       | 2.0    | 3.0    |

## 5. Selection

Selection uses validation only under the base cost scenario. Stress results are reported as a robustness check but never used for selection.

### 4hour

Selection: no candidate meets the criteria for this timeframe.

Notes:

- atr_enabled=true: rejected: validation net expectancy -5.0932943486493836304669488762 is not positive
- atr_sl=1.5: rejected: validation net expectancy -3.974843495365881768156530324 is not positive
- atr_sl=2.5: rejected: validation net expectancy -6.1891824059091910637947740553 is not positive
- atr_tp=2.0: rejected: validation net expectancy -5.0086208901416727337984076692 is not positive
- atr_tp=4.0: rejected: validation net expectancy -8.330120231843542792859079626 is not positive
- baseline: rejected: validation net expectancy -13.770335279714112040908606072 is not positive
- joint-12: rejected: validation net expectancy -1.8472122095543049604399669211 is not positive
- joint-13: rejected: validation net expectancy -5.3057984482977189837750921785 is not positive
- joint-14: rejected: validation net expectancy -2.0932994105935144525053448147 is not positive
- joint-15: rejected: validation net expectancy -2.4335903361134865232333601252 is not positive
- joint-16: rejected: validation net expectancy -8.261964941624419475509109215 is not positive
- joint-17: rejected: validation net expectancy -4.8914775899111698895372310764 is not positive
- joint-18: rejected: validation net expectancy -8.720032259975687463676021152 is not positive
- joint-19: rejected: validation net expectancy -28.324359310645724348020385861 is not positive
- joint-20: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- joint-21: rejected: validation net expectancy -6.3164707505063405091220363318 is not positive
- joint-22: rejected: validation net expectancy -12.250178990321668992133034133 is not positive
- joint-23: rejected: validation net expectancy -3.7620721440450479353161272498 is not positive
- joint-24: rejected: validation net expectancy -6.587430468748732152813818982 is not positive
- joint-25: rejected: validation net expectancy -2.1355441988932522797957253683 is not positive
- joint-26: rejected: validation net expectancy -6.8467909922550367132231897412 is not positive
- joint-27: rejected: validation net expectancy -7.8280129780920827667799471726 is not positive
- joint-28: rejected: validation net expectancy -7.1011671760581385339077246776 is not positive
- joint-29: rejected: validation net expectancy -5.2302848911694033938377536535 is not positive
- no candidate meets the criteria
- rsi_long=55: rejected: validation net expectancy -6.0201415724668794524262459578 is not positive
- rsi_long=65: rejected: validation net expectancy -25.383019471315889981323223997 is not positive
- rsi_long=70: rejected: validation net expectancy -48.561089987689672931328307441 is not positive
- rsi_period=14: rejected: validation net expectancy -3.1798655121747570198932142031 is not positive
- rsi_short=40: rejected: validation net expectancy -2.4328012423554460431602706559 is not positive
- rsi_short=45: rejected: validation net expectancy -8.136511440829356777207241968 is not positive

## 6. Walk-forward

Expanding-fold validation metrics are pooled across symbols; per-symbol validation metrics are shown separately. Only the baseline and selected winners are shown.

### 4hour

#### Pooled folds

| Label    | Fold      | Net expectancy                  | Trades | Max DD %                       |
| -------- | --------- | ------------------------------- | ------ | ------------------------------ |
| baseline | fold1_val | -13.679107036517069946745310425 | 32     | 26.760119656490743405017306040 |
| baseline | fold2_val | 10.981828468610013336702441207  | 11     | 13.183939771440269982083010470 |
| baseline | fold3_val | -12.660616226663423250798637015 | 22     | 16.823311131770545610679533090 |

#### Per-symbol validation

| Label    | Symbol   | Net expectancy                  | Trades | Max DD %                       |
| -------- | -------- | ------------------------------- | ------ | ------------------------------ |
| baseline | BTC-USDT | -9.294674944327918148351260728  | 12     | 11.153609933193501778021512880 |
| baseline | ETH-USDT | -16.556179738127145393168490839 | 12     | 19.867415685752574471802189020 |
| baseline | SOL-USDT | -15.798114332081904689265558764 | 10     | 17.077865005085129795167122620 |

## 7. Parameter sensitivity

Each row groups candidates by a tested parameter value and reports the validation net expectancy range (pooled across symbols, base costs). Joint draws change several parameters at once, so these ranges are descriptive rather than causal.

### 4hour

| Parameter           | Value | Candidates | Mean validation net expectancy  | Min                             | Max                             |
| ------------------- | ----- | ---------- | ------------------------------- | ------------------------------- | ------------------------------- |
| RSI period          | 14    | 13         | -5.4837113964619532763324931133 | -12.250178990321668992133034133 | -1.8472122095543049604399669211 |
| RSI period          | 21    | 17         | -10.78135250820117054010531744  | -48.561089987689672931328307441 | 1.444272181836223834835912553   |
| RSI long threshold  | 55    | 4          | -2.1396564497695532144565064235 | -6.0201415724668794524262459578 | 1.444272181836223834835912553   |
| RSI long threshold  | 60    | 12         | -5.7131980725308865342400303747 | -13.770335279714112040908606072 | -2.0932994105935144525053448147 |
| RSI long threshold  | 65    | 8          | -8.417144290711462762762300232  | -25.383019471315889981323223997 | -2.4335903361134865232333601252 |
| RSI long threshold  | 70    | 6          | -18.352847299714123067218002483 | -48.561089987689672931328307441 | -4.8914775899111698895372310764 |
| RSI short threshold | 40    | 10         | -7.5709060734446205832097459576 | -28.324359310645724348020385861 | 1.444272181836223834835912553   |
| RSI short threshold | 45    | 7          | -6.0803675615512332032015607791 | -8.720032259975687463676021152  | -1.8472122095543049604399669211 |
| RSI short threshold | 50    | 13         | -10.484585163701573347661878608 | -48.561089987689672931328307441 | -2.1355441988932522797957253683 |
| ATR enabled         | false | 13         | -12.27406973039536445750320839  | -48.561089987689672931328307441 | 1.444272181836223834835912553   |
| ATR enabled         | true  | 17         | -5.5887255469579737545041822276 | -8.330120231843542792859079626  | -1.8472122095543049604399669211 |
| ATR SL multiplier   | 1.5   | 8          | -6.7294124514114422207520791554 | -28.324359310645724348020385861 | 1.444272181836223834835912553   |
| ATR SL multiplier   | 2.0   | 18         | -9.840706850802196322609012324  | -48.561089987689672931328307441 | -2.0932994105935144525053448147 |
| ATR SL multiplier   | 2.5   | 4          | -5.9008044669235550502834879672 | -8.720032259975687463676021152  | -1.8472122095543049604399669211 |
| ATR TP multiplier   | 2.0   | 7          | -7.634236778779654505646456908  | -28.324359310645724348020385861 | -2.0932994105935144525053448147 |
| ATR TP multiplier   | 3.0   | 15         | -10.256751394862215890072745003 | -48.561089987689672931328307441 | -2.1355441988932522797957253683 |
| ATR TP multiplier   | 4.0   | 8          | -5.910039052379308985437054194  | -12.250178990321668992133034133 | 1.444272181836223834835912553   |

ATR SL and TP multipliers only affect candidates with ATR enabled; otherwise positions have no protective levels.

## 8. Cost assumptions

- Base scenario: 10 bps fee + 5 bps slippage per side (15 bps per side in total).
- Stress scenario: 20 bps fee + 10 bps slippage per side (30 bps per side in total).
- Costs are applied once to each side of the actual simulated fill notional. Every reported metric is net of costs; results are gross of nothing.
- The simulator caps a position's gross loss at its notional `E`; a net loss may slightly exceed `E` because costs are real.

## 9. Command (synthesized from run configuration)

The actual process argv was not captured; this invocation is synthesized from the recorded run configuration.

```bash
make analyzer-sweep ARGS="--symbols BTC-USDT,ETH-USDT,SOL-USDT --timeframes 4hour --months 12 --as-of 1792713600000 --generated-at 2026-10-23T00:00:00Z --seed 20261023 --fee-bps 10 --slip-bps 5 --stress-fee-bps 20 --stress-slip-bps 10 --init-fund 1000"
cargo fmt -p analyzer
cargo test -p analyzer --features harness
cargo clippy -p analyzer --features harness --all-targets --no-deps -- -D warnings
cargo build -p analyzer --no-default-features
```

## 9b. Limitations

The neighbor-median-within-one-standard-error stability check from the study spec is approximated by fold and symbol consistency: only 30 of the 432 grid points are sampled, so a candidate's one-step grid neighbors are usually absent. No bootstrap confidence interval is reported; only the pooled standard error of validation net expectancy is shown. The single chronological 60/20/20 split is regime-sensitive, and the holdout is a one-shot evaluation rather than a rolling out-of-sample test.

## 10. Recommendation

**insufficient evidence**

No candidate met the validation criteria for every requested timeframe, so the screening produced no evidence strong enough to change the study baseline. This is a provisional screening result, not a profitability claim; a larger or differently scoped experiment would be needed before revising the study baseline.
