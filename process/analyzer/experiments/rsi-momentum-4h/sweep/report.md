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

| Label                | Config       | Val trades | Val net expectancy              | Val PF                         | Val max DD %                   | Val stderr        | Holdout trades | Holdout net expectancy         | Holdout PF                     | Holdout max DD %              | Status       |
| -------------------- | ------------ | ---------- | ------------------------------- | ------------------------------ | ------------------------------ | ----------------- | -------------- | ------------------------------ | ------------------------------ | ----------------------------- | ------------ |
| baseline             | RSI 21/60:50 | 34         | -13.770335279714112040908606072 | 0.0942652757748924780745880734 | 19.867415685752574471802189020 | 2.482255136787885 | 25             | 6.6390391955048502395539701204 | 1.5496111539018269089644331898 | 9.258724973192409032364620340 | not selected |
| rsi14-long55-short40 | RSI 14/55:40 | 43         | -0.6119286038324708905159477326 | 0.9368365047166157329246657408 | 16.169061879707718930620328400 | 4.191786237765639 | —              | —                              | —                              | —                             | not selected |
| rsi14-long55-short45 | RSI 14/55:45 | 52         | -1.6331114227635480414760723133 | 0.7901222415681367234564567569 | 12.252260616161697883458533190 | 2.955526884065291 | —              | —                              | —                              | —                             | not selected |
| rsi14-long55-short50 | RSI 14/55:50 | 64         | -2.1355441988932522797957253683 | 0.6458331411736526821446661252 | 9.496051137205874071037782670  | 1.745629990370979 | —              | —                              | —                              | —                             | not selected |
| rsi14-long60-short40 | RSI 14/60:40 | 33         | -2.0932994105935144525053448147 | 0.8265447397984028445207016193 | 11.881253717951548335818448930 | 5.14338125182008  | —              | —                              | —                              | —                             | not selected |
| rsi14-long60-short45 | RSI 14/60:45 | 38         | -3.453335048771479819235660675  | 0.6616123789033624971507693443 | 11.546503592495475779306370860 | 3.801780445033601 | —              | —                              | —                              | —                             | not selected |
| rsi14-long60-short50 | RSI 14/60:50 | 42         | -3.1798655121747570198932142031 | 0.5904093966138419239726133633 | 10.129543078550913204148647890 | 2.407205539575122 | —              | —                              | —                              | —                             | not selected |
| rsi14-long65-short40 | RSI 14/65:40 | 27         | -6.1529084300521696310939930541 | 0.5849383826420288885427084903 | 14.663995272097220491719245560 | 5.65571156936221  | —              | —                              | —                              | —                             | not selected |
| rsi14-long65-short45 | RSI 14/65:45 | 32         | -8.720032259975687463676021152  | 0.3715826315333978496564910858 | 15.18398518262345148184377800  | 4.25596739405369  | —              | —                              | —                              | —                             | not selected |
| rsi14-long65-short50 | RSI 14/65:50 | 34         | -8.075010608021243060259588016  | 0.2630367848406232127810604211 | 15.481033699524140979209794770 | 2.514391213606289 | —              | —                              | —                              | —                             | not selected |
| rsi14-long70-short40 | RSI 14/70:40 | 19         | -12.250178990321668992133034133 | 0.4223238279090073197848611735 | 18.962654288230318171347360050 | 8.60149539843592  | —              | —                              | —                              | —                             | not selected |
| rsi14-long70-short45 | RSI 14/70:45 | 24         | -17.12970230780685315009078101  | 0.208997777388347022150141641  | 19.897583665563804100904289170 | 6.42624922629494  | —              | —                              | —                              | —                             | not selected |
| rsi14-long70-short50 | RSI 14/70:50 | 24         | -16.309808493177920766022403637 | 0.0614852874633909682824720417 | 15.536087050780778879814957820 | 4.049473659470786 | —              | —                              | —                              | —                             | not selected |
| rsi21-long55-short40 | RSI 21/55:40 | 21         | 1.444272181836223834835912553   | 1.1141584978842159030494187158 | 9.649864018895386483503685540  | 7.46891526173764  | —              | —                              | —                              | —                             | not selected |
| rsi21-long55-short45 | RSI 21/55:45 | 37         | -3.4489252693755939324559305641 | 0.6915966994770881729275124941 | 13.770101719016379594104025360 | 4.198858961186979 | —              | —                              | —                              | —                             | not selected |
| rsi21-long55-short50 | RSI 21/55:50 | 46         | -6.0201415724668794524262459578 | 0.3106867593949374560904188974 | 15.975269875460320415861413360 | 1.993459405697681 | —              | —                              | —                              | —                             | not selected |
| rsi21-long60-short40 | RSI 21/60:40 | 16         | -2.4328012423554460431602706559 | 0.8356315236034371405829187187 | 14.436299613246398373784591620 | 8.70091468356727  | —              | —                              | —                              | —                             | not selected |
| rsi21-long60-short45 | RSI 21/60:45 | 25         | -8.136511440829356777207241968  | 0.4637087363288062084597934077 | 14.668735857234941201399351020 | 5.37357669727466  | —              | —                              | —                              | —                             | not selected |
| rsi21-long65-short40 | RSI 21/65:40 | 13         | -13.786402648430390998034932802 | 0.433810279377185604370518363  | 14.37693454940547726492467600  | 11.68889653112679 | —              | —                              | —                              | —                             | not selected |
| rsi21-long65-short45 | RSI 21/65:45 | 15         | -15.16340538476252359579016174  | 0.3644025320831678684123694544 | 12.989287347192848327790327430 | 9.11620947874406  | —              | —                              | —                              | —                             | not selected |
| rsi21-long65-short50 | RSI 21/65:50 | 20         | -25.383019471315889981323223997 | 0.0256853907981010083287201275 | 26.746916606207460436840140160 | 4.84264803783585  | —              | —                              | —                              | —                             | not selected |
| rsi21-long70-short40 | RSI 21/70:40 | 8          | -28.324359310645724348020385861 | 0.2334565829036682737310098649 | 20.343590164242398071861139320 | 17.52838070409435 | —              | —                              | —                              | —                             | not selected |
| rsi21-long70-short45 | RSI 21/70:45 | 8          | -20.92107075806257540436912973  | 0.3466281274389419221488717631 | 17.663963259478455718453824020 | 17.2902819119798  | —              | —                              | —                              | —                             | not selected |
| rsi21-long70-short50 | RSI 21/70:50 | 10         | -48.561089987689672931328307441 | 0                              | 25.89445060489845279886374300  | 7.66538257646639  | —              | —                              | —                              | —                             | not selected |

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

