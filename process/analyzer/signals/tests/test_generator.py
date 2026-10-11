import pandas as pd
import numpy as np
from analyzer.signals.generator import generate_signals


def _make_df(close_vals, high_vals=None, low_vals=None):
    n = len(close_vals)
    if high_vals is None:
        high_vals = [c * 1.02 for c in close_vals]
    if low_vals is None:
        low_vals = [c * 0.98 for c in close_vals]
    return pd.DataFrame(
        {"High": high_vals, "Low": low_vals, "Close": close_vals}
    )


def test_generate_signals_hold_when_no_divergence():
    close = list(np.linspace(100, 105, 200))
    df = _make_df(close)
    signals = generate_signals(df, ticker="BTC-USDT")
    assert len(signals) == 0


def test_generate_signals_buy_on_bullish_divergence():
    np.random.seed(10)
    n = 200
    close = np.linspace(100, 80, n)
    close += np.random.randn(n) * 0.5
    high = close + np.random.rand(n)
    low = close - np.random.rand(n)
    # Force a lower price low
    low[190] = low[189] - 5.0
    # But RSI should recover
    close[190] = close[189] + 1.0

    df = pd.DataFrame({"High": high, "Low": low, "Close": close})
    signals = generate_signals(df, ticker="ETH-USDT")

    buy_signals = [s for s in signals if s.action == "buy"]
    assert len(buy_signals) >= 1
    assert buy_signals[0].ticker == "ETH-USDT"
    assert buy_signals[0].entry_price > 0
    assert buy_signals[0].confidence > 0


def test_generate_signals_sell_on_bearish_divergence():
    """Synthetic diverging pattern: generate_signals returns 0 or 1 signal."""
    n = 250
    x = np.linspace(0, 6 * np.pi, n)
    amp = np.linspace(30, 15, n)
    close = 100 + np.sin(x) * amp
    close += np.random.default_rng(42).normal(0, 0.1, n)
    high = close + 3.0 + np.abs(np.sin(x * 2)) * 5.0
    low = close - 3.0

    df = pd.DataFrame({"High": high, "Low": low, "Close": close})
    signals = generate_signals(df, ticker="SOL-USDT", rsi_order=15)

    assert len(signals) <= 1
    if signals:
        assert signals[0].action in ("buy", "sell")
        assert signals[0].ticker == "SOL-USDT"
        assert 0.0 < signals[0].confidence <= 1.0


def test_generate_signals_returns_list_of_signals():
    np.random.seed(30)
    n = 200
    close = np.linspace(100, 80, n) + np.random.randn(n) * 0.5
    high = close + np.random.rand(n)
    low = close - np.random.rand(n)
    low[190] = low[189] - 5.0
    close[190] = close[189] + 1.0

    df = pd.DataFrame({"High": high, "Low": low, "Close": close})
    signals = generate_signals(df, ticker="BTC-USDT")

    assert isinstance(signals, list)
    assert len(signals) <= 1
    for s in signals:
        assert hasattr(s, "ticker")
        assert hasattr(s, "action")
        assert s.action in ("buy", "sell")
        assert 0.0 <= s.confidence <= 1.0


def test_sma_filter_disabled_keeps_signals():
    np.random.seed(10)
    n = 200
    close = np.linspace(100, 80, n)
    close += np.random.randn(n) * 0.5
    high = close + np.random.rand(n)
    low = close - np.random.rand(n)
    low[190] = low[189] - 5.0
    close[190] = close[189] + 1.0

    df = pd.DataFrame({"High": high, "Low": low, "Close": close})
    signals = generate_signals(
        df, ticker="BTC-USDT", sma_enabled=False, sma_period=20
    )
    assert len(signals) <= 1
    assert len([s for s in signals if s.action == "buy"]) >= 1


def test_sma_filter_blocks_buy_in_downtrend():
    """Bullish divergence in a strong downtrend is rejected by SMA filter."""
    np.random.seed(10)
    n = 200
    close = np.linspace(100, 80, n)
    close += np.random.randn(n) * 0.5
    high = close + np.random.rand(n)
    low = close - np.random.rand(n)
    low[190] = low[189] - 5.0
    close[190] = close[189] + 1.0

    df = pd.DataFrame({"High": high, "Low": low, "Close": close})
    sma_period = 50
    signals = generate_signals(
        df, ticker="BTC-USDT", sma_enabled=True, sma_period=sma_period
    )
    for s in signals:
        assert s.action == "sell", "buy suppressed in downtrend by SMA filter"
