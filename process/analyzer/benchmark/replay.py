"""Replay capture for the benchmark HTML report."""

from __future__ import annotations

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
