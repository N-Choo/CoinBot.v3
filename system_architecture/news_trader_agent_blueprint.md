# News Trader Agent — Full System Blueprint

## 1. System Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          NEWS TRADER SYSTEM                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                       DATA INGESTION LAYER                          │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌───────────────────┐   │   │
│  │  │ NewsAPI  │  │ RSS Feed │  │ Reddit   │  │ Financial Stmts   │   │   │
│  │  │ REST     │  │ (Yahoo,  │  │ (r/wsb,  │  │ (SEC, quarterly   │   │   │
│  │  │          │  │  Reuters)│  │  r/stocks)│  │  reports)         │   │   │
│  │  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────────┬──────────┘   │   │
│  └───────┼──────────────┼─────────────┼──────────────────┼──────────────┘   │
│          └──────────────┼─────────────┼──────────────────┘                  │
│                         ▼             ▼                                     │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                      NORMALIZATION LAYER                            │   │
│  │  • Strip HTML/noise   • Translate → EN   • Deduplicate articles     │   │
│  │  • Extract: ticker, date, text, source                               │   │
│  │  • Output: Standardized Article object                               │   │
│  └────────────────────────────┬─────────────────────────────────────────┘   │
│                               ▼                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                     PROCESSING LAYER                                │   │
│  │                                                                     │   │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────────┐    │   │
│  │  │ Sentiment       │  │ Time Decay      │  │ Source Trust     │    │   │
│  │  │ Analyze         │  │ Weight           │  │ Weight           │    │   │
│  │  │                 │  │                 │  │                  │    │   │
│  │  │ VADER = fast    │  │ 0-1 scale       │  │ Reuters  = 1.0   │    │   │
│  │  │ FinBERT=precise │  │ newer = near 1  │  │ Reddit   = 0.3   │    │   │
│  │  │ LLM   =best     │  │                 │  │ Twitter  = 0.5   │    │   │
│  │  └────────┬────────┘  └────────┬────────┘  └────────┬─────────┘    │   │
│  └───────────┼────────────────────┼─────────────────────┼──────────────┘   │
│              ▼                    ▼                     ▼                   │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                      AGGREGATION LAYER                              │   │
│  │                                                                     │   │
│  │  weighted_score = Σ( sent × time_weight × source_weight )          │   │
│  │                           ─────────────────────                     │   │
│  │                         Σ( time_weight × source_weight )            │   │
│  │                                                                     │   │
│  │  confidence = abs(weighted_score) × article_volume_factor           │   │
│  │  volume_factor = min(article_count / 10, 1.0)                       │   │
│  └────────────────────────────┬─────────────────────────────────────────┘   │
│                               ▼                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                      DECISION LAYER                                 │   │
│  │                                                                     │   │
│  │  if score >  threshold:    SIGNAL = BUY                             │   │
│  │  if score < -threshold:    SIGNAL = SELL                            │   │
│  │  else:                     SIGNAL = HOLD                            │   │
│  │                                                                     │   │
│  │  if confidence < min_conf: SIGNAL = HOLD (uncertain)                │   │
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

## 2. Data Flow Diagram (Fetch → Signal)

```
Timeline for one article:

FETCH:
  NewsAPI ──→ GET /everything?q=AAPL&from=2026-05-15
       │
       ▼
  Response: { articles: [ { title, description, source, publishedAt } ] }
       │
       ▼
NORMALIZE:
  article = {
      ticker: "AAPL",
      title: "Apple stock jumps on strong earnings",
      text: "Apple Inc...",                         ← title + description
      source: "reuters",                            ← normalized name
      timestamp: 2026-05-15T10:30:00Z,
      url: "https://..."
      age_hours: 2.5                                ← calculated from timestamp
  }
       │
       ▼
ANALYZE SENTIMENT:
  VADER("Apple stock jumps on strong earnings")
      → compound: 0.85
      → pos: 0.72, neu: 0.28, neg: 0.00
       │
       ▼
CALCULATE WEIGHTS:
  time_weight = exp(-0.1 × age_hours)              ← exponential decay
              = exp(-0.1 × 2.5)
              = 0.779

  source_weight = SOURCE_WEIGHTS["reuters"] = 1.0   ← lookup table
       │
       ▼
COMPUTE ARTICLE SCORE:
  article_score = 0.85 × 0.779 × 1.0 = 0.662
       │
       ▼
AGGREGATE (all articles in last N hours):
                    Σ(article_score)
  final_score = ─────────────────────
                Σ(time_weight × source_weight)

  confidence = min(article_count / 10, 1.0) × abs(final_score)
       │
       ▼
DECISION:
  if final_score > 0.15 and confidence > 0.3:
      signal = "BUY"
      reason = f"Positive sentiment ({final_score:.2f})"
  elif final_score < -0.15 and confidence > 0.3:
      signal = "SELL"
      reason = f"Negative sentiment ({final_score:.2f})"
  else:
      signal = "HOLD"
      reason = f"Weak signal ({final_score:.2f}, conf={confidence:.2f})"
```

