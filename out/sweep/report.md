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

## 2. Data quality

| Symbol | Timeframe | Requested range (ms) | Effective range (ms) | Candles | Requested | Expected | Missing | Duplicates | Pages | SHA-256 |
|---|---|---|---|---|---|---|---|---|---|---|
| BTC-USDT | 1hour | 1730453878840..1792661878840 | 1730455200000..1792656000000 | 17268 | 17281 | 17279 | 11 | 0 | 12 | c096a4d94209b6ca3f739ec24c5b2fe079f03903836b98505535e6a67b6e5edd |
| ETH-USDT | 1hour | 1730453878840..1792661878840 | 1730455200000..1792656000000 | 17268 | 17281 | 17279 | 11 | 0 | 12 | 7277a5e10e2bcd5a509a4d03d22158bce9ef4194ec76c44e18a6c6178d086450 |
| SOL-USDT | 1hour | 1730453878840..1792661878840 | 1730455200000..1792656000000 | 17268 | 17281 | 17279 | 11 | 0 | 12 | c8bf052e5a0dfe2b9f98bc9f00e5104f22bfd8a0f0bf87e11e0b898194cedf3a |
| BTC-USDT | 4hour | 1730453878840..1792661878840 | 1730462400000..1792641600000 | 4317 | 4321 | 4319 | 2 | 0 | 3 | 76a0633ad80992ab68c49f535aa25efc63b24297c583e1c4cb1e68716ded98c9 |
| ETH-USDT | 4hour | 1730453878840..1792661878840 | 1730462400000..1792641600000 | 4317 | 4321 | 4319 | 2 | 0 | 3 | f84f2dabe2f0fec77096d13ecde41f0b490f3017da17db52a18fd7084c26f13c |
| SOL-USDT | 4hour | 1730453878840..1792661878840 | 1730462400000..1792641600000 | 4317 | 4321 | 4319 | 2 | 0 | 3 | 3004f662f29bff966e71fc22b735a99ce5878b0c00c4dfebcc723b8ea59010da |

### Errors

No errors recorded.

### Warnings

- Warning: BTC-USDT 1hour has 11 missing candle(s).
- Warning: ETH-USDT 1hour has 11 missing candle(s).
- Warning: SOL-USDT 1hour has 11 missing candle(s).
- Warning: BTC-USDT 4hour has 2 missing candle(s).
- Warning: ETH-USDT 4hour has 2 missing candle(s).
- Warning: SOL-USDT 4hour has 2 missing candle(s).

## 3. Baseline vs candidates

Validation is the selection partition; holdout is the one-shot evaluation partition, scored only for the baseline and selected winners, and is never used for selection. Metrics are pooled across symbols under base costs. A missing holdout is shown as —.

### 1hour

