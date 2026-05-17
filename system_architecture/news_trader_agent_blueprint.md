# Trader Agent — Full System Blueprint

## 1. System Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          TRADER AGENT SYSTEM                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                      DATA INGESTION LAYER                           │   │
│  │  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐   │   │
│  │  │ yfinance         │  │ CCXT / Binance   │  │ Alpaca / IBKR    │   │   │
│  │  │ (stocks/ETFs)    │  │ (crypto OHLCV)   │  │ (live quotes)    │   │   │
│  │  └────────┬─────────┘  └────────┬─────────┘  └────────┬─────────┘   │   │
│  └───────────┼─────────────────────┼──────────────────────┼─────────────┘   │
│              └─────────────────────┼──────────────────────┘                 │
│                                    ▼                                        │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                      INDICATOR LAYER                                │   │
│  │                                                                     │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌───────────────────┐   │   │
│  │  │ Momentum │  │ Trend    │  │ Volatility│  │ Volume            │   │   │
│  │  │ RSI,Stoch│  │ MACD,    │  │ BB, ATR,  │  │ OBV, VWAP, Vol   │   │   │
│  │  │ Will %R  │  │ SMA/EMA  │  │ Keltner   │  │ Profile, Ratio    │   │   │
│  │  │          │  │ ADX      │  │           │  │                   │   │   │
│  │  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────────┬──────────┘   │   │
│  └───────┼──────────────┼─────────────┼──────────────────┼──────────────┘   │
│          │              │              │                  │                  │
│   Multi-TF ratios  Market regime  Cross-sectional        │                  │
│   (1d ÷ 4h RSI,    (SPY correlation,                    │                  │
│    MACD, vol, ADX)  SPY trend)                          │                  │
│          │              │              │                  │                  │
│          └──────────────┼─────────────┼──────────────────┘                  │
│                         ▼             ▼                                     │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                      DECISION LAYER (ML + AI Hybrid)                 │   │
│  │                                                                     │   │
│  │  ┌──────────────────────────────────────────────────────────────┐   │   │
│  │  │  STEP 1: ML SCREENER (XGBoost / LightGBM)                    │   │   │
│  │  │  ─────────────────────────────────────────────────────       │   │   │
│  │  │  Input:  RSI, MACD, MA, ADX, BB, ATR, volume,            │   │   │
│  │  │          multi-TF ratios, market regime, price action     │   │   │
│  │  │  Trained on: historical indicator → direction labels         │   │   │
│  │  │  Output:  probability(LONG), probability(SHORT) + flag       │   │   │
│  │  │                                                               │   │   │
│  │  │  ▸ Fast, deterministic, runs every ticker every N minutes    │   │   │
│  │  │  ▸ Filters 500+ tickers → top 3-5 high-conviction setups    │   │   │
│  │  └──────────────────────────────────────────────────────────────┘   │   │
│  │                               ▼                                     │   │
│  │  ┌──────────────────────────────────────────────────────────────┐   │   │
│  │  │  STEP 2: AI REVIEWER (LLM — GPT-4 / Claude)                   │   │   │
│  │  │  ─────────────────────────────────────────────────────       │   │   │
│  │  │  Context fed to LLM:                                         │   │   │
│  │  │    • ML prediction + probabilities                           │   │   │
│  │  │    • Full indicator snapshot (multi-timeframe)               │   │   │
│  │  │    • Market regime context (SPY corr, SPY trend)             │   │   │
│  │  │                                                               │   │   │
│  │  │  Structured output:                                          │   │   │
│  │  │    { decision: LONG|SHORT|WAIT, confidence: 0.0-1.0,         │   │   │
│  │  │      reasoning: "...", entry_zones: [...], stop_loss: ... }  │   │   │
│  │  │                                                               │   │   │
│  │  │  ▸ Slow, expensive — only runs on flagged setups             │   │   │
│  │  │  ▸ Adds reasoning, context, and sanity check                 │   │   │
│  │  └──────────────────────────────────────────────────────────────┘   │   │
│  └────────────────────────────┬─────────────────────────────────────────┘   │
│                               ▼                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                      EXECUTION LAYER                                │   │
│  │                                                                     │   │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────────┐    │   │
│  │  │ Backtest        │  │ Live Trade       │  │ Alert / Log      │    │   │
│  │  │ (historical sim)│  │ (Alpaca/IBKR)    │  │ (Telegram/       │    │   │
│  │  │                 │  │                  │  │  Discord)        │    │   │
│  │  └─────────────────┘  └─────────────────┘  └──────────────────┘    │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Data Flow Diagram (Indicators → ML → AI → Signal)

