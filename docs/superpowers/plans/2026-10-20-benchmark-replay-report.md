# Benchmark Replay Report Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `make benchmark-web` command that turns the analyzer benchmark into a self-contained HTML report replaying candles, RSI divergences, signals, trades, and account state one candle at a time.

**Architecture:** The existing `run_benchmark` loop stays the single source of truth. A `ReplayRecorder` observes it, `build_ticker_replay` assembles a JSON-safe payload, and `report.py` injects that payload into a static HTML template. No new runtime dependencies and no network requests from the report.

**Tech Stack:** Python 3.13, pandas, scipy (existing), vanilla HTML/CSS/JS + SVG, pytest.

**Spec:** `docs/superpowers/specs/2026-10-20-benchmark-replay-report-design.md`

---

## File Map

| Path | Responsibility |
| --- | --- |
| `process/analyzer/models/types.py` | `Signal.divergence_idx` optional field. |
| `process/analyzer/signals/generator.py` | Populate `divergence_idx` from the selected divergence. |
| `process/analyzer/benchmark/engine.py` | `ClosedTrade.tp` / `ClosedTrade.sl`. |
| `process/analyzer/benchmark/replay.py` | Date helper, account snapshot, `ReplayRecorder`, `build_ticker_replay`, `config_dict`, JSON sanitizing. |
| `process/analyzer/benchmark/report.py` | Render/write the HTML report. |
| `process/analyzer/benchmark/replay_template.html` | Self-contained viewer. |
| `process/analyzer/benchmark/main.py` | Recorder hook, `--web` / `--out`, failure capture. |
| `process/analyzer/benchmark/tests/test_replay.py` | Recorder and payload tests. |
| `process/analyzer/benchmark/tests/test_report.py` | Rendering, escaping, file-write tests. |
| `process/analyzer/benchmark/tests/test_benchmark.py` | Add web-mode CLI test. |
| `Makefile` | `benchmark-web` target. |
| `.gitignore` | Ignore the generated `out/` directory. |
| `process/analyzer/README.md` | Usage docs. |

All Python commands below assume `cd process/analyzer` (that is what `make analyzer-test` does). Run `git` commands from the repo root.

---

### Task 1: Expose the selected divergence pivot on `Signal`

**Files:**
- Modify: `process/analyzer/models/types.py`
- Modify: `process/analyzer/signals/generator.py`
- Test: `process/analyzer/models/tests/test_types.py`
- Test: `process/analyzer/signals/tests/test_generator.py`

- [ ] **Step 1: Write the failing model test**

Append to `process/analyzer/models/tests/test_types.py`:

```python
def test_signal_divergence_idx_defaults_to_none_and_is_not_published():
    s = Signal(
        ticker="BTC-USDT",
        action="buy",
        confidence=0.6,
        entry_price=100.0,
        reason="regular_bullish_divergence",
    )
    assert s.divergence_idx is None
    assert "divergence_idx" not in s.to_dict()
```

- [ ] **Step 2: Write the failing generator test**

Append to `process/analyzer/signals/tests/test_generator.py`:

```python
def test_generate_signals_exposes_divergence_index(monkeypatch):
    import analyzer.signals.generator as gen

    n = 50
    close = [100.0] * n
    df = _make_df(close)
    result = pd.DataFrame(
        {
            "High": df["High"],
            "Low": df["Low"],
            "Close": df["Close"],
            "RSI": [50.0] * n,
            "Reg_Bullish_Div": [0] * n,
            "Reg_Bearish_Div": [0] * n,
            "Hid_Bullish_Div": [0] * n,
            "Hid_Bearish_Div": [0] * n,
        }
    )
    result.loc[42, "Reg_Bullish_Div"] = 1
    monkeypatch.setattr(gen, "detect_rsi_divergences", lambda *a, **k: result)

    signals = gen.generate_signals(df, ticker="TEST-USDT")

    assert len(signals) == 1
    assert signals[0].divergence_idx == 42
```

- [ ] **Step 3: Run the tests to verify they fail**

Run:

```bash
cd process/analyzer
python3 -m pytest models/tests/test_types.py::test_signal_divergence_idx_defaults_to_none_and_is_not_published signals/tests/test_generator.py::test_generate_signals_exposes_divergence_index -v
```

Expected: FAIL — `Signal` has no `divergence_idx` attribute.

- [ ] **Step 4: Implement the field and assignment**

In `process/analyzer/models/types.py`, add the field after `tp_price`:

```python
    tp_price: float = 0.0
    divergence_idx: int | None = None
```

Do **not** add it to `to_dict()`.

In `process/analyzer/signals/generator.py`, replace the final `return` block with:

```python
    return [
        Signal(
            ticker=ticker,
            action=best[1],
            confidence=best[3],
            entry_price=latest_close,
            reason=best[2],
            sl_price=sl_price,
            tp_price=tp_price,
            divergence_idx=best[0],
        )
    ]
```

- [ ] **Step 5: Run the tests to verify they pass**

Run:

```bash
cd process/analyzer
python3 -m pytest models/tests/test_types.py signals/tests/test_generator.py -v
```

Expected: PASS (all tests in both files).

- [ ] **Step 6: Commit**

```bash
git add process/analyzer/models/types.py process/analyzer/models/tests/test_types.py process/analyzer/signals/generator.py process/analyzer/signals/tests/test_generator.py
git commit -m "feat(analyzer): expose divergence pivot index on signals"
```

---

### Task 2: Capture replay events from `run_benchmark`

**Files:**
- Modify: `process/analyzer/benchmark/engine.py`
- Create: `process/analyzer/benchmark/replay.py`
- Modify: `process/analyzer/benchmark/main.py`
- Test: `process/analyzer/benchmark/tests/test_engine.py`
- Create: `process/analyzer/benchmark/tests/test_replay.py`

- [ ] **Step 1: Write the failing `ClosedTrade` test**

Append to `process/analyzer/benchmark/tests/test_engine.py`:

```python
def test_closed_trade_keeps_tp_and_sl():
    e = SimEngine(init_fund=1000.0, fee_rate=0.0)
    e.on_signal("buy", 100.0, 110.0, 90.0, 0, "d0")
    e.on_candle(1, high=111.0, low=99.0, date="d1")
    t = e.closed[0]
    assert t.tp == pytest.approx(110.0)
    assert t.sl == pytest.approx(90.0)
```

- [ ] **Step 2: Run it to verify it fails**

```bash
cd process/analyzer
python3 -m pytest benchmark/tests/test_engine.py::test_closed_trade_keeps_tp_and_sl -v
```

Expected: FAIL — `ClosedTrade` has no `tp`.

- [ ] **Step 3: Add `tp` / `sl` to `ClosedTrade`**

In `process/analyzer/benchmark/engine.py`, add fields at the end of `ClosedTrade`:

```python
    exit_idx: int
    exit_date: str
    tp: float = 0.0
    sl: float = 0.0
```

In `SimEngine._close`, add them to the constructor call:

```python
            exit_idx=idx,
            exit_date=date,
            tp=pos.tp,
            sl=pos.sl,
        )
```

- [ ] **Step 4: Run the engine tests**

```bash
cd process/analyzer
python3 -m pytest benchmark/tests/test_engine.py -v
```

Expected: PASS (27 tests).

- [ ] **Step 5: Write the failing recorder tests**

Create `process/analyzer/benchmark/tests/test_replay.py`:

```python
import numpy as np
import pandas as pd
import pytest

from analyzer.benchmark.main import run_benchmark
from analyzer.benchmark.replay import ReplayRecorder
from analyzer.models.types import Signal


def _df(n=80):
    close = np.full(n, 100.0)
    return pd.DataFrame(
        {"Open": close, "High": close + 1, "Low": close - 1, "Close": close}
    )


def _run(df, monkeypatch, signal_at=None, divergence_idx=None):
    import analyzer.benchmark.main as bm

    def fake_signals(df_slice, **kwargs):
        if signal_at is not None and len(df_slice) != signal_at + 1:
            return []
        entry = float(df_slice["Close"].iloc[-1])
        return [
            Signal(
                ticker="TEST-USDT",
                action="buy",
                confidence=0.6,
                entry_price=entry,
                reason="regular_bullish_divergence",
                sl_price=entry * 0.9,
                tp_price=entry * 1.1,
                divergence_idx=divergence_idx,
            )
        ]

    monkeypatch.setattr(bm, "generate_signals", fake_signals)
    recorder = ReplayRecorder()
    trades, open_positions, summary = run_benchmark(
        df,
        ticker="TEST-USDT",
        rsi_period=1,
        atr_period=1,
        sma_enabled=False,
        benchmark_days=len(df),
        init_fund=1000.0,
        fee_rate=0.0,
        recorder=recorder,
    )
    return recorder, trades, open_positions, summary


def test_events_align_with_replay_window(monkeypatch):
    df = _df(80)
    recorder, _, _, _ = _run(df, monkeypatch, signal_at=40)
    assert recorder.start == 1
    assert [e.i for e in recorder.events] == list(range(recorder.start, len(df)))


def test_position_is_not_evaluated_on_entry_candle(monkeypatch):
    df = _df(80)
    df.loc[40, ["High", "Low"]] = [200.0, 1.0]
    df.loc[41, ["High", "Low"]] = [200.0, 1.0]
    recorder, trades, open_positions, _ = _run(df, monkeypatch, signal_at=40)

    by_i = {e.i: e for e in recorder.events}
    assert by_i[40].exits == []
    assert [t.exit_idx for t in by_i[41].exits] == [41]
    assert len(trades) == 1
    assert trades[0]["entry_idx"] == 40
    assert trades[0]["exit_idx"] == 41
    assert open_positions == []


def test_candle_can_close_and_open_on_the_same_bar(monkeypatch):
    df = _df(80)
    df["High"] = 200.0
    df["Low"] = 1.0
    recorder, _, _, _ = _run(df, monkeypatch)

    event = next(e for e in recorder.events if e.exits and e.signal is not None)
    assert event.exits[0].exit_idx == event.i
    assert event.opened is True


def test_final_snapshot_matches_summary(monkeypatch):
    df = _df(80)
    recorder, _, _, summary = _run(df, monkeypatch, signal_at=10)
    final = recorder.events[-1].snapshot
    assert final["equity"] == pytest.approx(summary["equity"])
    assert final["realized_pnl"] == pytest.approx(summary["realized_pnl"])
    assert final["unrealized_pnl"] == pytest.approx(summary["unrealized_pnl"])


def test_opened_signals_match_trades_and_open_positions(monkeypatch):
    df = _df(80)
    recorder, trades, open_positions, _ = _run(df, monkeypatch)
    opened = sum(1 for e in recorder.events if e.opened)
    assert opened == len(trades) + len(open_positions)
```