| Label | Config | Val trades | Val net expectancy | Val PF | Val max DD % | Holdout trades | Holdout net expectancy | Holdout PF | Holdout max DD % | Status |
|---|---|---|---|---|---|---|---|---|---|---|
| atr_sl=2.0 | RSI 14/o5 age 12 SMA on (20) SL 2.0x TP 2.0x | 314 | -2.5450301127617256890536733392 | 0.5722744742369714202072963027 | 39.046163485435845237014880260 | — | — | — | — | not selected |
| atr_sl=2.5 | RSI 14/o5 age 12 SMA on (20) SL 2.5x TP 2.0x | 296 | -2.871746723518612414358229601 | 0.5649513709827961257988362954 | 50.145869644077954747230373410 | — | — | — | — | not selected |
| atr_tp=3.0 | RSI 14/o5 age 12 SMA on (20) SL 1.5x TP 3.0x | 354 | -1.9136641418937114089828634845 | 0.6587572216346807862751399273 | 36.641039808230364456414897090 | — | — | — | — | not selected |
| atr_tp=4.0 | RSI 14/o5 age 12 SMA on (20) SL 1.5x TP 4.0x | 325 | -2.0405978196687696168012087336 | 0.6662402145677385813557758787 | 34.371108858522594345506023310 | — | — | — | — | not selected |
| baseline | RSI 14/o5 age 12 SMA on (20) SL 1.5x TP 2.0x | 349 | -2.2670225776036455328615581383 | 0.5604993659042967477824883131 | 40.998924304049322768782400240 | 453 | -0.8347219815914704130179686623 | 0.7699515224632937207296547631 | 30.431381197542285117753500860 | not selected |
| joint-13 | RSI 10/o5 age uncapped SMA on (20) SL 2.5x TP 4.0x | 278 | -2.2024886680371142893000450969 | 0.7389067053945894762562676816 | 47.812326023137303630752133290 | — | — | — | — | not selected |
| joint-14 | RSI 21/o3 age 8 SMA on (20) SL 1.5x TP 4.0x | 382 | -1.2230036462512042467260423942 | 0.7882000604766348444521919512 | 34.987421634636309019332555570 | — | — | — | — | not selected |
| joint-15 | RSI 10/o5 age 4 SMA off (20) SL 2.5x TP 2.0x | 382 | -2.0573085314543378306422628067 | 0.6714562313452683202518415716 | 44.684156459375553383484811760 | — | — | — | — | not selected |
| joint-16 | RSI 10/o3 age 12 SMA off (20) SL 2.0x TP 2.0x | 554 | -0.8244667472989844548162424831 | 0.8286985545418363536124866472 | 37.107644625326376915696663360 | — | — | — | — | not selected |
| joint-17 | RSI 10/o7 age 8 SMA on (20) SL 1.5x TP 2.0x | 369 | -1.8501609686400524543307422444 | 0.6201480228434894880147038775 | 29.084204046979218286473852930 | — | — | — | — | not selected |
| joint-18 | RSI 21/o7 age 4 SMA on (20) SL 2.0x TP 3.0x | 234 | -1.9819730808235320127573312424 | 0.6997149867324626272589720691 | 26.978866389666143583755955630 | — | — | — | — | not selected |
| joint-19 | RSI 14/o7 age uncapped SMA off (20) SL 1.5x TP 3.0x | 432 | -1.5543863705201693674060520129 | 0.7259374041637923793806695918 | 29.797671445065685036040450580 | — | — | — | — | not selected |
| joint-20 | RSI 14/o5 age 4 SMA off (20) SL 2.5x TP 3.0x | 325 | -1.875580550025433047956462442 | 0.751134486417719435776615051 | 38.771345080468518083316855680 | — | — | — | — | not selected |
| joint-21 | RSI 14/o5 age 12 SMA off (20) SL 2.5x TP 2.0x | 434 | -1.8001000588106799145183931686 | 0.697389128257910428779033821 | 43.838900648336706613395226170 | — | — | — | — | not selected |
| joint-22 | RSI 14/o3 age uncapped SMA off (20) SL 2.5x TP 3.0x | 410 | -1.5847257668934545794773939211 | 0.7792535642571811413033564147 | 54.590692783618801657720799140 | — | — | — | — | not selected |
| joint-23 | RSI 10/o3 age 12 SMA off (20) SL 1.5x TP 4.0x | 481 | -0.6921429075149544348393847022 | 0.8746147457081867655618273571 | 23.331324318345308779472761670 | — | — | — | — | not selected |
| joint-24 | RSI 21/o5 age 12 SMA on (20) SL 2.0x TP 4.0x | 262 | -2.3241345113366879653280136167 | 0.6877677839945337221518537136 | 42.360673736109837533790850720 | — | — | — | — | not selected |
| joint-25 | RSI 21/o3 age 8 SMA off (20) SL 2.5x TP 2.0x | 473 | -1.6652132663574724668358794287 | 0.7165660642346787108748252081 | 58.679991127155876865578598830 | — | — | — | — | not selected |
| joint-26 | RSI 21/o7 age uncapped SMA off (20) SL 2.0x TP 4.0x | 328 | -1.4206338950783102904669775716 | 0.7983001870109363054173356696 | 30.383936037705130829763673360 | — | — | — | — | not selected |
| joint-27 | RSI 10/o3 age uncapped SMA on (20) SL 1.5x TP 2.0x | 506 | -1.2907504023818207284096996364 | 0.7121748181061676059612638903 | 38.174736939363865581675431820 | — | — | — | — | not selected |
| joint-28 | RSI 10/o7 age 8 SMA off (20) SL 1.5x TP 4.0x | 348 | -2.0765818476200339155092829495 | 0.6668402997493381904102688386 | 34.108681823073491848326925960 | — | — | — | — | not selected |
| joint-29 | RSI 10/o5 age uncapped SMA on (20) SL 1.5x TP 2.0x | 411 | -1.8138214314622790507477512083 | 0.6237845500225444292983771331 | 32.930464016873414485087907510 | — | — | — | — | not selected |
| max_age=4 | RSI 14/o5 age 4 SMA on (20) SL 1.5x TP 2.0x | 309 | -2.1335302131423574080135933129 | 0.5785953216834012406085257033 | 34.423255651421271855487149350 | — | — | — | — | not selected |
| max_age=8 | RSI 14/o5 age 8 SMA on (20) SL 1.5x TP 2.0x | 349 | -2.1454435939397129771718356239 | 0.5699645373171399365000320404 | 38.521248585892143116334606180 | — | — | — | — | not selected |
| max_age=uncapped | RSI 14/o5 age uncapped SMA on (20) SL 1.5x TP 2.0x | 386 | -2.0635313914860278140686276712 | 0.5851844015566947356330968671 | 40.724778596946726188711212380 | — | — | — | — | not selected |
| rsi_order=3 | RSI 14/o3 age 12 SMA on (20) SL 1.5x TP 2.0x | 455 | -1.5682800294252363122873505127 | 0.6659033361878355763693400917 | 41.432486802666643845055602420 | — | — | — | — | not selected |
| rsi_order=7 | RSI 14/o7 age 12 SMA on (20) SL 1.5x TP 2.0x | 365 | -1.8074167808717573984841567612 | 0.6371394624876000193493714433 | 27.449378601884302968609346920 | — | — | — | — | not selected |
| rsi_period=10 | RSI 10/o5 age 12 SMA on (20) SL 1.5x TP 2.0x | 389 | -1.9358297351080501452790663989 | 0.606893118055374994797069953 | 34.966132688352318754192112100 | — | — | — | — | not selected |
| rsi_period=21 | RSI 21/o5 age 12 SMA on (20) SL 1.5x TP 2.0x | 380 | -1.8243019400176149726844216141 | 0.6236256472428205045269144368 | 35.393321647663415672558202050 | — | — | — | — | not selected |
| sma_enabled=false | RSI 14/o5 age 12 SMA off (20) SL 1.5x TP 2.0x | 498 | -1.9380920222398780857525872564 | 0.6042803315257285190197407993 | 40.377571038351520149298978840 | — | — | — | — | not selected |

