import numpy as np
import pytest
import pandas as pd

from analyzer.benchmark.main import run_benchmark
from analyzer.models.types import Signal


def test_run_benchmark_returns_trades_and_summary():
    """End-to-end: the replay produces valid trade and summary structures."""
    np.random.seed(99)
    n = 200
    close = np.linspace(100, 80, n) + np.random.randn(n) * 0.3
    high = close + np.random.rand(n) * 2
    low = close - np.random.rand(n) * 2
    df = pd.DataFrame({"High": high, "Low": low, "Close": close})

    trades, open_positions, summary = run_benchmark(
        df,
        ticker="TEST-USDT",
        rsi_period=14,
        rsi_order=10,
        sma_enabled=False,
        sma_period=20,
        benchmark_days=90,
        init_fund=1000.0,
        fee_rate=0.001,
    )

    assert isinstance(trades, list)
    assert isinstance(open_positions, list)
    for key in (
        "ticker",
        "trades",
        "wins",
        "win_rate",
        "realized_pnl",
        "unrealized_pnl",
        "open_count",
        "init_fund",
        "available_fund",
        "equity",
    ):
        assert key in summary
    assert summary["ticker"] == "TEST-USDT"
    assert summary["equity"] == pytest.approx(
        summary["init_fund"]
        + summary["realized_pnl"]
        + summary["unrealized_pnl"]
    )
    for t in trades:
        assert t["side"] in ("buy", "sell")
        assert t["reason"] in ("TP", "SL")


def test_run_benchmark_simulates_trades_end_to_end(monkeypatch):
    """Drive the replay with a fixed signal and verify TP exits are booked."""
    import analyzer.benchmark.main as bm

    n = 60
    close = np.full(n, 100.0)
    close[40:] = 120.0
    df = pd.DataFrame(
        {"High": close + 1, "Low": close - 1, "Close": close}
    )

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
                tp_price=entry * 1.1,
            )
        ]

    monkeypatch.setattr(bm, "generate_signals", fake_signals)

    trades, open_positions, summary = run_benchmark(
        df, ticker="TEST-USDT", benchmark_days=50,
        init_fund=1000.0, fee_rate=0.0,
    )

    assert summary["trades"] >= 1
    assert all(t["reason"] == "TP" for t in trades)
    assert all(t["pnl"] > 0 for t in trades)