- [ ] **Step 6: Run the new tests to verify they fail**

```bash
cd process/analyzer
python3 -m pytest benchmark/tests/test_replay.py -v
```

Expected: FAIL — `No module named 'analyzer.benchmark.replay'` and `run_benchmark() got an unexpected keyword argument 'recorder'`.

- [ ] **Step 7: Create `replay.py` with the recorder and account helper**

Create `process/analyzer/benchmark/replay.py`:

```python
"""Replay capture for the benchmark HTML report."""

from __future__ import annotations

import math
from dataclasses import dataclass
from datetime import datetime
from typing import Any

import pandas as pd

from analyzer.benchmark.engine import ClosedTrade, SimEngine
from analyzer.models.types import Signal


def date_str(df: pd.DataFrame, i: int) -> str:
    """Format the timestamp at position ``i`` for display."""
    ts = df["Timestamp"].iloc[i] if "Timestamp" in df.columns else df.index[i]
    if isinstance(ts, (datetime, pd.Timestamp)):
        return ts.strftime("%Y-%m-%d")
    return str(ts)


def account_state(engine: SimEngine, last_close: float) -> dict[str, Any]:
    """Current account snapshot, valued at ``last_close``."""
    unrealized = 0.0
    for pos in engine.positions:
        ret = (last_close - pos.entry_price) / pos.entry_price
        if pos.side == "sell":
            ret = -ret
        unrealized += pos.E * ret

    reserved = sum(p.E for p in engine.positions)
    realized = sum(t.pnl for t in engine.closed)
    return {
        "realized_pnl": realized,
        "unrealized_pnl": unrealized,
        "open_count": len(engine.positions),
        "available_fund": engine.available_fund,
        "equity": engine.available_fund + reserved + unrealized,
    }


@dataclass
class CandleEvent:
    """One replayed candle: exits, the evaluated signal, and account state."""

    i: int
    signal: Signal | None
    opened: bool
    rejected: str | None
    exits: list[ClosedTrade]
    snapshot: dict[str, Any]


class ReplayRecorder:
    """Observe one `run_benchmark` replay without changing its behavior."""

    def __init__(self) -> None:
        self.start: int | None = None
        self.events: list[CandleEvent] = []
        self.finished = False

    def begin(self, start: int, df: pd.DataFrame) -> None:
        self.start = start

    def record(
        self,
        i: int,
        last_close: float,
        signal: Signal | None,
        opened: bool,
        rejected: str | None,
        exits: list[ClosedTrade],
        engine: SimEngine,
    ) -> None:
        self.events.append(
            CandleEvent(
                i=i,
                signal=signal,
                opened=opened,
                rejected=rejected,
                exits=list(exits),
                snapshot=account_state(engine, last_close),
            )
        )

    def finish(self, engine: SimEngine) -> None:
        self.finished = True
```

- [ ] **Step 8: Hook the recorder into `run_benchmark` and `_summarize`**

In `process/analyzer/benchmark/main.py`:

Replace the local `_date_str` definition with this import near the other analyzer imports:

```python
from analyzer.benchmark.replay import (
    ReplayRecorder,
    account_state,
    date_str as _date_str,
)
```

Change `run_benchmark`'s signature:

```python
def run_benchmark(
    df: pd.DataFrame,
    ticker: str,
    *,
    rsi_period: int = 14,
    rsi_order: int = 5,
    price_diff_threshold: float = 0.05,
    sma_enabled: bool = False,
    sma_period: int = 20,
    atr_period: int = 14,
    atr_sl_multiplier: float = 2.0,
    atr_tp_multiplier: float = 3.0,
    atr_pct_high: float = 5.0,
    atr_high_adjust: float = 1.5,
    atr_sl_max_pct: float = 0.0,
    init_fund: float = 1000.0,
    fee_rate: float = 0.001,
    benchmark_days: int = 90,
    recorder: ReplayRecorder | None = None,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]], dict[str, Any]]:
```

Replace everything from `engine = SimEngine(...)` through the `return trades, open_positions, _summarize(ticker, engine, df)` line with:

```python
    engine = SimEngine(init_fund=init_fund, fee_rate=fee_rate)

    if recorder is not None:
        recorder.begin(start, df)

    for i in range(start, len(df)):
        row = df.iloc[i]
        before_closed = len(engine.closed)
        engine.on_candle(
            i, float(row["High"]), float(row["Low"]), _date_str(df, i)
        )
        exits = engine.closed[before_closed:]

        signal = None
        opened = False
        rejected = None

        signals = generate_signals(
            df.iloc[: i + 1],
            ticker=ticker,
            rsi_period=rsi_period,
            rsi_order=rsi_order,
            price_diff_threshold=price_diff_threshold,
            sma_enabled=sma_enabled,
            sma_period=sma_period,
            atr_period=atr_period,
            atr_sl_multiplier=atr_sl_multiplier,
            atr_tp_multiplier=atr_tp_multiplier,
            atr_pct_high=atr_pct_high,
            atr_high_adjust=atr_high_adjust,
            atr_sl_max_pct=atr_sl_max_pct,
        )
        if signals:
            sig = signals[0]
            signal = sig
            if sig.tp_price <= 0 or sig.sl_price <= 0:
                rejected = "invalid_tp_sl"
                logger.warning(
                    "skip %s signal on %s - non-positive tp/sl (%.4f/%.4f)",
                    ticker,
                    _date_str(df, i),
                    sig.tp_price,
                    sig.sl_price,
                )
            else:
                opened = engine.on_signal(
                    sig.action,
                    sig.entry_price,
                    sig.tp_price,
                    sig.sl_price,
                    i,
                    _date_str(df, i),
                )

        if recorder is not None:
            recorder.record(
                i,
                float(row["Close"]),
                signal,
                opened,
                rejected,
                exits,
                engine,
            )

    if recorder is not None:
        recorder.finish(engine)

    trades = [asdict(t) for t in engine.closed]
    open_positions = [asdict(p) for p in engine.positions]
    return trades, open_positions, _summarize(ticker, engine, df)
```

Replace `_summarize` with:

```python
def _summarize(
    ticker: str, engine: SimEngine, df: pd.DataFrame
) -> dict[str, Any]:
    last_close = float(df["Close"].iloc[-1])
    state = account_state(engine, last_close)
    total = len(engine.closed)
    wins = sum(1 for t in engine.closed if t.pnl > 0)

    return {
        "ticker": ticker,
        "trades": total,
        "wins": wins,
        "win_rate": (wins / total * 100) if total else 0.0,
        **state,
    }
```

- [ ] **Step 9: Run the recorder and existing benchmark tests**

```bash
cd process/analyzer
python3 -m pytest benchmark/tests/test_replay.py benchmark/tests/test_engine.py benchmark/tests/test_benchmark.py -v
```

Expected: PASS.

- [ ] **Step 10: Commit**

```bash
git add process/analyzer/benchmark/engine.py process/analyzer/benchmark/replay.py process/analyzer/benchmark/main.py process/analyzer/benchmark/tests/test_engine.py process/analyzer/benchmark/tests/test_replay.py
git commit -m "feat(analyzer): record benchmark replay events"
```

---

### Task 3: Assemble the JSON-safe replay payload

**Files:**
- Modify: `process/analyzer/benchmark/replay.py`
- Test: `process/analyzer/benchmark/tests/test_replay.py`

- [ ] **Step 1: Write the failing payload tests**

Append to `process/analyzer/benchmark/tests/test_replay.py`:

```python
from types import SimpleNamespace

from analyzer.benchmark.replay import (
    _json_safe,
    build_ticker_replay,
    config_dict,
)


CONFIG = {
    "timeframe": "1day",
    "rsi_period": 1,
    "rsi_order": 2,
    "price_diff_threshold": 0.05,
    "sma_enabled": False,
    "sma_period": 20,
    "atr_period": 1,
    "sl_mult": 2.0,
    "tp_mult": 3.0,
    "atr_pct_high": 5.0,
    "atr_high_adjust": 1.5,
    "atr_sl_max_pct": 0.0,
    "init_fund": 1000.0,
    "fee_rate": 0.0,
    "benchmark_days": 200,
}


def _payload(df, monkeypatch, signal_at=None, divergence_idx=None):
    recorder, trades, open_positions, summary = _run(
        df, monkeypatch, signal_at=signal_at, divergence_idx=divergence_idx
    )
    return build_ticker_replay(
        df,
        "TEST-USDT",
        start=recorder.start,
        recorder=recorder,
        trades=trades,
        open_positions=open_positions,
        summary=summary,
        config=CONFIG,
        warmup=0,
    )


def test_payload_contract_and_alignment(monkeypatch):
    df = _df(80)
    replay = _payload(df, monkeypatch, signal_at=40)
    recorder_start = replay["start_index"]

    for key in (
        "ticker",
        "config",
        "warmup",
        "start_index",
        "candles",
        "rsi",
        "sma",
        "divergences",
        "signals",
        "events",
        "trades",
        "open_positions",
        "summary",
    ):
        assert key in replay

    n = len(replay["candles"])
    assert n == len(df) - recorder_start
    assert len(replay["rsi"]) == n
    assert len(replay["sma"]) == n
    assert len(replay["events"]) == n
    assert replay["candles"][0]["t"] == str(recorder_start)
    assert replay["events"][0]["i"] == recorder_start


def test_divergence_marker_uses_pivot_index(monkeypatch):
    df = _df(80)
    replay = _payload(df, monkeypatch, signal_at=40, divergence_idx=5)
    assert replay["divergences"] == [
        {"i": 5, "detected_at": 40, "kind": "regular_bull", "side": "buy"}
    ]


def test_signal_without_divergence_idx_has_no_marker(monkeypatch):
    df = _df(80)
    replay = _payload(df, monkeypatch, signal_at=40)
    assert replay["divergences"] == []


def test_json_safe_converts_non_finite_floats():
    cleaned = _json_safe({"a": float("nan"), "b": [float("inf"), 1.0]})
    assert cleaned == {"a": None, "b": [None, 1.0]}


def test_config_dict_maps_config_fields():
    cfg = SimpleNamespace(
        kucoin_timeframe="1day",
        rsi_period=14,
        rsi_divergence_order=5,
        price_diff_threshold=0.05,
        sma_enabled=True,
        sma_period=200,
        atr_period=14,
        atr_sl_multiplier=2.8,
        atr_tp_multiplier=3.5,
        atr_pct_high=5.0,
        atr_high_adjust=1.5,
        atr_sl_max_pct=0.1,
        benchmark_init_fund=1000.0,
        benchmark_fee_rate=0.001,
        benchmark_days=90,
    )
    mapped = config_dict(cfg)
    assert mapped["timeframe"] == "1day"
    assert mapped["sma_enabled"] is True
    assert mapped["sl_mult"] == 2.8
    assert mapped["tp_mult"] == 3.5
    assert mapped["benchmark_days"] == 90
```

- [ ] **Step 2: Run them to verify they fail**

```bash
cd process/analyzer
python3 -m pytest benchmark/tests/test_replay.py -v
```

Expected: FAIL — `build_ticker_replay`, `config_dict`, and `_json_safe` are not defined.

- [ ] **Step 3: Add the payload builder and helpers**

Append to `process/analyzer/benchmark/replay.py`:

```python
REASON_KIND = {
    "regular_bullish_divergence": "regular_bull",
    "regular_bearish_divergence": "regular_bear",
    "hidden_bullish_divergence": "hidden_bull",
    "hidden_bearish_divergence": "hidden_bear",
}


def config_dict(cfg: Any) -> dict[str, Any]:
    """Subset of analyzer config embedded in the report."""
    return {
        "timeframe": cfg.kucoin_timeframe,
        "rsi_period": cfg.rsi_period,
        "rsi_order": cfg.rsi_divergence_order,
        "price_diff_threshold": cfg.price_diff_threshold,
        "sma_enabled": cfg.sma_enabled,
        "sma_period": cfg.sma_period,
        "atr_period": cfg.atr_period,
        "sl_mult": cfg.atr_sl_multiplier,
        "tp_mult": cfg.atr_tp_multiplier,
        "atr_pct_high": cfg.atr_pct_high,
        "atr_high_adjust": cfg.atr_high_adjust,
        "atr_sl_max_pct": cfg.atr_sl_max_pct,
        "init_fund": cfg.benchmark_init_fund,
        "fee_rate": cfg.benchmark_fee_rate,
        "benchmark_days": cfg.benchmark_days,
    }


def _finite_or_none(value: Any) -> float | None:
    try:
        number = float(value)
    except (TypeError, ValueError):
        return None
    return number if math.isfinite(number) else None


def _json_safe(value: Any) -> Any:
    """Convert numpy scalars and non-finite floats into JSON-safe values."""
    if isinstance(value, dict):
        return {str(k): _json_safe(v) for k, v in value.items()}
    if isinstance(value, (list, tuple)):
        return [_json_safe(v) for v in value]
    if isinstance(value, float):
        return value if math.isfinite(value) else None
    if isinstance(value, bool) or value is None or isinstance(value, (int, str)):
        return value
    item = getattr(value, "item", None)
    if callable(item):
        return _json_safe(item())
    return value


def build_ticker_replay(
    df: pd.DataFrame,
    ticker: str,
    *,
    start: int,
    recorder: ReplayRecorder,
    trades: list[dict[str, Any]],
    open_positions: list[dict[str, Any]],
    summary: dict[str, Any],
    config: dict[str, Any],
    warmup: int,
) -> dict[str, Any]:
    """Assemble one ticker's replay payload from a completed run."""
    from analyzer.indicators.rsi import calculate_rsi
    from analyzer.indicators.sma import calculate_sma

    window = df.iloc[start:]
    n = len(window)
    dates = [date_str(df, start + k) for k in range(n)]
    open_series = window["Open"] if "Open" in window.columns else window["Close"]

    rsi_series = calculate_rsi(df["Close"], period=int(config["rsi_period"]))
    rsi = [_finite_or_none(rsi_series.iloc[start + k]) for k in range(n)]

    if config.get("sma_enabled"):
        sma_series = calculate_sma(
            df["Close"], period=int(config["sma_period"])
        )
        sma = [_finite_or_none(sma_series.iloc[start + k]) for k in range(n)]
    else:
        sma = [None] * n

    candles = [
        {
            "t": dates[k],
            "o": float(open_series.iloc[k]),
            "h": float(window["High"].iloc[k]),
            "l": float(window["Low"].iloc[k]),
            "c": float(window["Close"].iloc[k]),
        }
        for k in range(n)
    ]

    events = []
    signals = []
    divergences = []
    seen_divergences: set[tuple[int, str]] = set()

    for event in recorder.events:
        snap = event.snapshot
        sig = event.signal
        events.append(
            {
                "i": event.i,
                "exits": [
                    {
                        "side": t.side,
                        "reason": t.reason,
                        "price": t.exit_price,
                        "pnl": t.pnl,
                        "pnl_pct": t.pnl_pct,
                        "entry_i": t.entry_idx,
                        "exit_i": t.exit_idx,
                    }
                    for t in event.exits
                ],
                "signal": None
                if sig is None
                else {
                    "action": sig.action,
                    "opened": event.opened,
                    "rejected": event.rejected,
                },
                "equity": snap["equity"],
                "realized": snap["realized_pnl"],
                "unrealized": snap["unrealized_pnl"],
                "available": snap["available_fund"],
                "open": snap["open_count"],
            }
        )

        if sig is None:
            continue

        signals.append(
            {
                "i": event.i,
                "action": sig.action,
                "confidence": sig.confidence,
                "reason": sig.reason,
                "entry": sig.entry_price,
                "tp": sig.tp_price,
                "sl": sig.sl_price,
                "opened": event.opened,
                "rejected": event.rejected,
            }
        )

        kind = REASON_KIND.get(sig.reason)
        pivot = sig.divergence_idx
        if pivot is None or kind is None or pivot < start:
            continue
        key = (pivot, kind)
        if key in seen_divergences:
            continue
        seen_divergences.add(key)
        divergences.append(
            {
                "i": pivot,
                "detected_at": event.i,
                "kind": kind,
                "side": sig.action,
            }
        )

    trade_payload = [
        {
            "side": t["side"],
            "entry_i": t["entry_idx"],
            "exit_i": t["exit_idx"],
            "entry_date": t["entry_date"],
            "exit_date": t["exit_date"],
            "entry": t["entry_price"],
            "exit": t["exit_price"],
            "tp": t["tp"],
            "sl": t["sl"],
            "pnl": t["pnl"],
            "pnl_pct": t["pnl_pct"],
            "reason": t["reason"],
        }
        for t in trades
    ]

    open_payload = [
        {
            "side": p["side"],
            "entry_i": p["entry_idx"],
            "entry_date": p["entry_date"],
            "entry": p["entry_price"],
            "tp": p["tp"],
            "sl": p["sl"],
            "E": p["E"],
        }
        for p in open_positions
    ]

    return _json_safe(
        {
            "ticker": ticker,
            "config": config,
            "warmup": warmup,
            "start_index": start,
            "candles": candles,
            "rsi": rsi,
            "sma": sma,
            "divergences": divergences,
            "signals": signals,
            "events": events,
            "trades": trade_payload,
            "open_positions": open_payload,
            "summary": summary,
        }
    )
```

- [ ] **Step 4: Run the replay tests**

```bash
cd process/analyzer
python3 -m pytest benchmark/tests/test_replay.py -v
```

Expected: PASS (10 tests).

- [ ] **Step 5: Commit**

```bash
git add process/analyzer/benchmark/replay.py process/analyzer/benchmark/tests/test_replay.py
git commit -m "feat(analyzer): build benchmark replay payload"
```

---

### Task 4: Render the self-contained HTML report

**Files:**
- Create: `process/analyzer/benchmark/report.py`
- Create: `process/analyzer/benchmark/replay_template.html`
- Create: `process/analyzer/benchmark/tests/test_report.py`

- [ ] **Step 1: Write the failing report tests**

Create `process/analyzer/benchmark/tests/test_report.py`:

```python
import json
import re

import pytest

from analyzer.benchmark.report import render_report, write_report


def _embedded(html: str) -> dict:
    match = re.search(
        r'<script id="replay-data" type="application/json">(.*?)</script>',
        html,
        re.S,
    )
    assert match is not None
    return json.loads(match.group(1))


def test_render_report_embeds_parseable_json():
    report = {
        "generated_at": "2026-10-20T12:00:00",
        "tickers": [{"ticker": "BTC-USDT"}],
        "failures": [],
    }
    html = render_report(report)
    assert html.startswith("<!DOCTYPE html>")
    assert _embedded(html) == report
    assert 'id="price-svg"' in html
    assert 'id="rsi-svg"' in html
    assert 'id="track"' in html


def test_render_report_escapes_script_breakout():
    bad = "</script><script>alert(1)</script>"
    html = render_report({"x": bad})
    assert "</script><script>alert(1)" not in html
    assert "\\u003c/script>" in html
    assert _embedded(html)["x"] == bad


def test_render_report_rejects_non_finite_json():
    with pytest.raises(ValueError):
        render_report({"x": float("nan")})


def test_write_report_creates_parent_directories(tmp_path):
    path = write_report({"tickers": [], "failures": []}, tmp_path / "a" / "b.html")
    assert path.exists()
    assert path.read_text(encoding="utf-8").startswith("<!DOCTYPE html>")
```

- [ ] **Step 2: Run them to verify they fail**

```bash
cd process/analyzer
python3 -m pytest benchmark/tests/test_report.py -v
```

Expected: FAIL — `No module named 'analyzer.benchmark.report'`.

- [ ] **Step 3: Create `report.py`**

Create `process/analyzer/benchmark/report.py`:

```python
"""Render the self-contained benchmark replay report."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

TEMPLATE_PATH = Path(__file__).with_name("replay_template.html")
PLACEHOLDER = "__REPLAY_DATA__"


def render_report(report: dict[str, Any]) -> str:
    """Inject a JSON payload into the static report template."""
    template = TEMPLATE_PATH.read_text(encoding="utf-8")
    if PLACEHOLDER not in template:
        raise ValueError(f"template missing {PLACEHOLDER}")
    payload = json.dumps(report, separators=(",", ":"), allow_nan=False)
    payload = payload.replace("<", "\\u003c")
    return template.replace(PLACEHOLDER, payload)


def write_report(report: dict[str, Any], path: Path) -> Path:
    """Write the report, creating parent directories as needed."""
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(render_report(report), encoding="utf-8")
    return path
```

- [ ] **Step 4: Create the full viewer template**

Create `process/analyzer/benchmark/replay_template.html` with exactly this content:

```html
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Benchmark Replay</title>
<style>
:root{--bg:#0a0c0f;--panel:#0b0e11;--panel2:#0e131b;--line:#1e2530;--text:#e2e8f0;--muted:#94a3b8;--dim:#64748b;--up:#1dcd9f;--down:#ff4d4d;--accent:#ff5722;--sma:#f59e0b;--rsi:#a78bfa}
*{box-sizing:border-box}
html,body{margin:0;min-height:100%}
body{background:var(--bg);color:var(--text);font:13px/1.45 ui-sans-serif,system-ui,-apple-system,"Segoe UI",sans-serif;display:flex;flex-direction:column;min-height:100vh}
button,select{font:inherit;color:var(--text);background:var(--panel2);border:1px solid var(--line);border-radius:6px;padding:5px 9px;cursor:pointer}
button:hover{border-color:var(--accent)}
button:disabled{opacity:.45;cursor:default}
.topbar{display:flex;align-items:center;gap:16px;padding:10px 16px;border-bottom:1px solid var(--line);background:var(--panel);position:sticky;top:0;z-index:5}
.brand{font-weight:700;letter-spacing:.4px;color:var(--accent)}
.tabs{display:flex;gap:6px;flex-wrap:wrap}
.tab{background:transparent;border-color:transparent;color:var(--muted);padding:4px 10px}
.tab.active{color:var(--text);border-color:var(--line);background:var(--panel2)}
.meta{margin-left:auto;color:var(--dim);font-size:11px}
.failures{margin:0;padding:6px 16px;background:#2a1212;border-bottom:1px solid #5a2020;color:#fca5a5;font-size:12px}
main{display:flex;flex-direction:column;flex:1;min-height:0}
.chart-box{position:relative;background:var(--panel);border-bottom:1px solid var(--line)}
#price-svg{display:block;width:100%;height:auto}
.rsi-box{background:var(--panel);border-bottom:1px solid var(--line)}
#rsi-svg{display:block;width:100%;height:auto}
.hud{position:absolute;top:12px;left:12px;width:330px;max-width:calc(100% - 24px);background:rgba(10,12,15,.92);border:1px solid var(--line);border-radius:10px;padding:10px 12px;backdrop-filter:blur(6px);font-size:12px;pointer-events:none}
.hud-title{display:flex;justify-content:space-between;align-items:baseline;margin-bottom:6px}
.hud-title b{font-size:14px}
.hud-title span{color:var(--dim)}
.hud-row{display:flex;gap:8px;color:var(--muted);margin:2px 0;flex-wrap:wrap}
.hud-row b{color:var(--text);font-weight:600}
.hud-signal{margin:7px 0;padding:6px 8px;border-radius:6px;background:rgba(29,205,159,.08);border-left:3px solid var(--up)}
.hud-signal.sell{background:rgba(255,77,77,.08);border-left-color:var(--down)}
.hud-signal.none{background:rgba(100,116,139,.08);border-left-color:var(--dim)}
.hud-signal .sub{display:block;color:var(--muted);font-size:11px;margin-top:2px}
.hud-account{color:var(--muted);font-size:11px;margin-top:5px}
.hud-last{color:var(--dim);font-size:11px;margin-top:4px}
.ledger-btn{position:absolute;top:12px;right:12px;z-index:2}
.ledger{position:absolute;top:46px;right:12px;z-index:3;width:min(560px,calc(100% - 24px));max-height:calc(100% - 60px);overflow:auto;background:rgba(10,12,15,.97);border:1px solid var(--line);border-radius:10px;padding:12px 14px;box-shadow:0 12px 40px rgba(0,0,0,.5)}
.ledger h3{margin:0 0 8px;font-size:13px;color:var(--muted);font-weight:600;text-transform:uppercase;letter-spacing:.6px}
.ledger table{width:100%;border-collapse:collapse;font-size:11px}
.ledger th{color:var(--dim);text-align:right;font-weight:500;padding:4px 5px;border-bottom:1px solid var(--line)}
.ledger th:first-child,.ledger td:first-child{text-align:left}
.ledger td{padding:4px 5px;border-bottom:1px solid rgba(30,37,48,.6);text-align:right;color:var(--muted)}
.ledger td.pos{color:var(--up)} .ledger td.neg{color:var(--down)}
.ledger .summary{margin-top:8px;color:var(--muted);font-size:11px}
.ledger .open{margin:10px 0 4px;color:var(--dim);font-size:11px;text-transform:uppercase;letter-spacing:.5px}
.controls{display:flex;align-items:center;gap:14px;padding:10px 16px;background:var(--panel);border-top:1px solid var(--line);flex-wrap:wrap}
.transport{display:flex;gap:4px}
.transport button{min-width:34px;text-align:center}
.scrub{flex:1;min-width:240px}
.track{position:relative;height:18px;cursor:pointer;touch-action:none}
.track::before{content:"";position:absolute;left:0;right:0;top:8px;height:4px;border-radius:2px;background:var(--line)}
.progress{position:absolute;left:0;top:8px;height:4px;border-radius:2px;background:var(--accent);pointer-events:none}
.dots{position:absolute;inset:0;pointer-events:none}
.dot{position:absolute;top:5px;width:8px;height:8px;margin-left:-4px;border-radius:50%;border:1px solid var(--bg)}
.dot.entry{background:var(--up)} .dot.exit{background:var(--down)}
.counters{display:flex;justify-content:space-between;color:var(--dim);font-size:11px;margin-top:2px}
.tools{display:flex;align-items:center;gap:8px;color:var(--muted);font-size:12px}
.tooltip{position:absolute;z-index:4;pointer-events:none;background:rgba(10,12,15,.97);border:1px solid var(--line);border-radius:6px;padding:6px 8px;font-size:11px;color:var(--muted);white-space:nowrap}
.empty{padding:60px 20px;text-align:center;color:var(--dim)}
@media(max-width:700px){.hud{width:calc(100% - 24px)}.meta{display:none}}
</style>
</head>
<body>
<header class="topbar">
  <div class="brand">Benchmark Replay</div>
  <div id="tabs" class="tabs"></div>
  <div id="meta" class="meta"></div>
</header>
<div id="failures" class="failures" hidden></div>
<main>
  <section class="chart-box" id="price-box">
    <svg id="price-svg" viewBox="0 0 1000 420" role="img" aria-label="Price candles"></svg>
    <div id="tooltip" class="tooltip" hidden></div>
    <div id="hud" class="hud"></div>
    <button id="ledger-btn" class="ledger-btn" type="button">Ledger</button>
    <aside id="ledger" class="ledger" hidden></aside>
  </section>
  <section class="rsi-box">
    <svg id="rsi-svg" viewBox="0 0 1000 160" role="img" aria-label="RSI"></svg>
  </section>
  <footer class="controls">
    <div class="transport">
      <button data-act="start" type="button" title="Reset">&#9198;</button>
      <button data-act="prev" type="button" title="Previous candle">&#8249;</button>
      <button data-act="play" id="play-btn" type="button" title="Play / pause">&#9655;</button>
      <button data-act="next" type="button" title="Next candle">&#8250;</button>
      <button data-act="end" type="button" title="Jump to end">&#9197;</button>
    </div>
    <div class="scrub">
      <div id="track" class="track"><div id="progress" class="progress"></div><div id="dots" class="dots"></div></div>
      <div class="counters"><span id="counter">0 / 0</span><span id="date-label"></span></div>
    </div>
    <div class="tools">
      <button data-act="prev-trade" type="button" title="Previous trade">&#10554; trade</button>
      <button data-act="next-trade" type="button" title="Next trade">trade &#10555;</button>
      <label>speed
        <select id="speed">
          <option value="0.5">0.5x</option>
          <option value="1" selected>1x</option>
          <option value="2">2x</option>
          <option value="4">4x</option>
          <option value="8">8x</option>
        </select>
      </label>
    </div>
  </footer>
</main>
<script id="replay-data" type="application/json">__REPLAY_DATA__</script>
<script>
(function () {
  "use strict";

  const REPORT = JSON.parse(document.getElementById("replay-data").textContent);
  const TICKERS = REPORT.tickers || [];
  const PRICE = { w: 1000, h: 420, l: 64, r: 84, t: 20, b: 28 };
  const RSI = { w: 1000, h: 160, l: 64, r: 84, t: 16, b: 20 };
  const state = { ti: 0, step: 0, playing: false, speed: 1, timer: null, hover: null };
  const $ = (id) => document.getElementById(id);

  const esc = (v) => String(v).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));

  const fmtPrice = (p) => {
    if (p === null || p === undefined || !Number.isFinite(Number(p))) return "\u2013";
    const n = Number(p), a = Math.abs(n);
    if (a >= 1000) return "$" + n.toLocaleString("en-US", { maximumFractionDigits: 0 });
    if (a >= 1) return "$" + n.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
    if (a >= 0.01) return "$" + n.toFixed(4);
    return "$" + n.toFixed(6);
  };
  const fmtSignedUsd = (p) => (Number(p) >= 0 ? "+$" : "-$") + Math.abs(Number(p)).toFixed(2);
  const fmtPct = (p) => (Number(p) >= 0 ? "+" : "") + Number(p).toFixed(1) + "%";

  const KIND = {
    regular_bullish_divergence: "regular bull",
    regular_bearish_divergence: "regular bear",
    hidden_bullish_divergence: "hidden bull",
    hidden_bearish_divergence: "hidden bear"
  };

  const cur = () => TICKERS[state.ti];
  const count = () => (cur() ? cur().candles.length : 0);
  const rel = (abs) => abs - cur().start_index;

  function xFor(r, dim) {
    const n = count();
    if (n <= 1) return dim.l;
    const clamped = Math.max(0, Math.min(n - 1, r));
    return dim.l + (clamped / (n - 1)) * (dim.w - dim.l - dim.r);
  }

  function priceDomain(t) {
    let lo = Infinity, hi = -Infinity;
    for (const c of t.candles) { lo = Math.min(lo, c.l); hi = Math.max(hi, c.h); }
    for (const tr of t.trades) {
      lo = Math.min(lo, tr.entry, tr.exit, tr.tp, tr.sl);
      hi = Math.max(hi, tr.entry, tr.exit, tr.tp, tr.sl);
    }
    for (const p of t.open_positions) {
      lo = Math.min(lo, p.entry, p.tp, p.sl);
      hi = Math.max(hi, p.entry, p.tp, p.sl);
    }
    if (!Number.isFinite(lo) || !Number.isFinite(hi)) return { lo: 0, hi: 1 };
    if (hi === lo) { hi += 1; lo -= 1; }
    const pad = (hi - lo) * 0.05;
    return { lo: lo - pad, hi: hi + pad };
  }

  function yFor(price, d, dim) {
    return dim.t + ((d.hi - price) / (d.hi - d.lo)) * (dim.h - dim.t - dim.b);
  }

  function renderTabs() {
    $("tabs").innerHTML = TICKERS.map((t, i) =>
      `<button type="button" class="tab${i === state.ti ? " active" : ""}" data-tab="${i}">${esc(t.ticker)}</button>`
    ).join("");
  }

  function renderFailures() {
    const failures = REPORT.failures || [];
    const el = $("failures");
    if (!failures.length) { el.hidden = true; el.textContent = ""; return; }
    el.hidden = false;
    el.innerHTML = "<b>Some tickers failed:</b> " + failures.map((f) => `${esc(f.symbol)} \u2014 ${esc(f.error)}`).join(" \u00b7 ");
  }

  function renderPrice() {
    const t = cur(), svg = $("price-svg");
    if (!t || !count()) {
      svg.innerHTML = `<text x="500" y="210" text-anchor="middle" fill="#64748b" font-size="16">No tickers were replayed.</text>`;
      return;
    }
    const n = count(), d = priceDomain(t), upTo = state.step;
    const cw = Math.max(2, Math.min(9, ((PRICE.w - PRICE.l - PRICE.r) / Math.max(1, n)) * 0.6));
    let s = "";

    for (let k = 0; k <= 4; k++) {
      const y = PRICE.t + (k / 4) * (PRICE.h - PRICE.t - PRICE.b);
      const price = d.hi - (k / 4) * (d.hi - d.lo);
      s += `<line x1="${PRICE.l}" y1="${y}" x2="${PRICE.w - PRICE.r}" y2="${y}" stroke="#1a202b"/>`;
      s += `<text x="${PRICE.l - 8}" y="${y + 4}" text-anchor="end" fill="#64748b" font-size="11">${fmtPrice(price)}</text>`;
    }

    const ticks = Math.min(6, n);
    for (let k = 0; k < ticks; k++) {
      const r = Math.round((k / Math.max(1, ticks - 1)) * (n - 1));
      const x = xFor(r, PRICE);
      s += `<text x="${x}" y="${PRICE.h - 8}" text-anchor="middle" fill="#64748b" font-size="10">${esc(t.candles[r].t)}</text>`;
    }

    for (const tr of t.trades) {
      const eRel = rel(tr.entry_i);
      if (eRel < 0 || eRel > upTo) continue;
      const xRel = rel(tr.exit_i);
      const endRel = Math.min(xRel, upTo);
      for (const [level, color, label] of [[tr.tp, "#1dcd9f", "TP"], [tr.sl, "#ff4d4d", "SL"]]) {
        const y = yFor(level, d, PRICE);
        s += `<line x1="${xFor(eRel, PRICE)}" y1="${y}" x2="${xFor(endRel, PRICE)}" y2="${y}" stroke="${color}" stroke-width="1" stroke-dasharray="4 4" opacity="0.7"/>`;
        s += `<text x="${xFor(endRel, PRICE) + 4}" y="${y + 4}" fill="${color}" font-size="9">${label}</text>`;
      }
      if (xRel <= upTo) {
        const y = yFor(tr.exit, d, PRICE);
        const color = tr.reason === "TP" ? "#1dcd9f" : "#ff4d4d";
        s += `<text x="${xFor(xRel, PRICE)}" y="${y + 4}" text-anchor="middle" fill="${color}" font-size="14" font-weight="700">\u00d7</text>`;
      }
    }

    for (const p of t.open_positions) {
      const eRel = rel(p.entry_i);
      if (eRel < 0 || eRel > upTo) continue;
      for (const [level, color, label] of [[p.tp, "#1dcd9f", "TP"], [p.sl, "#ff4d4d", "SL"]]) {
        const y = yFor(level, d, PRICE);
        s += `<line x1="${xFor(eRel, PRICE)}" y1="${y}" x2="${PRICE.w - PRICE.r}" y2="${y}" stroke="${color}" stroke-width="1" stroke-dasharray="4 4" opacity="0.7"/>`;
        s += `<text x="${PRICE.w - PRICE.r + 4}" y="${y + 4}" fill="${color}" font-size="9">${label}</text>`;
      }
    }

    for (let r = 0; r <= upTo; r++) {
      const c = t.candles[r], x = xFor(r, PRICE);
      const color = c.c >= c.o ? "#1dcd9f" : "#ff4d4d";
      const yH = yFor(c.h, d, PRICE), yL = yFor(c.l, d, PRICE);
      const yO = yFor(c.o, d, PRICE), yC = yFor(c.c, d, PRICE);
      s += `<line x1="${x}" y1="${yH}" x2="${x}" y2="${yL}" stroke="${color}" stroke-width="1"/>`;
      s += `<rect x="${x - cw / 2}" y="${Math.min(yO, yC)}" width="${cw}" height="${Math.max(1, Math.abs(yC - yO))}" fill="${color}" opacity="0.9"/>`;
    }

    const smaPts = [];
    for (let r = 0; r <= upTo; r++) {
      const v = t.sma[r];
      if (v !== null && v !== undefined) smaPts.push(`${xFor(r, PRICE)},${yFor(v, d, PRICE)}`);
    }
    if (smaPts.length > 1) {
      s += `<polyline points="${smaPts.join(" ")}" fill="none" stroke="#f59e0b" stroke-width="1.5" opacity="0.85"/>`;
    }

    for (const tr of t.trades) {
      const eRel = rel(tr.entry_i);
      if (eRel < 0 || eRel > upTo) continue;
      const x = xFor(eRel, PRICE), y = yFor(tr.entry, d, PRICE);
      s += tr.side === "buy"
        ? `<polygon points="${x},${y - 10} ${x - 5},${y - 2} ${x + 5},${y - 2}" fill="#1dcd9f"/>`
        : `<polygon points="${x},${y + 10} ${x - 5},${y + 2} ${x + 5},${y + 2}" fill="#ff4d4d"/>`;
    }
    for (const p of t.open_positions) {
      const eRel = rel(p.entry_i);
      if (eRel < 0 || eRel > upTo) continue;
      const x = xFor(eRel, PRICE), y = yFor(p.entry, d, PRICE);
      s += p.side === "buy"
        ? `<polygon points="${x},${y - 10} ${x - 5},${y - 2} ${x + 5},${y - 2}" fill="#1dcd9f"/>`
        : `<polygon points="${x},${y + 10} ${x - 5},${y + 2} ${x + 5},${y + 2}" fill="#ff4d4d"/>`;
    }

    const cx = xFor(upTo, PRICE);
    s += `<line x1="${cx}" y1="${PRICE.t}" x2="${cx}" y2="${PRICE.h - PRICE.b}" stroke="#ff5722" stroke-width="1" opacity="0.28"/>`;
    const cc = t.candles[upTo];
    const cyH = yFor(cc.h, d, PRICE), cyL = yFor(cc.l, d, PRICE);
    s += `<rect x="${cx - cw / 2 - 1.5}" y="${cyH - 1.5}" width="${cw + 3}" height="${Math.max(3, cyL - cyH + 3)}" fill="none" stroke="#ff5722" stroke-width="1" opacity="0.7"/>`;
    if (state.hover !== null && state.hover >= 0 && state.hover <= upTo) {
      const hx = xFor(state.hover, PRICE);
      s += `<line x1="${hx}" y1="${PRICE.t}" x2="${hx}" y2="${PRICE.h - PRICE.b}" stroke="#94a3b8" stroke-width="1" stroke-dasharray="3 3" opacity="0.5"/>`;
    }
    svg.innerHTML = s;
  }

  function renderRsi() {
    const t = cur(), svg = $("rsi-svg");
    if (!t || !count()) { svg.innerHTML = ""; return; }
    const y = (v) => RSI.t + ((100 - v) / 100) * (RSI.h - RSI.t - RSI.b);
    let s = `<rect x="${RSI.l}" y="${y(70)}" width="${RSI.w - RSI.l - RSI.r}" height="${y(30) - y(70)}" fill="#a78bfa" opacity="0.05"/>`;
    for (const v of [70, 30]) {
      s += `<line x1="${RSI.l}" y1="${y(v)}" x2="${RSI.w - RSI.r}" y2="${y(v)}" stroke="#334155" stroke-dasharray="3 3"/>`;
      s += `<text x="${RSI.l - 8}" y="${y(v) + 4}" text-anchor="end" fill="#64748b" font-size="10">${v}</text>`;
    }

    const pts = [];
    for (let r = 0; r <= state.step; r++) {
      const v = t.rsi[r];
      if (v !== null && v !== undefined) pts.push(`${xFor(r, RSI)},${y(v)}`);
    }
    if (pts.length > 1) {
      s += `<polyline points="${pts.join(" ")}" fill="none" stroke="#a78bfa" stroke-width="1.5"/>`;
    }

    for (const dv of t.divergences) {
      const det = rel(dv.detected_at), pivot = rel(dv.i);
      if (det < 0 || det > state.step || pivot < 0 || pivot >= count()) continue;
      const value = t.rsi[pivot];
      if (value === null || value === undefined) continue;
      const x = xFor(pivot, RSI), yy = y(value);
      const color = dv.side === "buy" ? "#1dcd9f" : "#ff4d4d";
      s += `<circle cx="${x}" cy="${yy}" r="3.5" fill="${color}"/>`;
      s += `<text x="${x + 6}" y="${yy - 4}" fill="${color}" font-size="9">${esc(dv.kind)}</text>`;
    }

    let last = null;
    for (let r = state.step; r >= 0; r--) {
      if (t.rsi[r] !== null && t.rsi[r] !== undefined) { last = t.rsi[r]; break; }
    }
    s += `<text x="${RSI.w - RSI.r}" y="${RSI.t + 10}" text-anchor="end" fill="#a78bfa" font-size="11">RSI ${last === null ? "\u2013" : last.toFixed(1)}</text>`;
    svg.innerHTML = s;
  }

  function renderHud() {
    const t = cur(), el = $("hud");
    if (!t || !count()) { el.innerHTML = ""; return; }
    const i = state.step, c = t.candles[i], ev = t.events[i];
    const absI = t.start_index + i;
    const full = (t.signals || []).find((s) => s.i === absI) || null;
    const change = c.o ? ((c.c - c.o) / c.o) * 100 : 0;

    let sigHtml = `<div class="hud-signal none">no signal on this candle</div>`;
    if (full) {
      const cls = full.action === "buy" ? "" : " sell";
      const status = full.rejected
        ? `skipped (${esc(full.rejected)})`
        : full.opened
          ? "entered"
          : "not opened (insufficient funds)";
      sigHtml = `<div class="hud-signal${cls}">${esc(full.action.toUpperCase())} \u00b7 ${esc(KIND[full.reason] || full.reason)}<span class="sub">conf ${Number(full.confidence).toFixed(2)} \u00b7 ${status} \u00b7 entry ${fmtPrice(full.entry)} \u00b7 TP ${fmtPrice(full.tp)} \u00b7 SL ${fmtPrice(full.sl)}</span></div>`;
    }

    const closed = t.trades.filter((tr) => rel(tr.exit_i) <= i).sort((a, b) => b.exit_i - a.exit_i);
    const last = closed[0];
    const lastHtml = last
      ? `Last: ${esc(last.side.toUpperCase())} ${fmtPct(last.pnl_pct)} ${esc(last.reason)} \u00b7 ${esc(last.exit_date)}`
      : "Last: \u2013";

    el.innerHTML = `
      <div class="hud-title"><b>${esc(t.ticker)}</b><span>${esc(c.t)}</span></div>
      <div class="hud-row"><span>O</span><b>${fmtPrice(c.o)}</b><span>H</span><b>${fmtPrice(c.h)}</b><span>L</span><b>${fmtPrice(c.l)}</b><span>C</span><b>${fmtPrice(c.c)}</b><span>${fmtPct(change)}</span></div>
      ${sigHtml}
      <div class="hud-account">Equity ${fmtPrice(ev ? ev.equity : t.summary.equity)} \u00b7 Realized ${fmtSignedUsd(ev ? ev.realized : 0)} \u00b7 Unrealized ${fmtSignedUsd(ev ? ev.unrealized : 0)} \u00b7 Open ${ev ? ev.open : 0} \u00b7 Available ${fmtPrice(ev ? ev.available : 0)}</div>
      <div class="hud-last">${lastHtml}</div>
    `;
  }

  function renderLedger() {
    const t = cur(), el = $("ledger");
    if (!t || !count()) { el.innerHTML = ""; return; }
    const i = state.step;
    const closed = t.trades
      .filter((tr) => rel(tr.entry_i) <= i && rel(tr.exit_i) <= i)
      .sort((a, b) => b.exit_i - a.exit_i);
    const stillOpen = t.trades
      .filter((tr) => rel(tr.entry_i) <= i && rel(tr.exit_i) > i)
      .map((tr) => ({ side: tr.side, entry_date: tr.entry_date, entry: tr.entry, tp: tr.tp, sl: tr.sl }));
    for (const p of t.open_positions) {
      if (rel(p.entry_i) <= i) stillOpen.push(p);
    }

    const rows = closed.map((tr) => {
      const cls = tr.pnl >= 0 ? "pos" : "neg";
      return `<tr><td>${esc(tr.entry_date)}</td><td>${esc(tr.side.toUpperCase())}</td><td>${fmtPrice(tr.entry)}</td><td>${esc(tr.exit_date)}</td><td>${fmtPrice(tr.exit)}</td><td>${esc(tr.reason)}</td><td class="${cls}">${fmtPct(tr.pnl_pct)}</td><td class="${cls}">${fmtSignedUsd(tr.pnl)}</td></tr>`;
    }).join("");
    const openRows = stillOpen.map((p) =>
      `<tr><td>${esc(p.entry_date)}</td><td>${esc(p.side.toUpperCase())}</td><td>${fmtPrice(p.entry)}</td><td colspan="2">open</td><td>\u2013</td><td>TP ${fmtPrice(p.tp)} \u00b7 SL ${fmtPrice(p.sl)}</td><td></td></tr>`
    ).join("");

    const wins = closed.filter((tr) => tr.pnl > 0).length;
    const winRate = closed.length ? (wins / closed.length) * 100 : 0;
    const ev = t.events[i];
    const summary = closed.length
      ? `trades ${closed.length} \u00b7 wins ${wins}/${closed.length} (${winRate.toFixed(0)}%) \u00b7 realized ${fmtSignedUsd(ev ? ev.realized : 0)} \u00b7 equity ${fmtPrice(ev ? ev.equity : t.summary.equity)} (init ${fmtPrice(t.summary.init_fund)})`
      : `no closed trades yet \u00b7 equity ${fmtPrice(ev ? ev.equity : t.summary.equity)} (init ${fmtPrice(t.summary.init_fund)})`;

    el.innerHTML = `
      <h3>Ledger \u00b7 ${esc(t.ticker)}</h3>
      <table><thead><tr><th>entry</th><th>dir</th><th>entry$</th><th>exit</th><th>exit$</th><th>why</th><th>pnl%</th><th>pnl$</th></tr></thead><tbody>${rows || `<tr><td colspan="8">No closed trades yet.</td></tr>`}</tbody></table>
      ${stillOpen.length ? `<div class="open">Open positions</div><table><tbody>${openRows}</tbody></table>` : ""}
      <div class="summary">${summary}</div>
    `;
  }

  function renderControls() {
    const t = cur(), n = count();
    const pct = n <= 1 ? 0 : (state.step / (n - 1)) * 100;
    $("progress").style.width = pct + "%";
    $("counter").textContent = n ? `${state.step + 1} / ${n}` : "0 / 0";
    $("date-label").textContent = t && n ? t.candles[state.step].t : "";

    const dots = [];
    if (t) {
      const seen = new Set();
      const add = (r, cls) => {
        if (r < 0 || r >= n || seen.has(cls + r)) return;
        seen.add(cls + r);
        dots.push(`<span class="dot ${cls}" style="left:${n <= 1 ? 0 : (r / (n - 1)) * 100}%"></span>`);
      };
      for (const tr of t.trades) { add(rel(tr.entry_i), "entry"); add(rel(tr.exit_i), "exit"); }
      for (const p of t.open_positions) add(rel(p.entry_i), "entry");
    }
    $("dots").innerHTML = dots.join("");
  }

  function renderAll() {
    renderPrice();
    renderRsi();
    renderHud();
    renderLedger();
    renderControls();
  }

  function stop() {
    state.playing = false;
    if (state.timer !== null) { clearTimeout(state.timer); state.timer = null; }
    const b = $("play-btn");
    if (b) b.textContent = "\u25b5";
  }

  function schedule() {
    if (!state.playing) return;
    state.timer = setTimeout(() => {
      state.timer = null;
      if (!state.playing) return;
      if (state.step >= count() - 1) { stop(); return; }
      state.step += 1;
      renderAll();
      schedule();
    }, 400 / state.speed);
  }

  function play() {
    if (!count() || state.playing) return;
    if (state.step >= count() - 1) state.step = 0;
    state.playing = true;
    $("play-btn").textContent = "\u23f8";
    renderAll();
    schedule();
  }

  function setStep(value) {
    const n = count();
    state.step = Math.max(0, Math.min(Math.max(0, n - 1), value));
    renderAll();
  }

  function setTicker(index) {
    state.ti = Math.max(0, Math.min(TICKERS.length - 1, index));
    state.step = 0;
    state.hover = null;
    stop();
    $("tooltip").hidden = true;
    $("ledger").hidden = true;
    renderTabs();
    renderAll();
  }

  function tradeStops() {
    const t = cur();
    if (!t) return [];
    const s = new Set();
    for (const tr of t.trades) { s.add(rel(tr.entry_i)); s.add(rel(tr.exit_i)); }
    for (const p of t.open_positions) s.add(rel(p.entry_i));
    return [...s].filter((r) => r >= 0 && r < count()).sort((a, b) => a - b);
  }

  function jumpTrade(dir) {
    const stops = tradeStops();
    if (!stops.length) return;
    if (dir > 0) {
      const next = stops.find((r) => r > state.step);
      setStep(next === undefined ? count() - 1 : next);
    } else {
      const prev = [...stops].reverse().find((r) => r < state.step);
      setStep(prev === undefined ? 0 : prev);
    }
  }

  function init() {
    const meta = $("meta");
    if (meta) meta.textContent = REPORT.generated_at ? `generated ${REPORT.generated_at}` : "";

    renderTabs();
    renderFailures();

    $("tabs").addEventListener("click", (e) => {
      const btn = e.target.closest("[data-tab]");
      if (btn) setTicker(Number(btn.dataset.tab));
    });

    document.querySelector(".controls").addEventListener("click", (e) => {
      const btn = e.target.closest("button[data-act]");
      if (!btn) return;
      const act = btn.dataset.act;
      if (act === "start") { stop(); setStep(0); }
      else if (act === "prev") { stop(); setStep(state.step - 1); }
      else if (act === "play") { state.playing ? stop() : play(); }
      else if (act === "next") { stop(); setStep(state.step + 1); }
      else if (act === "end") { stop(); setStep(count() - 1); }
      else if (act === "prev-trade") { stop(); jumpTrade(-1); }
      else if (act === "next-trade") { stop(); jumpTrade(1); }
    });

    $("speed").addEventListener("change", (e) => {
      state.speed = Number(e.target.value) || 1;
      if (state.playing) { clearTimeout(state.timer); state.timer = null; schedule(); }
    });

    $("ledger-btn").addEventListener("click", () => {
      const el = $("ledger");
      el.hidden = !el.hidden;
    });

    const track = $("track");
    let dragging = false;
    const scrub = (clientX) => {
      const rect = track.getBoundingClientRect();
      const p = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
      setStep(Math.round(p * Math.max(0, count() - 1)));
    };
    track.addEventListener("pointerdown", (e) => {
      dragging = true;
      track.setPointerCapture(e.pointerId);
      scrub(e.clientX);
    });
    track.addEventListener("pointermove", (e) => { if (dragging) scrub(e.clientX); });
    track.addEventListener("pointerup", () => { dragging = false; });
    track.addEventListener("pointercancel", () => { dragging = false; });

    document.addEventListener("keydown", (e) => {
      const tag = (e.target && e.target.tagName) || "";
      if (["INPUT", "SELECT", "TEXTAREA"].includes(tag)) return;
      if (e.code === "Space") { e.preventDefault(); state.playing ? stop() : play(); }
      else if (e.key === "ArrowLeft" && e.shiftKey) { e.preventDefault(); stop(); jumpTrade(-1); }
      else if (e.key === "ArrowRight" && e.shiftKey) { e.preventDefault(); stop(); jumpTrade(1); }
      else if (e.key === "ArrowLeft") { e.preventDefault(); stop(); setStep(state.step - 1); }
      else if (e.key === "ArrowRight") { e.preventDefault(); stop(); setStep(state.step + 1); }
      else if (e.key === "Home") { e.preventDefault(); stop(); setStep(0); }
      else if (e.key === "End") { e.preventDefault(); stop(); setStep(count() - 1); }
    });

    const svg = $("price-svg"), tip = $("tooltip");
    let hoverFrame = null;
    svg.addEventListener("mousemove", (e) => {
      const t = cur();
      if (!t || !count()) return;
      const rect = svg.getBoundingClientRect();
      const x = ((e.clientX - rect.left) / rect.width) * PRICE.w;
      const r = Math.round(((x - PRICE.l) / (PRICE.w - PRICE.l - PRICE.r)) * (count() - 1));
      if (r < 0 || r >= count() || r > state.step) { tip.hidden = true; return; }
      const c = t.candles[r];
      const box = $("price-box").getBoundingClientRect();
      tip.hidden = false;
      tip.innerHTML = `<b>${esc(c.t)}</b><br>O ${fmtPrice(c.o)} \u00b7 H ${fmtPrice(c.h)}<br>L ${fmtPrice(c.l)} \u00b7 C ${fmtPrice(c.c)}`;
      tip.style.left = `${e.clientX - box.left + 12}px`;
      tip.style.top = `${e.clientY - box.top + 12}px`;
      state.hover = r;
      if (hoverFrame === null) {
        hoverFrame = requestAnimationFrame(() => { hoverFrame = null; renderPrice(); });
      }
    });
    svg.addEventListener("mouseleave", () => {
      tip.hidden = true;
      state.hover = null;
      renderPrice();
    });

    if (TICKERS.length) state.ti = 0;
    state.step = 0;
    renderAll();
  }

  init();
})();
</script>
</body>
</html>
```