### 4hour

| Label | Config | Val trades | Val net expectancy | Val PF | Val max DD % | Holdout trades | Holdout net expectancy | Holdout PF | Holdout max DD % | Status |
|---|---|---|---|---|---|---|---|---|---|---|
| atr_sl=2.0 | RSI 14/o5 age 12 SMA on (20) SL 2.0x TP 2.0x | 94 | 0.7492762597888434039130054534 | 1.0854670112202020295843295064 | 16.273046316027304423776223280 | — | — | — | — | not selected |
| atr_sl=2.5 | RSI 14/o5 age 12 SMA on (20) SL 2.5x TP 2.0x | 86 | 0.4498287589286729494223784133 | 1.0448740216814595771079927536 | 16.218186372258790179922181030 | — | — | — | — | not selected |
| atr_tp=3.0 | RSI 14/o5 age 12 SMA on (20) SL 1.5x TP 3.0x | 96 | 0.0618837541261006899992663382 | 1.0064358365881625280051234144 | 17.188627518696684681557443580 | — | — | — | — | not selected |
| atr_tp=4.0 | RSI 14/o5 age 12 SMA on (20) SL 1.5x TP 4.0x | 89 | 0.3946729603257565355125135688 | 1.0363797499790854233244123353 | 17.001735483949822841842481690 | 63 | -5.078658921718099057610802569 | 0.5327462001743584586193379677 | 14.609257669532246809702365760 | selected |
| baseline | RSI 14/o5 age 12 SMA on (20) SL 1.5x TP 2.0x | 106 | -0.3294729795845051765618648536 | 0.9603952313246748864201143333 | 20.579867000202170273738499980 | 111 | -1.0404840172951392282469820971 | 0.8586490441863132459905632911 | 17.503495901373234494194306140 | not selected |
| joint-13 | RSI 10/o5 age uncapped SMA on (20) SL 2.5x TP 4.0x | 57 | 4.9963629646896502938864068795 | 1.3797982013681379426871417104 | 18.201787006085755708658271600 | 47 | -3.0424747826336476022237138843 | 0.7919939347693540467087625813 | 15.989963734785018953244315420 | selected |
| joint-14 | RSI 21/o3 age 8 SMA on (20) SL 1.5x TP 4.0x | 84 | -1.4033916859067143431622995001 | 0.8762101197645212653247402656 | 23.493479025007721507202993750 | — | — | — | — | not selected |
| joint-15 | RSI 10/o5 age 4 SMA off (20) SL 2.5x TP 2.0x | 106 | -0.1438340171766780737449894181 | 0.9857475862612181863976043535 | 18.995958101557055824145926240 | — | — | — | — | not selected |
| joint-16 | RSI 10/o3 age 12 SMA off (20) SL 2.0x TP 2.0x | 143 | 1.0652585931716845789604978874 | 1.1267831446366953861262534338 | 14.703378280239423031801251320 | — | — | — | — | not selected |
| joint-17 | RSI 10/o7 age 8 SMA on (20) SL 1.5x TP 2.0x | 84 | 0.9035349634597174909348169804 | 1.1136388823855387382121763368 | 11.340711892984336140688686890 | — | — | — | — | not selected |
| joint-18 | RSI 21/o7 age 4 SMA on (20) SL 2.0x TP 3.0x | 37 | 0.8469152992561327439550929821 | 1.0697764572263309820675851285 | 11.094299595427166975777151010 | — | — | — | — | not selected |
| joint-19 | RSI 14/o7 age uncapped SMA off (20) SL 1.5x TP 3.0x | 99 | -2.9261745065886301432782514784 | 0.7336282530459734970583020577 | 23.534957168141826631069223520 | — | — | — | — | not selected |
| joint-20 | RSI 14/o5 age 4 SMA off (20) SL 2.5x TP 3.0x | 86 | -0.5661840667749781822371789719 | 0.9570479150401693846601225791 | 20.751732191475608876612874870 | — | — | — | — | not selected |
| joint-21 | RSI 14/o5 age 12 SMA off (20) SL 2.5x TP 2.0x | 106 | -2.9932561842302377698747531837 | 0.7469274448137533508399737855 | 25.093385336790075513666199020 | — | — | — | — | not selected |
| joint-22 | RSI 14/o3 age uncapped SMA off (20) SL 2.5x TP 3.0x | 80 | -3.4980249448798504742183466082 | 0.7567264115140798829753556171 | 28.251973715481880983085341440 | — | — | — | — | not selected |
| joint-23 | RSI 10/o3 age 12 SMA off (20) SL 1.5x TP 4.0x | 131 | 0.0601674917573083843038845325 | 1.005810790931224530310645774 | 22.742208153417056603958733410 | — | — | — | — | not selected |
| joint-24 | RSI 21/o5 age 12 SMA on (20) SL 2.0x TP 4.0x | 73 | 4.6191924142313521196903722903 | 1.4363069377715878701441616555 | 13.943966144209401488913389390 | 60 | -3.9043150207849903147032745025 | 0.6922820255552837907444450868 | 14.398000976078659072862482290 | selected |
| joint-25 | RSI 21/o3 age 8 SMA off (20) SL 2.5x TP 2.0x | 116 | -0.7167177197462878566749114221 | 0.9327459937765929881752011838 | 17.751775444220082865960328240 | — | — | — | — | not selected |
| joint-26 | RSI 21/o7 age uncapped SMA off (20) SL 2.0x TP 4.0x | 65 | 6.2241456162425368763042501042 | 1.6002615983203815519702856275 | 12.542152420101327181663738120 | — | — | — | — | not selected |
| joint-27 | RSI 10/o3 age uncapped SMA on (20) SL 1.5x TP 2.0x | 136 | -0.0029002449631814853692798817 | 0.9996392406505728303677205218 | 15.896383154930252215624679240 | — | — | — | — | not selected |
| joint-28 | RSI 10/o7 age 8 SMA off (20) SL 1.5x TP 4.0x | 110 | -0.4010030098351483430141416635 | 0.9638441554762715045597410463 | 28.487421562262055801387378800 | — | — | — | — | not selected |
| joint-29 | RSI 10/o5 age uncapped SMA on (20) SL 1.5x TP 2.0x | 125 | -0.0117388585977008699457556427 | 0.998546258818861940400742426 | 16.253690661330367412216099220 | — | — | — | — | not selected |
| max_age=4 | RSI 14/o5 age 4 SMA on (20) SL 1.5x TP 2.0x | 85 | 1.6240689380932051092813144993 | 1.2214382544615465837765928088 | 12.847498364384181139178986440 | 101 | -0.814170014744723530881009295 | 0.8882122921418198190183868919 | 14.275309785426725819500507730 | selected |
| max_age=8 | RSI 14/o5 age 8 SMA on (20) SL 1.5x TP 2.0x | 103 | 0.0142354837445346525758956947 | 1.0017601639022610542411650809 | 19.247652172601314480591616770 | — | — | — | — | not selected |
| max_age=uncapped | RSI 14/o5 age uncapped SMA on (20) SL 1.5x TP 2.0x | 108 | -0.5665963370812498550059302474 | 0.9326137701569556827039846487 | 20.579867000202170273738499980 | — | — | — | — | not selected |
| rsi_order=3 | RSI 14/o3 age 12 SMA on (20) SL 1.5x TP 2.0x | 123 | -0.6922617215299067311490110695 | 0.9180817624778258135116558958 | 22.980294733221312899386638640 | — | — | — | — | not selected |
| rsi_order=7 | RSI 14/o7 age 12 SMA on (20) SL 1.5x TP 2.0x | 71 | 0.3008503864017594073788240492 | 1.0350749490383885826456654672 | 18.173571935785662273390628800 | — | — | — | — | not selected |
| rsi_period=10 | RSI 10/o5 age 12 SMA on (20) SL 1.5x TP 2.0x | 124 | 0.1357322407292676548736996607 | 1.0169824833109591559587107276 | 16.253690661330367412216099220 | — | — | — | — | not selected |
| rsi_period=21 | RSI 21/o5 age 12 SMA on (20) SL 1.5x TP 2.0x | 98 | 0.5347624732410525768371459351 | 1.067989243084002678152307847 | 18.490406114933161986349510850 | — | — | — | — | not selected |
| sma_enabled=false | RSI 14/o5 age 12 SMA off (20) SL 1.5x TP 2.0x | 142 | -1.2910351838023836741604195526 | 0.8523651419793000157678349251 | 20.984221418835305596966035560 | — | — | — | — | not selected |

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
- atr_sl=2.0: rejected: validation net expectancy -2.5450301127617256890536733392 is not positive
- atr_sl=2.5: rejected: validation net expectancy -2.871746723518612414358229601 is not positive
- atr_tp=3.0: rejected: validation net expectancy -1.9136641418937114089828634845 is not positive
- atr_tp=4.0: rejected: validation net expectancy -2.0405978196687696168012087336 is not positive
- baseline: rejected: validation net expectancy -2.2670225776036455328615581383 is not positive
- joint-13: rejected: validation net expectancy -2.2024886680371142893000450969 is not positive
- joint-14: rejected: validation net expectancy -1.2230036462512042467260423942 is not positive
- joint-15: rejected: validation net expectancy -2.0573085314543378306422628067 is not positive
- joint-16: rejected: validation net expectancy -0.8244667472989844548162424831 is not positive
- joint-17: rejected: validation net expectancy -1.8501609686400524543307422444 is not positive
- joint-18: rejected: validation net expectancy -1.9819730808235320127573312424 is not positive
- joint-19: rejected: validation net expectancy -1.5543863705201693674060520129 is not positive
- joint-20: rejected: validation net expectancy -1.875580550025433047956462442 is not positive
- joint-21: rejected: validation net expectancy -1.8001000588106799145183931686 is not positive
- joint-22: rejected: validation net expectancy -1.5847257668934545794773939211 is not positive
- joint-23: rejected: validation net expectancy -0.6921429075149544348393847022 is not positive
- joint-24: rejected: validation net expectancy -2.3241345113366879653280136167 is not positive
- joint-25: rejected: validation net expectancy -1.6652132663574724668358794287 is not positive
- joint-26: rejected: validation net expectancy -1.4206338950783102904669775716 is not positive
- joint-27: rejected: validation net expectancy -1.2907504023818207284096996364 is not positive
- joint-28: rejected: validation net expectancy -2.0765818476200339155092829495 is not positive
- joint-29: rejected: validation net expectancy -1.8138214314622790507477512083 is not positive
- max_age=4: rejected: validation net expectancy -2.1335302131423574080135933129 is not positive
- max_age=8: rejected: validation net expectancy -2.1454435939397129771718356239 is not positive
- max_age=uncapped: rejected: validation net expectancy -2.0635313914860278140686276712 is not positive
- no candidate meets the criteria
- rsi_order=3: rejected: validation net expectancy -1.5682800294252363122873505127 is not positive
- rsi_order=7: rejected: validation net expectancy -1.8074167808717573984841567612 is not positive
- rsi_period=10: rejected: validation net expectancy -1.9358297351080501452790663989 is not positive
- rsi_period=21: rejected: validation net expectancy -1.8243019400176149726844216141 is not positive
- sma_enabled=false: rejected: validation net expectancy -1.9380920222398780857525872564 is not positive

