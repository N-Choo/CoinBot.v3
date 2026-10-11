import numpy as np
from typing import Any

import pytest
from analyzer.benchmark.main import _score_signal


def test_score_buy_hit():
    hit, pct = _score_signal("buy", entry=100.0, forward=110.0)
    assert hit is True
    assert pct == pytest.approx(10.0)


def test_score_buy_miss():
    hit, pct = _score_signal("buy", entry=100.0, forward=95.0)
    assert hit is False
    assert pct == pytest.approx(-5.0)


def test_score_sell_hit():
    hit, pct = _score_signal("sell", entry=100.0, forward=90.0)
    assert hit is True
    assert pct == pytest.approx(10.0)


def test_score_sell_miss():
    hit, pct = _score_signal("sell", entry=100.0, forward=105.0)
    assert hit is False
    assert pct == pytest.approx(-5.0)


def test_score_na_when_no_forward():
    hit, pct = _score_signal("buy", entry=100.0, forward=None)
    assert hit is None
    assert pct is None


from analyzer.benchmark.main import _make_divergence_label


def test_divergence_label_regular_bullish():
    row_data = {"Reg_Bullish_Div": 1, "Reg_Bearish_Div": 0,
                "Hid_Bullish_Div": 0, "Hid_Bearish_Div": 0}
    assert _make_divergence_label(row_data) == "regular_bullish"


def test_divergence_label_regular_bearish():
    row_data = {"Reg_Bullish_Div": 0, "Reg_Bearish_Div": 1,
                "Hid_Bullish_Div": 0, "Hid_Bearish_Div": 0}
    assert _make_divergence_label(row_data) == "regular_bearish"


def test_divergence_label_hidden_bullish():
    row_data = {"Reg_Bullish_Div": 0, "Reg_Bearish_Div": 0,
                "Hid_Bullish_Div": 1, "Hid_Bearish_Div": 0}
    assert _make_divergence_label(row_data) == "hidden_bullish"


def test_divergence_label_hidden_bearish():
    row_data = {"Reg_Bullish_Div": 0, "Reg_Bearish_Div": 0,
                "Hid_Bullish_Div": 0, "Hid_Bearish_Div": 1}
    assert _make_divergence_label(row_data) == "hidden_bearish"


def test_divergence_label_no_match_returns_unknown():
    row_data = {"Reg_Bullish_Div": 0, "Reg_Bearish_Div": 0,
                "Hid_Bullish_Div": 0, "Hid_Bearish_Div": 0}
    assert _make_divergence_label(row_data) == "unknown"


import pandas as pd
from analyzer.benchmark.main import _apply_sma_filter


def test_sma_filter_allows_buy_above_sma():
    close = pd.Series([100.0] * 20 + [110.0])
    assert _apply_sma_filter(close, idx=20, action="buy", sma_period=5) is True


def test_sma_filter_blocks_buy_below_sma():
    close = pd.Series([100.0] * 20 + [90.0])
    assert _apply_sma_filter(close, idx=20, action="buy", sma_period=5) is False


def test_sma_filter_allows_sell_below_sma():
    close = pd.Series([100.0] * 20 + [90.0])
    assert _apply_sma_filter(close, idx=20, action="sell", sma_period=5) is True


def test_sma_filter_blocks_sell_above_sma():
    close = pd.Series([100.0] * 20 + [110.0])
    assert _apply_sma_filter(close, idx=20, action="sell", sma_period=5) is False


def test_sma_filter_short_window_returns_true():
    close = pd.Series([100.0] * 3)
    assert _apply_sma_filter(close, idx=2, action="buy", sma_period=5) is True


from analyzer.benchmark.main import run_benchmark


def test_run_benchmark_on_synthetic_data():
    """End-to-end: rolling simulation produces valid signal structures."""
    np.random.seed(99)
    n = 120
    close = np.linspace(100, 80, n) + np.random.randn(n) * 0.3
    high = close + np.random.rand(n) * 2
    low = close - np.random.rand(n) * 2

    df = pd.DataFrame({"High": high, "Low": low, "Close": close})

    signals, summary = run_benchmark(
        df,
        ticker="TEST-USDT",
        rsi_period=14,
        rsi_order=10,
        forward_days=7,
        sma_enabled=False,
        sma_period=20,
    )

    assert isinstance(signals, list)
    assert "ticker" in summary
    assert "buy_total" in summary
    assert "sell_total" in summary

    for s in signals:
        assert "action" in s
        assert s["action"] in ("buy", "sell")
        assert "divergence" in s
        assert "entry_price" in s
        assert "pct" in s
        assert "hit" in s
        assert s["hit"] in (True, False, None)


from analyzer.benchmark.main import print_ledger


def test_print_ledger_no_signals(capsys):
    signals: list[dict[str, Any]] = []
    summary = {
        "ticker": "BTC-USDT", "buy_hits": 0, "buy_total": 0,
        "sell_hits": 0, "sell_total": 0,
    }
    print_ledger(signals, summary, forward_days=7)
    captured = capsys.readouterr().out
    assert "(no signals)" in captured


def test_print_ledger_with_signals(capsys):
    signals = [
        {
            "ticker": "BTC-USDT", "date": "2026-05-12",
            "action": "buy", "divergence": "regular_bullish",
            "entry_price": 91200.0, "forward_price": 94500.0,
            "pct": 3.62, "hit": True, "sl_price": 0, "tp_price": 0,
            "exit": "expiry",
        },
        {
            "ticker": "BTC-USDT", "date": "2026-05-25",
            "action": "buy", "divergence": "hidden_bullish",
            "entry_price": 93400.0, "forward_price": 92100.0,
            "pct": -1.39, "hit": False, "sl_price": 0, "tp_price": 0,
            "exit": "expiry",
        },
    ]
    summary = {
        "ticker": "BTC-USDT", "buy_hits": 1, "buy_total": 2,
        "sell_hits": 0, "sell_total": 0,
    }
    print_ledger(signals, summary, forward_days=7)
    captured = capsys.readouterr().out
    assert "BTC-USDT" in captured
    assert "\u2713" in captured
    assert "\u2717" in captured
    assert "50%" in captured