```
  ═══════════════════════════════════════════════════════════════════════
  SINGLE PIPELINE: INDICATORS → ML SCREENER → AI FINAL REVIEW
  ═══════════════════════════════════════════════════════════════════════

  FETCH PRICE DATA:
    yfinance ──→ AAPL 1y daily OHLCV
         │
         ▼
  COMPUTE INDICATORS:
    rsi_14          = RSI(close, 14)               → 28.4
    stoch_k         = Stochastic %K(14,3,3)         → 12.5
    macd_line       = MACD line(12,26)              → -2.10
    macd_signal     = MACD signal(9)                → -3.30
    macd_histogram  = MACD histogram                 → +1.20
    sma_dist_50     = (close - SMA50) / SMA50        → -2.3%
    sma_dist_200    = (close - SMA200) / SMA200      → +5.8%
    adx_14          = ADX(high, low, close, 14)      → 31.5
    bb_position     = (close - BB_lower) / (BB_upper - BB_lower) → 0.12
    atr_pct         = ATR(14) / close                → 2.8%
    volume_ratio    = vol_today / vol_20d_avg        → 1.8
    obv_change_5d   = (OBV_today - OBV_5d_ago) / OBV_5d_ago → +4.2%
    price_change_1d  = (close / close_1d_ago) - 1    → -1.5%
    price_change_5d  = (close / close_5d_ago) - 1    → -7.2%
    price_change_20d = (close / close_20d_ago) - 1   → -4.1%
    volatility_20d   = std(ret, 20)                  → 2.1%
    rsi_1d_div_4h    = 28.4 / 42.1                  → 0.67 (daily more oversold)
    macd_1d_div_4h   = -2.10 / 1.15                 → -1.83 (daily bearish, 4h bullish)
    adx_1d_div_4h    = 31.5 / 18.2                  → 1.73 (daily trend stronger)
    corr_to_spy_20d  = pearsonr(ret, spy_ret, 20)   → 0.72
    spy_trend_20d    = SPY 20d return               → +1.2%
         │
         ▼
  FEATURE VECTOR (one row per ticker per timeframe):
    [rsi_14, stoch_k, stoch_d, williams_r,
     macd_line, macd_signal, macd_histogram,
     sma_dist_50, sma_dist_200, ema_dist_20, adx_14,
     bb_position, atr_pct, volatility_20d,
     volume_ratio, obv_change_5d,
     price_change_1d, price_change_5d, price_change_20d,
     rsi_1d_div_4h, macd_1d_div_4h, vol_1d_div_4h, adx_1d_div_4h,
     corr_to_spy_20d, spy_trend_20d]


  ═══════════════════════════════════════════════════════════════════════
  ML SCREENER
  ═══════════════════════════════════════════════════════════════════════

  ML MODEL (XGBoost trained on 5+ years of data):
    feature_vector ──→ model.predict_proba()
    → { LONG: 0.72, SHORT: 0.08, WAIT: 0.20 }

  FILTER:
    If max_prob > 0.65 and prob > 2× second-best → FLAG for AI review
    Else → SILENT WAIT (no action)

  Result: 500 tickers scanned → 3 flagged for AI review


  ═══════════════════════════════════════════════════════════════════════
  AI REVIEW (only on flagged setups)
  ═══════════════════════════════════════════════════════════════════════

  PROMPT TO LLM:
    ┌─────────────────────────────────────────────────────────────────┐
    │ System: You are a trading analyst. Review the ML-predicted     │
    │ setup and decide: LONG, SHORT, or WAIT. Return JSON.           │
    │                                                                 │
    │ Indicators (raw values, no pre-interpretation):                  │
    │   RSI(14)   = 28.4                                             │
    │   Stoch %K  = 12.5                                             │
    │   MACD line / signal / histogram = -2.10 / -3.30 / +1.20       │
    │   SMA 50/200 dist = -2.3% / +5.8%                               │
    │   ADX(14)  = 31.5                                              │
    │   Bollinger pos = 0.12                                         │
    │   Volume ratio = 1.8x                                          │
    │   OBV 5d    = +4.2%                                            │
    │   ATR(14)   = 2.8%                                             │
    │   Returns: 1d=-1.5%, 5d=-7.2%, 20d=-4.1%                       │
    │   Volatility(20d) = 2.1%                                        │
    │   RSI 1d/4h = 0.67  MACD 1d/4h = -1.83  ADX 1d/4h = 1.73     │
    │   SPY corr  = 0.72   SPY 20d = +1.2%                            │
    │                                                                 │
    │ ML prediction: LONG (72% confidence)                            │
    │                                                                 │
    │ Return JSON: {decision, confidence, reasoning, entry_zone,     │
    │               stop_loss_pct, take_profit_pct, risks}            │
    └─────────────────────────────────────────────────────────────────┘

  LLM RESPONSE:
    {
      "decision": "LONG",
      "confidence": 0.82,
      "reasoning": "Triple oversold confluence (RSI 28, Stoch 12.5,
                    BB 0.12) with bullish MACD divergence and
                    positive OBV despite the -7.2% drawdown.
                    Golden cross still active. High probability
                    mean-reversion bounce. ML agrees at 72%.
                    Low VIX supportive for upside.",
      "entry_zone": [182.00, 184.50],
      "stop_loss_pct": 3.0,
      "take_profit_pct": 8.0,
      "risks": ["Momentum could continue downward if support at 180 breaks",
                "Low volume on recovery would invalidate the setup"]
    }
```