### 4hour

Winners (in rank order): `joint-13`, `joint-24`, `max_age=4`, `atr_tp=4.0`.

Notes:
- atr_sl=2.0: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- atr_sl=2.5: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- atr_tp=3.0: rejected: only 1 of 3 symbols have positive net expectancy (need 2)
- atr_tp=4.0: flagged: negative stress validation net expectancy (-1.1057831629416540345154188549)
- baseline: rejected: validation net expectancy -0.3294729795845051765618648536 is not positive
- joint-14: rejected: validation net expectancy -1.4033916859067143431622995001 is not positive
- joint-15: rejected: validation net expectancy -0.1438340171766780737449894181 is not positive
- joint-16: rejected: only 0 of 3 folds have positive net expectancy (need 2)
- joint-17: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- joint-18: rejected: only 1 of 3 symbols have positive net expectancy (need 2)
- joint-19: rejected: validation net expectancy -2.9261745065886301432782514784 is not positive
- joint-20: rejected: validation net expectancy -0.5661840667749781822371789719 is not positive
- joint-21: rejected: validation net expectancy -2.9932561842302377698747531837 is not positive
- joint-22: rejected: validation net expectancy -3.4980249448798504742183466082 is not positive
- joint-23: rejected: ETH-USDT max drawdown 22.742208153417056603958733410% exceeds 20%
- joint-25: rejected: validation net expectancy -0.7167177197462878566749114221 is not positive
- joint-26: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- joint-27: rejected: validation net expectancy -0.0029002449631814853692798817 is not positive
- joint-28: rejected: validation net expectancy -0.4010030098351483430141416635 is not positive
- joint-29: rejected: validation net expectancy -0.0117388585977008699457556427 is not positive
- max_age=8: rejected: only 0 of 3 folds have positive net expectancy (need 2)
- max_age=uncapped: rejected: validation net expectancy -0.5665963370812498550059302474 is not positive
- rsi_order=3: rejected: validation net expectancy -0.6922617215299067311490110695 is not positive
- rsi_order=7: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- rsi_period=10: rejected: only 0 of 3 folds have positive net expectancy (need 2)
- rsi_period=21: rejected: only 1 of 3 folds have positive net expectancy (need 2)
- sma_enabled=false: rejected: validation net expectancy -1.2910351838023836741604195526 is not positive