---

## 3. UML Diagrams

### 3a. Use Case Diagram

```
┌──────────────────────────────────────────────────┐
│              News Trader System                   │
│                                                    │
│  ┌─────────────┐                                   │
│  │  User       │                                   │
│  └──────┬──────┘                                   │
│         │                                          │
│         │  (actor)                                 │
│         │                                          │
│         │  ┌───────────────────────────────────┐   │
│         ├──│ 1. Configure ticker + threshold   │   │
│         │  └───────────────────────────────────┘   │
│         │  ┌───────────────────────────────────┐   │
│         ├──│ 2. Run sentiment scan (manual)    │   │
│         │  └───────────────────────────────────┘   │
│         │  ┌───────────────────────────────────┐   │
│         ├──│ 3. View signal + score            │   │
│         │  └───────────────────────────────────┘   │
│         │  ┌───────────────────────────────────┐   │
│         ├──│ 4. View article details           │   │
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
│  │ NewsAPI (external)│  │ Yahoo Finance   │        │
│  └──────────────────┘  └──────────────────┘        │
└────────────────────────────────────────────────────┘
```

### 3b. Class Diagram

```
┌────────────────────────────────────────────────────┐
│  Article                                           │
├────────────────────────────────────────────────────┤
│  - ticker: str                                     │
│  - title: str                                      │
│  - text: str                                       │
│  - source: str                                     │
│  - timestamp: datetime                             │
│  - url: str                                        │
├────────────────────────────────────────────────────┤
│  + age_hours() → float                             │
│  + age_minutes() → float                           │
│  + is_recent(hours: int) → bool                    │
└────────────────────────────────────────────────────┘
           ▲
           │
┌────────────────────────────────────────────────────┐
│  ScoredArticle extends Article                     │
├────────────────────────────────────────────────────┤
│  - sentiment: float         # -1 .. +1              │
│  - time_weight: float       # 0 .. 1                │
│  - source_weight: float     # 0 .. 1                │
│  - final_score: float       # sentiment × weights   │
└────────────────────────────────────────────────────┘
           │
           ▼
┌────────────────────────────────────────────────────┐
│  NewsTraderEngine                                   │
├────────────────────────────────────────────────────┤
│  - ticker: str                                      │
│  - lookback_hours: int                              │
│  - threshold: float                                 │
│  - min_confidence: float                            │
│  - api_key: str                                     │
│  - source_weights: dict                             │
├────────────────────────────────────────────────────┤
│  + fetch_articles() → list[Article]                 │
│  + analyze(article) → ScoredArticle                 │
│  + aggregate(scores) → Signal                       │
│  + run() → Signal                                   │
└────────────────────────────────────────────────────┘
           │
           ▼
┌────────────────────────────────────────────────────┐
│  Signal                                            │
├────────────────────────────────────────────────────┤
│  - direction: str              # BUY/SELL/HOLD      │
│  - score: float                                     │
│  - confidence: float                                │
│  - article_count: int                               │
│  - timestamp: datetime                              │
│  - reason: str                                      │
├────────────────────────────────────────────────────┤
│  + to_dict() → dict                                 │
│  + to_emoji() → str                                 │
└────────────────────────────────────────────────────┘
           │
           ▼
┌────────────────────────────────────────────────────┐
│  Backtester                                         │
├────────────────────────────────────────────────────┤
│  - ticker: str                                      │
│  - start_date: date                                 │
│  - end_date: date                                   │
│  - engine: NewsTraderEngine                         │
├────────────────────────────────────────────────────┤
│  - simulate_day(date) → Signal                      │
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
│  - trades: list[Trade]                              │
│  + summary() → str                                  │
└────────────────────────────────────────────────────┘
```