---

## 3. UML Diagrams

### 3a. Use Case Diagram

```
┌──────────────────────────────────────────────────┐
│              Trader Agent System                   │
│                                                    │
│  ┌─────────────┐                                   │
│  │  User       │                                   │
│  └──────┬──────┘                                   │
│         │                                          │
│         │  (actor)                                 │
│         │                                          │
│         │  ┌───────────────────────────────────┐   │
│         ├──│ 1. Configure tickers + thresholds │   │
│         │  └───────────────────────────────────┘   │
│         │  ┌───────────────────────────────────┐   │
│         ├──│ 2. Run indicator scan             │   │
│         │  └───────────────────────────────────┘   │
│         │  ┌───────────────────────────────────┐   │
│         ├──│ 3. View signal + confidence       │   │
│         │  └───────────────────────────────────┘   │
│         │  ┌───────────────────────────────────┐   │
│         ├──│ 4. View indicator breakdown       │   │
│         │  └───────────────────────────────────┘   │
│         │  ┌───────────────────────────────────┐   │
│         ├──│ 5. Run backtest                   │   │
│         │  └───────────────────────────────────┘   │
│         │  ┌───────────────────────────────────┐   │
│         └──│ 6. Enable auto-trading            │   │
│            └───────────────────────────────────┘   │
│                                                    │
│  System also uses:                                 │
│  ┌──────────────────┐  ┌──────────────────┐        │
│  │ yfinance (OHLCV) │  │ Alpaca / IBKR    │        │
│  └──────────────────┘  └──────────────────┘        │
└────────────────────────────────────────────────────┘
```

### 3b. Class Diagram

```
┌────────────────────────────────────────────────────┐
│  FeatureVector                                     │
├────────────────────────────────────────────────────┤
│  - ticker: str                                     │
│  - timestamp: datetime                             │
│  - rsi: float                                      │
│  - macd_line: float                                │
│  - macd_signal: float                              │
│  - macd_histogram: float                           │
│  - sma_dist_50: float                              │
│  - sma_dist_200: float                             │
│  - bb_position: float                              │
│  - atr_pct: float                                  │
│  - volume_ratio: float                             │
│  - price_change_5d: float                          │
│  - price_change_20d: float                         │
│  - ... (15-20 dimensions total)                    │
├────────────────────────────────────────────────────┤
│  + to_array() → list[float]                        │
│  + to_dict() → dict                                │
└────────────────────────────────────────────────────┘
           │
           ▼
┌────────────────────────────────────────────────────┐
│  TraderEngine                                      │
├────────────────────────────────────────────────────┤
│  - tickers: list[str]                              │
│  - model: XGBClassifier                            │
│  - min_confidence: float                           │
│  - max_ai_calls: int                               │
├────────────────────────────────────────────────────┤
│  + fetch_ohlcv(ticker) → DataFrame                 │
│  + compute_indicators(ohlcv) → FeatureVector       │
│  + screen(features) → MLPrediction                 │
│  + ai_review(setup) → AIDecision                   │
│  + merge(ml, ai) → Signal                          │
│  + run() → list[Signal]                            │
└────────────────────────────────────────────────────┘
           │
           ▼
┌────────────────────────────────────────────────────┐
│  Signal                                            │
├────────────────────────────────────────────────────┤
│  - ticker: str                                     │
│  - direction: str              # LONG/SHORT/WAIT    │
│  - ml_confidence: float                             │
│  - ai_confidence: float                             │
│  - final_confidence: float                          │
│  - timestamp: datetime                              │
│  - reason: str                                      │
│  - entry_zone: tuple[float, float]                  │
│  - stop_loss_pct: float                             │
│  - take_profit_pct: float                           │
├────────────────────────────────────────────────────┤
│  + to_dict() → dict                                 │
│  + to_emoji() → str                                 │
└────────────────────────────────────────────────────┘
           │
           ▼
┌────────────────────────────────────────────────────┐
│  Backtester                                         │
├────────────────────────────────────────────────────┤
│  - tickers: list[str]                               │
│  - start_date: date                                 │
│  - end_date: date                                   │
│  - engine: TraderEngine                             │
├────────────────────────────────────────────────────┤
│  - simulate_day(date) → list[Signal]                │
│  + run() → BacktestResult                           │
└────────────────────────────────────────────────────┘
           │
           ▼
┌────────────────────────────────────────────────────┐
│  BacktestResult                                    │
├────────────────────────────────────────────────────┤
│  - total_trades: int                                │
│  - win_rate: float                                  │
│  - total_return: float                              │
│  - max_drawdown: float                              │
│  - sharpe_ratio: float                              │
│  - trades: list[Trade]                              │
│  + summary() → str                                  │
│  + plot_equity_curve() → None                       │
└────────────────────────────────────────────────────┘
```