## 6. Walk-forward

Expanding-fold validation metrics are pooled across symbols; per-symbol validation metrics are shown separately. Only the baseline and selected winners are shown.

### 1hour

#### Pooled folds

| Label | Fold | Net expectancy | Trades | Max DD % |
|---|---|---|---|---|
| baseline | fold1_val | -1.8932662982832060408765316219 | 300 | 29.565163225067810842068912910 |
| baseline | fold2_val | -1.7426516648164982311899810339 | 292 | 30.052973795115877473670026230 |
| baseline | fold3_val | -2.3353350869671003790047060824 | 174 | 16.365026229157088736838794840 |

#### Per-symbol validation

| Label | Symbol | Net expectancy | Trades | Max DD % |
|---|---|---|---|---|
| baseline | BTC-USDT | -1.6778519776353336943235621702 | 124 | 21.795280260533481279734897460 |
| baseline | ETH-USDT | -3.0307356652451166322308383412 | 116 | 40.998924304049322768782400240 |
| baseline | SOL-USDT | -2.1245128182427282893007782898 | 109 | 23.942971951937444961456598400 |

### 4hour

#### Pooled folds

| Label | Fold | Net expectancy | Trades | Max DD % |
|---|---|---|---|---|
| baseline | fold1_val | 1.7687643754482301274746918735 | 81 | 10.717922217609814304360708160 |
| baseline | fold2_val | -1.9174128493378701975161319428 | 71 | 14.526926509838916722396285900 |
| baseline | fold3_val | -1.6004697361397600138522910723 | 53 | 10.759918350491478834404959160 |
| joint-13 | fold1_val | 1.8090132424751863952753685042 | 55 | 22.408113805233686187095928720 |
| joint-13 | fold2_val | 4.6283011776702707499489480225 | 44 | 15.06638750877823722058798400 |
| joint-13 | fold3_val | 4.0541538913539012432378764938 | 37 | 13.508599716497664002638259530 |
| joint-24 | fold1_val | 2.245248797476758105920657272 | 46 | 17.649323242806632112471265460 |
| joint-24 | fold2_val | 1.6502670069470499028127380699 | 45 | 12.259921851771915482378571430 |
| joint-24 | fold3_val | 4.4431431829520874704934223707 | 44 | 8.113462603412853321917218110 |
| max_age=4 | fold1_val | 1.0289038116924160208012861679 | 68 | 12.191074857325265651139089960 |
| max_age=4 | fold2_val | -0.8824557195867124225886734575 | 64 | 12.091462864005235660845954620 |
| max_age=4 | fold3_val | 0.6980141623406680643240445233 | 41 | 6.1782320299530953362356007900 |
| atr_tp=4.0 | fold1_val | 0.9989527949991999322104087985 | 61 | 14.418603043861516580361925060 |
| atr_tp=4.0 | fold2_val | 0.9900527392662129399963376544 | 60 | 10.315124221422693526958870830 |
| atr_tp=4.0 | fold3_val | -1.2795473668926705725773537276 | 47 | 11.887228724714917084933241430 |

