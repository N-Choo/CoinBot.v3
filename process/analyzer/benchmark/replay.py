"""Replay capture for the benchmark HTML report."""

from __future__ import annotations

import math
from dataclasses import dataclass
from datetime import datetime
from typing import Any

import pandas as pd

from analyzer.benchmark.engine import ClosedTrade, SimEngine
from analyzer.indicators.rsi import calculate_rsi
from analyzer.indicators.sma import calculate_sma
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
        self.events = []
        self.finished = False

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
    """Assemble one ticker's replay payload from a completed run.

    ``start`` must equal ``recorder.start``: the payload drops the warm-up
    candles before that position. Every emitted ``i`` field is an absolute
    DataFrame position, which the viewer maps through ``start_index``.
    ``config`` must be the exact config used for the run, so the values
    embedded in the report match the recorded indicators and decisions.
    """
    if recorder.start is None:
        raise ValueError("recorder has no start index")
    if start != recorder.start:
        raise ValueError(
            f"start {start} does not match recorder start {recorder.start}"
        )
    if len(recorder.events) != len(df) - start:
        raise ValueError(
            f"recorder has {len(recorder.events)} events, expected "
            f"{len(df) - start}"
        )
    if recorder.events and recorder.events[0].i != start:
        raise ValueError(
            f"first event is {recorder.events[0].i}, expected {start}"
        )

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