### 3c. Sequence Diagram (one full run)

```
User        TraderEngine       yfinance        ML Model        AI (LLM)
 │                │                │                │               │
 │  1. run()      │                │                │               │
 │───────────────▶│                │                │               │
 │                │  2. fetch OHLCV (loop tickers)  │               │
 │                │───────────────▶│                │               │
 │                │◀───────────────│                │               │
 │                │  DataFrame     │                │               │
 │                │                │                │               │
 │                │ 3. compute_indicators()         │               │
 │                │ ── RSI, MACD, BB, ATR etc. ──   │               │
 │                │                │                │               │
 │                │ 4. screen(feature_vector)       │               │
 │                │────────────────────────────────▶│               │
 │                │  5. predict_proba()             │               │
 │                │◀────────────────────────────────│               │
 │                │  {LONG:0.72, WAIT:0.20, SHORT:0.08}            │
 │                │                │                │               │
 │                │ 6. Filter: keep if prob > 0.65  │               │
 │                │                │                │               │
 │                │ 7. ai_review(setup) [only flagged]              │
 │                │────────────────────────────────────────────────▶│
 │                │  8. JSON {decision, confidence, reasoning}      │
 │                │◀────────────────────────────────────────────────│
 │                │                │                │               │
 │                │ 9. merge(ml_pred, ai_decision)  │               │
 │                │ ── apply decision matrix ──     │               │
 │                │                │                │               │
 │ 10. return Signal[]                                           │
 │◀───────────────│                │                │               │
 │                │                │                │               │
```

### 3d. Component Diagram (microservices)

```
┌──────────────────────────────────────────────────────────────────┐
│                        MESSAGE QUEUE (RabbitMQ / Redis)          │
│                                                                  │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐    │
│  │ indicator│    │ ml       │    │ signals  │    │ trades   │    │
│  │ .queue   │    │ .queue   │    │ .queue   │    │ .queue   │    │
│  └──────────┘    └──────────┘    └──────────┘    └──────────┘    │
└──────────────────────────────────────────────────────────────────┘
         ▲               ▲               ▲               ▲
         │               │               │               │
┌────────┴──────┐  ┌─────┴──────┐  ┌────┴──────┐  ┌────┴──────────┐
│  Indicator    │  │ ML Screener│  │ AI Reviewer│  │ Executor      │
│  Service      │  │ Service    │  │ Service    │  │ Service       │
│  (Python)     │  │ (Python)   │  │ (Python)   │  │ (Python/Rust) │
│               │  │            │  │            │  │               │
│  Fetches      │  │ XGBoost    │  │ GPT-4 /    │  │ Alpaca/IBKR   │
│  OHLCV +      │  │ predict on │  │ Claude API │  │ Telegram/     │
│  computes all │  │ all active │  │ on flagged │  │ Discord alert │
│  indicators   │  │ tickers    │  │ setups     │  │               │
└───────────────┘  └────────────┘  └────────────┘  └───────────────┘
```

---

## 4. Database Schema (SQLite)