### 3c. Sequence Diagram (one full run)

```
User        NewsTraderEngine      NewsAPI         VADER/LLM        User Output
 │                  │                │                │                │
 │  1. run()        │                │                │                │
 │─────────────────▶│                │                │                │
 │                  │  2. GET articles                 │                │
 │                  │───────────────▶│                │                │
 │                  │◀───────────────│                │                │
 │                  │  3. Article[]  │                │                │
 │                  │                │                │                │
 │                  │  loop for each article:          │                │
 │                  │  ────────────────────────────────│                │
 │                  │  4. analyze(article)             │                │
 │                  │─────────────────────────────────▶│                │
 │                  │    5. return sentiment (-1..+1)  │                │
 │                  │◀─────────────────────────────────│                │
 │                  │                │                │                │
 │                  │  6. aggregate()                  │                │
 │                  │  ── compute weighted avg ──      │                │
 │                  │  ── compute confidence ──        │                │
 │                  │  ── apply threshold ──           │                │
 │                  │                │                │                │
 │  7. return Signal                                  │                │
 │◀────────────────│                │                │                │
 │                  │                │                │                │
 │  8. Show result  │                │                │                │
 │◀───────────────────────────────────────────────────────────────────│
 │                  │                │                │                │
```

### 3d. Component Diagram (microservices)

```
┌──────────────────────────────────────────────────────────────────┐
│                        MESSAGE QUEUE (RabbitMQ / Redis)           │
│                                                                    │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐   │
│  │ articles │    │  scored  │    │ signals  │    │ trades   │   │
│  │  .queue  │    │ .queue   │    │ .queue   │    │ .queue   │   │
│  └──────────┘    └──────────┘    └──────────┘    └──────────┘   │
└──────────────────────────────────────────────────────────────────┘
         ▲               ▲               ▲               ▲
         │               │               │               │
┌────────┴──────┐  ┌─────┴──────┐  ┌────┴──────┐  ┌────┴──────────┐
│  Fetcher      │  │ Analyzer   │  │ Decider   │  │ Executor      │
│  Service      │  │ Service    │  │ Service   │  │ Service       │
│  (Python)     │  │ (Python)   │  │ (Python)  │  │ (Python/Rust) │
│               │  │            │  │           │  │               │
│  Fetches      │  │ VADER/     │  │ Aggregates│  │ Alpaca API    │
│  news every   │  │ FinBERT/   │  │ signal    │  │ Telegram bot  │
│  N minutes    │  │ LLM        │  │           │  │               │
└───────────────┘  └────────────┘  └───────────┘  └───────────────┘
```

---

## 4. Database Schema (SQLite)