#### Per-symbol validation

| Label | Symbol | Net expectancy | Trades | Max DD % |
|---|---|---|---|---|
| baseline | BTC-USDT | -2.722271784017740543905597491 | 31 | 10.629423662189720445267061110 |
| baseline | ETH-USDT | 2.5503681853889269712659529542 | 38 | 10.759918350491478834404959160 |
| baseline | SOL-USDT | -1.2823703128699139665564963383 | 37 | 20.579867000202170273738499980 |
| joint-13 | BTC-USDT | -2.0358451696697733489714634408 | 13 | 11.452910310619487415087827920 |
| joint-13 | ETH-USDT | 4.9817641572518319293507746692 | 24 | 18.201787006085755708658271600 |
| joint-13 | SOL-USDT | 9.58481682094865769918678124 | 20 | 7.927979080772525946418389280 |
| joint-24 | BTC-USDT | 4.5873001985980185036020607688 | 25 | 7.6509259309681730175244191600 |
| joint-24 | ETH-USDT | 4.8776433678813306764841658661 | 28 | 12.144079760116165691164330510 |
| joint-24 | SOL-USDT | 4.297226348663049160289450686 | 20 | 13.943966144209401488913389390 |
| max_age=4 | BTC-USDT | -1.7602514441275210734459884414 | 28 | 8.849978024864396651798594720 |
| max_age=4 | ETH-USDT | 6.0764426097962069829849353352 | 29 | 6.1782320299530953362356007900 |
| max_age=4 | SOL-USDT | 0.3970023031929650656727244313 | 28 | 12.847498364384181139178986440 |
| atr_tp=4.0 | BTC-USDT | -3.4699003959992170873986045016 | 22 | 15.128127092547308365681911020 |
| atr_tp=4.0 | ETH-USDT | 1.8815447230608733256324961204 | 33 | 13.392821654669747941434064690 |
| atr_tp=4.0 | SOL-USDT | 1.4521390094107731716914892553 | 34 | 17.001735483949822841842481690 |