```sql
CREATE TABLE indicator_snapshots (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    ticker          TEXT NOT NULL,
    rsi_14          REAL NOT NULL,
    macd_line       REAL NOT NULL,
    macd_signal     REAL NOT NULL,
    macd_histogram  REAL NOT NULL,
    sma_dist_50     REAL NOT NULL,
    sma_dist_200    REAL NOT NULL,
    bb_position     REAL NOT NULL,
    atr_pct         REAL NOT NULL,
    volume_ratio    REAL NOT NULL,
    price_change_5d REAL NOT NULL,
    price_change_20d REAL NOT NULL,
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE ml_predictions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    ticker          TEXT NOT NULL,
    snapshot_id     INTEGER REFERENCES indicator_snapshots(id),
    prob_long       REAL NOT NULL,
    prob_short      REAL NOT NULL,
    prob_wait       REAL NOT NULL,
    flagged         INTEGER DEFAULT 0,    -- 0=no, 1=yes (sent to AI)
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE ai_reviews (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    ticker          TEXT NOT NULL,
    prediction_id   INTEGER REFERENCES ml_predictions(id),
    decision        TEXT NOT NULL,         -- LONG, SHORT, WAIT
    confidence      REAL NOT NULL,
    reasoning       TEXT,
    entry_low       REAL,
    entry_high      REAL,
    stop_loss_pct   REAL,
    take_profit_pct REAL,
    risks           TEXT,                  -- JSON array
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE signals (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    ticker          TEXT NOT NULL,
    direction       TEXT NOT NULL,         -- LONG, SHORT, WAIT
    ml_confidence   REAL NOT NULL,
    ai_confidence   REAL,
    final_confidence REAL NOT NULL,
    reason          TEXT,
    entry_zone_low  REAL,
    entry_zone_high REAL,
    stop_loss_pct   REAL,
    take_profit_pct REAL,
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE trades (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    ticker          TEXT NOT NULL,
    signal_id       INTEGER REFERENCES signals(id),
    direction       TEXT NOT NULL,
    entry_price     REAL NOT NULL,
    exit_price      REAL,
    entry_date      DATETIME NOT NULL,
    exit_date       DATETIME,
    pnl             REAL,
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

---

## 5. Implementation Phases

| Phase  | What                                                     | Estimate |
| ------ | -------------------------------------------------------- | -------- |
| **1**  | Indicator module — RSI, MACD, BB, ATR, OBV, Stoch, etc.  | 3 hours  |
| **2**  | `FeatureVector` + `Signal` + `Trade` data classes        | 1 hour   |
| **3**  | OHLCV fetcher — yfinance / CCXT wrapper                  | 1 hour   |
| **4**  | Feature builder — merge all indicators into vector       | 1 hour   |
| **5**  | Training pipeline — label 5+ years OHLCV → train XGBoost | 3 hours  |
| **6**  | ML screener — scan all tickers, flag top setups          | 2 hours  |
| **7**  | AI reviewer — structured LLM prompt + JSON parse         | 2 hours  |
| **8**  | Merge logic — ML prediction + AI review → final decision | 1 hour   |
| **9**  | `database.py` — log signals/trades to SQLite             | 2 hours  |
| **10** | Full pipeline integration + backtest                     | 3 hours  |
| **11** | `alerter.py` — Telegram / Discord notifications          | 2 hours  |
| **12** | `trader.py` — Alpaca/IBKR API with AI veto gate          | 4 hours  |

**Total estimate: ~25 hours** (4-5 days full-time)

---

## 6. Common Pitfalls

1. **ML lookahead bias** — labels use future returns, but features must only use data available AT THAT TIME. No future indicator values in training.
2. **Overfitting in backtest** — perfect backtest, poor live performance. Always validate on out-of-sample data.
3. **LLM hallucination** — model may invent indicator values. Always validate LLM output against actual data before executing.
4. **Drift between train and live** — market regime changes (bull → bear, high vol → low vol). Retrain ML model quarterly minimum.
5. **AI cost spiral** — if too many tickers get flagged, API costs explode. Hard cap at 5 LLM calls per cycle.
6. **Too few indicators** — 1-2 indicators give noisy signals. The model needs 15+ dimensions to find real patterns.
7. **Ignoring macro context** — a perfect LONG setup on a stock means nothing if SPY is crashing. Always include broad market context in the AI reviewer prompt.
8. **No stop-loss discipline** — ML and AI will be wrong sometimes. Always execute the stop-loss the AI recommends.
