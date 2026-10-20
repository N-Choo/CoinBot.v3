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