## 7. Parameter sensitivity

Each row groups candidates by a tested parameter value and reports the validation net expectancy range (pooled across symbols, base costs). Joint draws change several parameters at once, so these ranges are descriptive rather than causal.

### 1hour

| Parameter | Value | Candidates | Mean validation net expectancy | Min | Max |
|---|---|---|---|---|---|
| RSI period | 10 | 9 | -1.6381723599464030337638308363 | -2.2024886680371142893000450969 | -0.6921429075149544348393847022 |
| RSI period | 14 | 15 | -2.0072765435200781044795990653 | -2.871746723518612414358229601 | -1.5543863705201693674060520129 |
| RSI period | 21 | 6 | -1.739876723310803659133110978 | -2.3241345113366879653280136167 | -1.2230036462512042467260423942 |
| RSI order | 3 | 7 | -1.2640832523033038890559990111 | -1.6652132663574724668358794287 | -0.6921429075149544348393847022 |
| RSI order | 5 | 17 | -2.1030720013239198919717996184 | -2.871746723518612414358229601 | -1.8001000588106799145183931686 |
| RSI order | 7 | 6 | -1.7818588239256425731590904637 | -2.0765818476200339155092829495 | -1.4206338950783102904669775716 |
| Max age | 12 | 14 | -1.8823447220050220247176535579 | -2.871746723518612414358229601 | -0.6921429075149544348393847022 |
| Max age | 4 | 4 | -2.012098093861415074842412451 | -2.1335302131423574080135933129 | -1.875580550025433047956462442 |
| Max age | 8 | 5 | -1.7920806645616952121147565282 | -2.1454435939397129771718356239 | -1.2230036462512042467260423942 |
| Max age | uncapped | 7 | -1.7043339894084537314109353026 | -2.2024886680371142893000450969 | -1.2907504023818207284096996364 |
| SMA enabled | false | 11 | -1.5899301785285189443837198856 | -2.0765818476200339155092829495 | -0.6921429075149544348393847022 |
| SMA enabled | true | 19 | -1.9896172509668374967182216121 | -2.871746723518612414358229601 | -1.2230036462512042467260423942 |
| ATR SL multiplier | 1.5 | 18 | -1.7854754344326264372420124808 | -2.2670225776036455328615581383 | -0.6921429075149544348393847022 |
| ATR SL multiplier | 2.0 | 5 | -1.8192476694598480824844476506 | -2.5450301127617256890536733392 | -0.8244667472989844548162424831 |
| ATR SL multiplier | 2.5 | 7 | -2.008166223585300649012666638 | -2.871746723518612414358229601 | -1.5847257668934545794773939211 |
| ATR TP multiplier | 2.0 | 18 | -1.9112248070289025361286706225 | -2.871746723518612414358229601 | -0.8244667472989844548162424831 |
| ATR TP multiplier | 3.0 | 5 | -1.7820659820312600833160206206 | -1.9819730808235320127573312424 | -1.5543863705201693674060520129 |
| ATR TP multiplier | 4.0 | 7 | -1.7113690422152963941387078666 | -2.3241345113366879653280136167 | -0.6921429075149544348393847022 |