## 5. Selection

Selection uses validation only under the base cost scenario. Stress results are reported as a robustness check but never used for selection.

### 4hour

Selection: no candidate meets the criteria for this timeframe.

Notes:

- baseline: rejected: validation net expectancy -13.770335279714112040908606072 is not positive
- no candidate meets the criteria
- rsi14-long55-short40: rejected: validation net expectancy -0.6119286038324708905159477326 is not positive
- rsi14-long55-short45: rejected: validation net expectancy -1.6331114227635480414760723133 is not positive
- rsi14-long55-short50: rejected: validation net expectancy -2.1355441988932522797957253683 is not positive
- rsi14-long60-short40: rejected: validation net expectancy -2.0932994105935144525053448147 is not positive
- rsi14-long60-short45: rejected: validation net expectancy -3.453335048771479819235660675 is not positive
- rsi14-long60-short50: rejected: validation net expectancy -3.1798655121747570198932142031 is not positive
- rsi14-long65-short40: rejected: validation net expectancy -6.1529084300521696310939930541 is not positive
- rsi14-long65-short45: rejected: validation net expectancy -8.720032259975687463676021152 is not positive
- rsi14-long65-short50: rejected: validation net expectancy -8.075010608021243060259588016 is not positive
- rsi14-long70-short40: rejected: validation net expectancy -12.250178990321668992133034133 is not positive
- rsi14-long70-short45: rejected: validation net expectancy -17.12970230780685315009078101 is not positive
- rsi14-long70-short50: rejected: validation net expectancy -16.309808493177920766022403637 is not positive
- rsi21-long55-short40: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- rsi21-long55-short45: rejected: validation net expectancy -3.4489252693755939324559305641 is not positive
- rsi21-long55-short50: rejected: validation net expectancy -6.0201415724668794524262459578 is not positive
- rsi21-long60-short40: rejected: validation net expectancy -2.4328012423554460431602706559 is not positive
- rsi21-long60-short45: rejected: validation net expectancy -8.136511440829356777207241968 is not positive
- rsi21-long65-short40: rejected: validation net expectancy -13.786402648430390998034932802 is not positive
- rsi21-long65-short45: rejected: validation net expectancy -15.16340538476252359579016174 is not positive
- rsi21-long65-short50: rejected: validation net expectancy -25.383019471315889981323223997 is not positive
- rsi21-long70-short40: rejected: validation net expectancy -28.324359310645724348020385861 is not positive
- rsi21-long70-short45: rejected: validation net expectancy -20.92107075806257540436912973 is not positive
- rsi21-long70-short50: rejected: validation net expectancy -48.561089987689672931328307441 is not positive

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
| RSI period          | 14    | 12         | -6.8120604405320471305581488425 | -17.12970230780685315009078101  | -0.6119286038324708905159477326 |
| RSI period          | 21    | 12         | -15.375315848650995139182377019 | -48.561089987689672931328307441 | 1.444272181836223834835912553   |
| RSI long threshold  | 55    | 6          | -2.0675631475825867936390015638 | -6.0201415724668794524262459578 | 1.444272181836223834835912553   |
| RSI long threshold  | 60    | 6          | -5.5110246557397776921517230648 | -13.770335279714112040908606072 | -2.0932994105935144525053448147 |
| RSI long threshold  | 65    | 6          | -12.88012980042631745502965346  | -25.383019471315889981323223997 | -6.1529084300521696310939930541 |
| RSI long threshold  | 70    | 6          | -23.916034974617402598660673635 | -48.561089987689672931328307441 | -12.250178990321668992133034133 |
| RSI short threshold | 40    | 8          | -8.025950806799395190078499562  | -28.324359310645724348020385861 | 1.444272181836223834835912553   |
| RSI short threshold | 45    | 8          | -9.825761736543452273037624894  | -20.92107075806257540436912973  | -1.6331114227635480414760723133 |
| RSI short threshold | 50    | 8          | -15.429351890431715941494664336 | -48.561089987689672931328307441 | -2.1355441988932522797957253683 |

Positions are reversal-only and close at end of data; there are no protective levels.

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

The neighbor-median-within-one-standard-error stability check from the study spec is approximated by fold and symbol consistency: the full 24-point grid is evaluated, so a candidate's one-step grid neighbors are present. No bootstrap confidence interval is reported; only the pooled standard error of validation net expectancy is shown. The single chronological 60/20/20 split is regime-sensitive, and the holdout is a one-shot evaluation rather than a rolling out-of-sample test.

## 10. Recommendation

**insufficient evidence**

No candidate met the validation criteria for every requested timeframe, so the screening produced no evidence strong enough to change the study baseline. This is a provisional screening result, not a profitability claim; a larger or differently scoped experiment would be needed before revising the study baseline.