- [ ] **Step 5: Run the report tests**

```bash
cd process/analyzer
python3 -m pytest benchmark/tests/test_report.py -v
```

Expected: PASS (4 tests).

- [ ] **Step 6: Syntax-check the embedded JavaScript**

Run from the repo root:

```bash
python3 -c 'import re, pathlib; h = pathlib.Path("process/analyzer/benchmark/replay_template.html").read_text(encoding="utf-8"); pathlib.Path("/tmp/replay.js").write_text(re.search(r"<script>\n(.*?)\n</script>", h, re.S).group(1), encoding="utf-8")' && node --check /tmp/replay.js
```

Expected: no output, exit code 0.

- [ ] **Step 7: Commit**

```bash
git add process/analyzer/benchmark/report.py process/analyzer/benchmark/replay_template.html process/analyzer/benchmark/tests/test_report.py
git commit -m "feat(analyzer): render self-contained replay report"
```

---

### Task 5: Add the `--web` CLI, Makefile target, and docs

**Files:**
- Modify: `process/analyzer/benchmark/main.py`
- Modify: `process/analyzer/benchmark/tests/test_benchmark.py`
- Modify: `Makefile`
- Modify: `.gitignore`
- Modify: `process/analyzer/README.md`

- [ ] **Step 1: Write the failing web-mode CLI test**