### 4hour

| Parameter | Value | Candidates | Mean validation net expectancy | Min | Max |
|---|---|---|---|---|---|
| RSI period | 10 | 9 | 0.7335089025816577367650154816 | -0.4010030098351483430141416635 | 4.9963629646896502938864068795 |
| RSI period | 14 | 15 | -0.6178792922041912838935038633 | -3.4980249448798504742183466082 | 1.6240689380932051092813144993 |
| RSI period | 21 | 6 | 1.6841510662196786861582750648 | -1.4033916859067143431622995001 | 6.2241456162425368763042501042 |
| RSI order | 3 | 7 | -0.7411243188709925610442094374 | -3.4980249448798504742183466082 | 1.0652585931716845789604978874 |
| RSI order | 5 | 17 | 0.4516410953323942579094768744 | -2.9932561842302377698747531837 | 4.9963629646896502938864068795 |
| RSI order | 7 | 6 | 0.8247114581560613387134318290 | -2.9261745065886301432782514784 | 6.2241456162425368763042501042 |
| Max age | 12 | 14 | 0.2189713759681974963675385335 | -2.9932561842302377698747531837 | 4.6191924142313521196903722903 |
| Max age | 4 | 4 | 0.4402415383494203993135597728 | -0.5661840667749781822371789719 | 1.6240689380932051092813144993 |
| Max age | 8 | 5 | -0.3206683936567796798681279821 | -1.4033916859067143431622995001 | 0.9035349634597174909348169804 |
| Max age | uncapped | 7 | 0.6021533841173677631961561608 | -3.4980249448798504742183466082 | 6.2241456162425368763042501042 |
| SMA enabled | false | 11 | -0.4715143574420604252394872522 | -3.4980249448798504742183466082 | 6.2241456162425368763042501042 |
| SMA enabled | true | 19 | 0.6644713194396203772140311342 | -1.4033916859067143431622995001 | 4.9963629646896502938864068795 |
| ATR SL multiplier | 1.5 | 18 | -0.1997036575561510066638662573 | -2.9261745065886301432782514784 | 1.6240689380932051092813144993 |
| ATR SL multiplier | 2.0 | 5 | 2.7009576365381099445646437434 | 0.7492762597888434039130054534 | 6.2241456162425368763042501042 |
| ATR SL multiplier | 2.5 | 7 | -0.3531178870271013019201991873 | -3.4980249448798504742183466082 | 4.9963629646896502938864068795 |
| ATR TP multiplier | 2.0 | 18 | -0.0539036193974107593505187054 | -2.9932561842302377698747531837 | 1.6240689380932051092813144993 |
| ATR TP multiplier | 3.0 | 5 | -1.2163168929722450731558835476 | -3.4980249448798504742183466082 | 0.8469152992561327439550929821 |
| ATR TP multiplier | 4.0 | 7 | 2.0700209645006773605029980303 | -1.4033916859067143431622995001 | 6.2241456162425368763042501042 |

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

## 10. Recommendation

**insufficient evidence**

No candidate met the validation criteria for every requested timeframe, so the screening produced no evidence strong enough to change the baseline. This is a provisional screening result, not a profitability claim; a larger or differently scoped experiment would be needed before revising defaults.

