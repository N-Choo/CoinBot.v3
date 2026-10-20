import json
from types import SimpleNamespace

import numpy as np
import pandas as pd
import pytest

from analyzer.benchmark.main import run_benchmark
from analyzer.benchmark.replay import (
    REASON_KIND,
    CandleEvent,
    ReplayRecorder,
    _json_safe,
    build_ticker_replay,
    config_dict,
)
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
    assert final["open_count"] == summary["open_count"]
    assert final["available_fund"] == pytest.approx(summary["available_fund"])
    for key in (
        "realized_pnl",
        "unrealized_pnl",
        "open_count",
        "available_fund",
        "equity",
    ):
        assert key in summary


def test_opened_signals_match_trades_and_open_positions(monkeypatch):
    df = _df(80)
    recorder, trades, open_positions, _ = _run(df, monkeypatch)
    opened = sum(1 for e in recorder.events if e.opened)
    assert opened == len(trades) + len(open_positions)
    assert opened == 2
    assert opened > 0


def test_invalid_tp_sl_is_recorded_as_rejection(monkeypatch):
    import analyzer.benchmark.main as bm

    df = _df(80)

    def fake_signals(df_slice, **kwargs):
        entry = float(df_slice["Close"].iloc[-1])
        return [
            Signal(
                ticker="TEST-USDT",
                action="buy",
                confidence=0.6,
                entry_price=entry,
                reason="regular_bullish_divergence",
                sl_price=entry * 0.9,
                tp_price=0.0,
            )
        ]

    monkeypatch.setattr(bm, "generate_signals", fake_signals)
    recorder = ReplayRecorder()
    trades, open_positions, _ = run_benchmark(
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

    final = recorder.events[-1]
    assert final.signal is not None
    assert final.opened is False
    assert final.rejected == "invalid_tp_sl"
    assert trades == []
    assert open_positions == []


def test_insufficient_funds_is_not_a_rejection(monkeypatch):
    df = _df(80)
    recorder, _, open_positions, _ = _run(df, monkeypatch)

    blocked = [
        e for e in recorder.events if e.signal is not None and not e.opened
    ]
    assert any(e.rejected is None for e in blocked)
    assert sum(1 for e in recorder.events if e.opened) == 2
    assert len(open_positions) == 2


def test_recorder_begin_resets_state():
    recorder = ReplayRecorder()
    recorder.begin(5, _df())
    recorder.events.append(
        CandleEvent(
            i=5,
            signal=None,
            opened=False,
            rejected=None,
            exits=[],
            snapshot={},
        )
    )
    recorder.finished = True

    recorder.begin(10, _df())

    assert recorder.start == 10
    assert recorder.events == []
    assert recorder.finished is False


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


def _payload(
    df, monkeypatch, signal_at=None, divergence_idx=None, config=CONFIG
):
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
        config=config,
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


def test_payload_is_strictly_json_safe(monkeypatch):
    df = _df(80)
    replay = _payload(df, monkeypatch, signal_at=40)
    json.dumps(replay, allow_nan=False)

    extended = dict(replay)
    extended["non_finite"] = [np.float64("nan"), np.float64("inf")]
    extended["numpy_int"] = np.int64(3)
    extended["numpy_bool"] = np.bool_(True)

    cleaned = _json_safe(extended)
    assert cleaned["non_finite"] == [None, None]
    assert cleaned["numpy_int"] == 3
    assert cleaned["numpy_bool"] is True

    text = json.dumps(cleaned, allow_nan=False)
    assert "NaN" not in text
    assert "Infinity" not in text


def test_closed_trade_mapping(monkeypatch):
    df = _df(80)
    df.loc[40, ["High", "Low"]] = [200.0, 1.0]
    df.loc[41, ["High", "Low"]] = [200.0, 1.0]
    replay = _payload(df, monkeypatch, signal_at=40)

    assert len(replay["trades"]) == 1
    trade = replay["trades"][0]
    assert trade["entry_i"] == 40
    assert trade["exit_i"] == 41
    assert trade["tp"] == pytest.approx(110)
    assert trade["sl"] == pytest.approx(90)
    assert trade["reason"] == "TP"

    exit_events = [e for e in replay["events"] if e["exits"]]
    assert len(exit_events) == 1
    event = exit_events[0]
    assert event["i"] == 41
    exit_trade = event["exits"][0]
    assert exit_trade["entry_i"] == 40
    assert exit_trade["exit_i"] == 41
    assert exit_trade["reason"] == "TP"
    assert replay["open_positions"] == []


def test_pre_window_pivot_is_omitted(monkeypatch):
    df = _df(80)
    replay = _payload(df, monkeypatch, signal_at=40, divergence_idx=0)
    assert replay["start_index"] == 1
    assert replay["divergences"] == []


def test_divergences_are_deduplicated(monkeypatch):
    df = _df(80)
    replay = _payload(df, monkeypatch, divergence_idx=5)
    assert len(replay["signals"]) > 1
    assert replay["divergences"] == [
        {"i": 5, "detected_at": 1, "kind": "regular_bull", "side": "buy"}
    ]


def test_sma_enabled_populates_series(monkeypatch):
    df = _df(80)
    replay = _payload(
        df,
        monkeypatch,
        config=dict(CONFIG, sma_enabled=True, sma_period=5),
    )
    assert len(replay["sma"]) == len(df) - replay["start_index"]
    assert any(value is not None for value in replay["sma"])


def test_mismatched_start_raises(monkeypatch):
    df = _df(80)
    recorder, trades, open_positions, summary = _run(
        df, monkeypatch, signal_at=40
    )
    with pytest.raises(ValueError):
        build_ticker_replay(
            df,
            "TEST-USDT",
            start=recorder.start + 1,
            recorder=recorder,
            trades=trades,
            open_positions=open_positions,
            summary=summary,
            config=CONFIG,
            warmup=0,
        )


def test_reason_kind_covers_all_four_reasons():
    assert set(REASON_KIND) == {
        "regular_bullish_divergence",
        "regular_bearish_divergence",
        "hidden_bullish_divergence",
        "hidden_bearish_divergence",
    }
    assert REASON_KIND == {
        "regular_bullish_divergence": "regular_bull",
        "regular_bearish_divergence": "regular_bear",
        "hidden_bullish_divergence": "hidden_bull",
        "hidden_bearish_divergence": "hidden_bear",
    }


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
    assert mapped == {
        "timeframe": "1day",
        "rsi_period": 14,
        "rsi_order": 5,
        "price_diff_threshold": 0.05,
        "sma_enabled": True,
        "sma_period": 200,
        "atr_period": 14,
        "sl_mult": 2.8,
        "tp_mult": 3.5,
        "atr_pct_high": 5.0,
        "atr_high_adjust": 1.5,
        "atr_sl_max_pct": 0.1,
        "init_fund": 1000.0,
        "fee_rate": 0.001,
        "benchmark_days": 90,
    }
    assert len(mapped) == 15