```sql
CREATE TABLE articles (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    ticker      TEXT NOT NULL,
    title       TEXT NOT NULL,
    text        TEXT,
    source      TEXT NOT NULL,
    url         TEXT UNIQUE NOT NULL,
    published_at DATETIME NOT NULL,
    fetched_at  DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE scores (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    article_id  INTEGER REFERENCES articles(id),
    sentiment   REAL NOT NULL,
    time_weight REAL NOT NULL,
    source_weight REAL NOT NULL,
    final_score REAL NOT NULL,
    analyzer    TEXT DEFAULT 'vader',
    created_at  DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE signals (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    ticker      TEXT NOT NULL,
    direction   TEXT NOT NULL,
    score       REAL NOT NULL,
    confidence  REAL NOT NULL,
    article_count INTEGER NOT NULL,
    reason      TEXT,
    created_at  DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE trades (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    ticker      TEXT NOT NULL,
    signal_id   INTEGER REFERENCES signals(id),
    direction   TEXT NOT NULL,
    entry_price REAL NOT NULL,
    exit_price  REAL,
    entry_date  DATETIME NOT NULL,
    exit_date   DATETIME,
    pnl         REAL,
    created_at  DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

---

## 5. Project Structure

```
news_trader/
├── README.md
├── requirements.txt
├── config.yaml                  # ticker, thresholds, API keys
├── main.py                      # CLI entry point
│
├── data/
│   ├── fetcher.py               # NewsAPI, RSS, Reddit
│   ├── normalizer.py            # Clean, deduplicate, structure
│   └── database.py              # SQLite CRUD
│
├── analysis/
│   ├── sentiment.py             # VADER / FinBERT / LLM wrapper
│   ├── weights.py               # Time decay + source weights
│   └── aggregation.py           # Weighted average + confidence
│
├── decision/
│   ├── threshold.py             # Threshold logic
│   └── signal.py                # Signal object
│
├── execution/
│   ├── backtester.py            # Historical simulation
│   ├── trader.py                # Live trading (Alpaca)
│   └── alerter.py               # Telegram / Discord bot
│
├── config/
│   ├── settings.py              # Load config.yaml
│   └── constants.py             # SOURCE_WEIGHTS, THRESHOLDS
│
└── tests/
    ├── test_fetcher.py
    ├── test_sentiment.py
    ├── test_aggregation.py
    ├── test_backtester.py
    └── test_integration.py
```

---

## 6. Implementation Phases (MVP → Full)

| Phase | What | Estimate |
|-------|------|----------|
| **1** | `Article` + `Signal` data classes | 1 hour |
| **2** | `fetcher.py` — NewsAPI + yfinance price | 2 hours |
| **3** | `sentiment.py` — VADER | 1 hour |
| **4** | `weights.py` + `aggregation.py` | 1 hour |
| **5** | `threshold.py` → Signal with BUY/SELL/HOLD | 1 hour |
| **6** | `main.py` — CLI that runs everything | 1 hour |
| **7** | `database.py` — log to SQLite | 2 hours |
| **8** | `backtester.py` — simulate over historical data | 4 hours |
| **9** | `alerter.py` — Telegram notifications | 2 hours |
| **10** | `trader.py` — Alpaca API for live trading | 4 hours |

**MVP = phases 1–6** (~7 hours).

---

## 7. Sentiment Methods Comparison

| Method | Speed | Accuracy | Needs API? | When to Use |
|--------|-------|----------|------------|-------------|
| **VADER** | 1000 art/sec | 60-70% | No | Prototyping, high volume |
| **FinBERT** | 100 art/sec | 80-85% | No (local) | Better accuracy, still fast |
| **GPT-4 / Claude** | 5 art/sec | 90-95% | Yes ($) | Best quality, low volume |

```python
# VADER (free, local, fast)
from vaderSentiment.vaderSentiment import SentimentIntensityAnalyzer
analyzer = SentimentIntensityAnalyzer()
s = analyzer.polarity_scores("Apple beats earnings expectations")
print(s["compound"])  # 0.85

# FinBERT (free, local, slower)
from transformers import pipeline
finbert = pipeline("sentiment-analysis", model="ProsusAI/finbert")
result = finbert("Apple beats earnings expectations")
print(result)  # [{'label': 'positive', 'score': 0.98}]

# GPT-4 (paid, API)
import openai
resp = openai.chat.completions.create(
    model="gpt-4",
    messages=[
        {"role": "system", "content": "Classify sentiment for a financial news headline. Reply with only one number: -1, -0.5, 0, 0.5, or 1"},
        {"role": "user", "content": "Apple beats earnings expectations"}
    ]
)
print(resp.choices[0].message.content)  # "1"
```

---

## 8. Common Pitfalls

1. **Too few articles** — 1-2 articles give noisy signals. Wait until >=5.
2. **Stale news** — an article from 3 days ago about a price move that already happened
3. **Market prices in quickly** — neutral news → 0 score → already priced in
4. **Fake news / hype** — Reddit pumping gives false positive signals
5. **Overfitting in backtest** — perfect backtest, poor live performance. Use out-of-sample data.