Append to `process/analyzer/benchmark/tests/test_benchmark.py`:

```python
def test_main_web_writes_report(monkeypatch, tmp_path):
    import analyzer.benchmark.main as bm

    monkeypatch.setenv("BENCHMARK_TICKERS", "TEST-USDT")
    monkeypatch.setenv("SMA_ENABLED", "false")
    monkeypatch.setenv("RSI_PERIOD", "1")
    monkeypatch.setenv("ATR_PERIOD", "1")
    monkeypatch.setenv("BENCHMARK_DAYS", "50")

    n = 80
    close = np.full(n, 100.0)
    frame = pd.DataFrame(
        {"Open": close, "High": close + 2, "Low": close - 2, "Close": close}
    )

    monkeypatch.setattr(bm, "fetch_klines", lambda symbol, **kwargs: frame)
    monkeypatch.setattr(bm, "generate_signals", lambda *a, **k: [])

    out = tmp_path / "replay.html"
    bm.main(web=True, out=out)

    assert out.exists()
    html = out.read_text(encoding="utf-8")
    assert html.startswith("<!DOCTYPE html>")
    assert "TEST-USDT" in html
    assert "replay-data" in html
```

- [ ] **Step 2: Run it to verify it fails**

```bash
cd process/analyzer
python3 -m pytest benchmark/tests/test_benchmark.py::test_main_web_writes_report -v
```

Expected: FAIL — `main()` got an unexpected keyword argument `web`.

- [ ] **Step 3: Implement the CLI and web output in `main.py`**

Add `argparse` and `pathlib` imports at the top of `process/analyzer/benchmark/main.py` (`datetime` is already imported):

```python
import argparse
from pathlib import Path
```

Extend the existing `analyzer.benchmark.replay` import (from Task 2) to:

```python
from analyzer.benchmark.replay import (
    ReplayRecorder,
    account_state,
    build_ticker_replay,
    config_dict,
    date_str as _date_str,
)
from analyzer.benchmark.report import write_report
```

Add this helper above `main`:

```python
def _parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Replay analyzer benchmark candles."
    )
    parser.add_argument(
        "--web",
        action="store_true",
        help="write a self-contained HTML replay report instead of the text ledger",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=None,
        help="output path for --web (default: benchmark/out/benchmark.html)",
    )
    return parser.parse_args(argv)
```

Change `main` to:

```python
def main(web: bool = False, out: Path | None = None) -> None:
```

After the config logging block, before `min_len`, add:

```python
    replay_tickers: list[dict[str, Any]] = []
    failures: list[dict[str, str]] = []
```

In the `except KuCoinFetchError` block, after the log line, add:

```python
            if web:
                failures.append({"symbol": symbol, "error": str(e)})
```

In the `except Exception` block, after the log line, add:

```python
            if web:
                failures.append(
                    {"symbol": symbol, "error": f"{type(e).__name__}: {e}"}
                )
```

In the `if len(df) < min_len:` block, after the warning, add:

```python
            if web:
                failures.append(
                    {
                        "symbol": symbol,
                        "error": f"only {len(df)} candles (need >= {min_len})",
                    }
                )
```

Replace the `trades, open_positions, summary = run_benchmark(...)` call and the `print_ledger` call with:

```python
        recorder = ReplayRecorder() if web else None
        trades, open_positions, summary = run_benchmark(
            df,
            ticker=ticker,
            rsi_period=cfg.rsi_period,
            rsi_order=cfg.rsi_divergence_order,
            price_diff_threshold=cfg.price_diff_threshold,
            sma_enabled=cfg.sma_enabled,
            sma_period=cfg.sma_period,
            atr_period=cfg.atr_period,
            atr_sl_multiplier=cfg.atr_sl_multiplier,
            atr_tp_multiplier=cfg.atr_tp_multiplier,
            atr_pct_high=cfg.atr_pct_high,
            atr_high_adjust=cfg.atr_high_adjust,
            atr_sl_max_pct=cfg.atr_sl_max_pct,
            init_fund=cfg.benchmark_init_fund,
            fee_rate=cfg.benchmark_fee_rate,
            benchmark_days=cfg.benchmark_days,
            recorder=recorder,
        )

        if web:
            assert recorder is not None and recorder.start is not None
            replay_tickers.append(
                build_ticker_replay(
                    df,
                    ticker=ticker,
                    start=recorder.start,
                    recorder=recorder,
                    trades=trades,
                    open_positions=open_positions,
                    summary=summary,
                    config=config_dict(cfg),
                    warmup=_warmup(
                        cfg.rsi_period,
                        cfg.atr_period,
                        cfg.sma_period,
                        cfg.sma_enabled,
                    ),
                )
            )
        else:
            print_ledger(ticker, trades, open_positions, summary)
```

Replace the final `logger.info("done")` with:

```python
    if web:
        path = out or Path(__file__).resolve().parent / "out" / "benchmark.html"
        write_report(
            {
                "generated_at": datetime.now().isoformat(timespec="seconds"),
                "tickers": replay_tickers,
                "failures": failures,
            },
            path,
        )
        logger.info("report written to %s", path.resolve())

    logger.info("done")
```

Replace the `if __name__ == "__main__":` block with:

```python
if __name__ == "__main__":
    args = _parse_args()
    main(web=args.web, out=args.out)
```

- [ ] **Step 4: Run the benchmark tests**

```bash
cd process/analyzer
python3 -m pytest benchmark/tests -v
```

Expected: PASS (all benchmark tests, including the new web-mode test).

- [ ] **Step 5: Add the Makefile target**

In `Makefile`, add `benchmark-web` to the `.PHONY` line:

```make
.PHONY: help clean ci clippy test fmt fmt-fix frontend-install frontend-lint frontend-lint-fix frontend-test frontend-build dev prod proto prod-build logs logs-api-gateway restart test-api trade analyzer-test benchmark-web status seed seed-reset
```

Add the help line after the `make benchmark` line:

```make
	@echo "  make benchmark-web  Backtest and write a self-contained HTML replay report"
```

Add the target after `benchmark:`:

```make
benchmark-web:
	@cd process && python3 -m analyzer.benchmark.main --web
```

- [ ] **Step 6: Ignore generated reports**

Append to `.gitignore`:

```
process/analyzer/benchmark/out/
```

- [ ] **Step 7: Document usage**

In `process/analyzer/README.md`, under `## Quick start`, after the `make benchmark` comment, add:

```markdown
# Write a self-contained HTML candle-by-candle replay
make benchmark-web
```

Add a new section before `## Core rules`:

```markdown
## Web replay

`make benchmark-web` runs the same backtest and writes
`benchmark/out/benchmark.html` (gitignored). Open it in a browser — no server or
build step. It contains one tab per ticker and replays the benchmark candles one
at a time:

- candlesticks plus the SMA trend filter;
- RSI with divergence markers at the pivot candle;
- green/red entry and exit markers, with TP/SL lines;
- a floating HUD with OHLC, the signal that fired, account state, and the last
  closed trade;
- a timeline with play/pause, step, speed, trade jumps, and click-to-scrub;
- a ledger drawer listing closed trades and positions still open at the current
  candle.

Keyboard: `Space` play/pause, `←`/`→` step, `Shift+←`/`→` jump trades,
`Home`/`End` jump to the ends.
```

- [ ] **Step 8: Run the full analyzer test suite**

```bash
cd process/analyzer
python3 -m pytest . -v
```

Expected: PASS (all analyzer tests).

- [ ] **Step 9: Commit**

```bash
git add process/analyzer/benchmark/main.py process/analyzer/benchmark/tests/test_benchmark.py Makefile .gitignore process/analyzer/README.md
git commit -m "feat(analyzer): add benchmark-web HTML replay command"
```

---

### Task 6: End-to-end verification

**Files:** none (verification only; fix and commit only if something fails).

- [ ] **Step 1: Run the benchmark test target**

```bash
make benchmark-test
```

Expected: PASS.

- [ ] **Step 2: Run the full analyzer test target**

```bash
make analyzer-test
```

Expected: PASS.

- [ ] **Step 3: Re-check the embedded JavaScript**

```bash
python3 -c 'import re, pathlib; h = pathlib.Path("process/analyzer/benchmark/replay_template.html").read_text(encoding="utf-8"); pathlib.Path("/tmp/replay.js").write_text(re.search(r"<script>\n(.*?)\n</script>", h, re.S).group(1), encoding="utf-8")' && node --check /tmp/replay.js
```

Expected: no output, exit code 0.

- [ ] **Step 4: Generate a real report from synthetic data**

Create a throwaway report with a deterministic DataFrame by running the web path directly:

```bash
cd process/analyzer && python3 -c '
import numpy as np, pandas as pd
from pathlib import Path
from analyzer.benchmark.replay import ReplayRecorder, build_ticker_replay, config_dict
from analyzer.benchmark.main import run_benchmark, _warmup
from analyzer.benchmark.report import write_report
from analyzer.config import Config

n = 220
close = np.linspace(100.0, 140.0, n) + np.sin(np.arange(n) / 3.0) * 4.0
df = pd.DataFrame({"Open": close, "High": close + 2, "Low": close - 2, "Close": close})
cfg = Config()
cfg.benchmark_days = 120
cfg.sma_enabled = False
rec = ReplayRecorder()
trades, open_positions, summary = run_benchmark(
    df, ticker="SYNTH-USDT", rsi_period=cfg.rsi_period,
    rsi_order=cfg.rsi_divergence_order, sma_enabled=False,
    atr_period=cfg.atr_period, benchmark_days=cfg.benchmark_days,
    init_fund=cfg.benchmark_init_fund, fee_rate=cfg.benchmark_fee_rate,
    recorder=rec,
)
payload = build_ticker_replay(
    df, "SYNTH-USDT", start=rec.start, recorder=rec, trades=trades,
    open_positions=open_positions, summary=summary, config=config_dict(cfg),
    warmup=_warmup(cfg.rsi_period, cfg.atr_period, cfg.sma_period, cfg.sma_enabled),
)
out = Path("/tmp/benchmark-replay-smoke.html")
write_report({"generated_at": "2026-10-20T12:00:00", "tickers": [payload], "failures": []}, out)
print(out, out.stat().st_size)
'
```

Expected: prints `/tmp/benchmark-replay-smoke.html` and a non-zero byte size.

- [ ] **Step 5: Inspect the smoke report**

```bash
grep -c 'id="price-svg"' /tmp/benchmark-replay-smoke.html
grep -c 'SYNTH-USDT' /tmp/benchmark-replay-smoke.html
```

Expected: `1` for each. Open `/tmp/benchmark-replay-smoke.html` in a browser and confirm:
- candles appear one at a time when pressing play;
- RSI advances with the candles;
- the ledger button opens/closes;
- step, scrub, and speed controls work;
- the HUD updates date, OHLC, signal, and account.

- [ ] **Step 6: Check the working tree**

```bash
git status --short
```

Expected: only the intended tracked changes (none left after the task commits), plus any pre-existing untracked files such as `design-demos/`.

---

## Self-Review Checklist

- [ ] Spec section "Architecture" → Tasks 2–5.
- [ ] Spec section "Replay data contract" → Task 3.
- [ ] Spec section "Causal correctness" → Tasks 2–3 (recorder order, entry candle test).
- [ ] Spec section "Viewer specification" → Task 4 template.
- [ ] Spec section "Error handling" → Task 5 failure capture and empty state.
- [ ] Spec section "Testing" → Tasks 2–4 tests plus Task 6 verification.
- [ ] Spec section "Usage" → Task 5 README/Makefile.
- [ ] No new runtime dependencies.
- [ ] `Signal.to_dict()` is unchanged.
- [ ] Every new public name is defined before it is used: `date_str`, `account_state`, `ReplayRecorder`, `config_dict`, `build_ticker_replay`, `render_report`, `write_report`.
