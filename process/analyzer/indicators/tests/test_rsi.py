import pandas as pd
import numpy as np
from analyzer.indicators.rsi import calculate_rsi, _match_extrema, detect_rsi_divergences


def test_calculate_rsi_flat_series_returns_0():
    """Flat series has no gains, so RSI is 0 (division by epsilon yields 0 RS)."""
    close = pd.Series([100.0] * 20, name="Close")
    rsi = calculate_rsi(close, period=14)
    assert rsi.iloc[-1] == 0.0


def test_calculate_rsi_all_gains_returns_100():
    close = pd.Series(np.linspace(100, 200, 30), name="Close")
    rsi = calculate_rsi(close, period=14)
    assert rsi.iloc[-1] > 99.0


def test_calculate_rsi_known_oversold():
    np.random.seed(42)
    n = 100
    prices = [100.0]
    for _ in range(n - 1):
        prices.append(prices[-1] * (1 + np.random.normal(-0.01, 0.02)))
    close = pd.Series(prices, name="Close")
    rsi = calculate_rsi(close, period=14)
    assert rsi.iloc[-1] < 50.0


def test_calculate_rsi_length_matches_input():
    close = pd.Series(np.random.randn(50).cumsum() + 100, name="Close")
    rsi = calculate_rsi(close, period=14)
    assert len(rsi) == len(close)


def test_regular_bullish_divergence_detected():
    """Price makes lower low, RSI makes higher low -> regular bullish"""
    np.random.seed(1)
    n = 200
    close = np.linspace(100, 50, n)
    close += np.random.randn(n) * 2
    high = close + np.random.rand(n) * 3
    low = close - np.random.rand(n) * 3
    # Force a lower low at index 180
    low[180] = low[179] - 10.0
    # Force a higher RSI low by making close recover slightly
    close[180] = close[179] + 2.0

    df = pd.DataFrame({"High": high, "Low": low, "Close": close})
    result = detect_rsi_divergences(df, rsi_period=14, order=10)
    assert result["Reg_Bullish_Div"].sum() >= 1


def test_regular_bearish_divergence_detected():
    """Price makes higher high, RSI makes lower high -> regular bearish"""
    np.random.seed(2)
    n = 200
    close = np.linspace(50, 100, n)
    close += np.random.randn(n) * 2
    high = close + np.random.rand(n) * 3
    low = close - np.random.rand(n) * 3
    # Force a higher high
    high[180] = high[179] + 10.0
    # RSI momentum diverges
    close[180] = close[179] - 2.0

    df = pd.DataFrame({"High": high, "Low": low, "Close": close})
    result = detect_rsi_divergences(df, rsi_period=14, order=10)
    assert result["Reg_Bearish_Div"].sum() >= 1


def test_no_same_rsi_extremum_reused():
    """
    Verify the fix: the old greedy nearest-neighbor approach could reuse
    the same RSI trough for multiple price troughs. After the fix,
    each RSI extremum is paired at most once, and temporal ordering is
    enforced (prev_rsi_idx < curr_rsi_idx).
    """
    from scipy.signal import find_peaks

    np.random.seed(3)
    n = 300
    t = np.linspace(0, 10 * np.pi, n)
    close = 100 + 10 * np.sin(t) + np.random.randn(n) * 0.5
    close = close + np.linspace(0, 5, n)
    high = close + np.random.rand(n) * 2
    low = close - np.random.rand(n) * 2

    df = pd.DataFrame({"High": high, "Low": low, "Close": close})
    result = detect_rsi_divergences(df, rsi_period=14, order=5)

    total = (
        result["Reg_Bullish_Div"].sum()
        + result["Reg_Bearish_Div"].sum()
        + result["Hid_Bullish_Div"].sum()
        + result["Hid_Bearish_Div"].sum()
    )
    assert total >= 0

    # Verify no RSI extremum reuse and temporal ordering
    rsi_series = result["RSI"]
    price_troughs, _ = find_peaks(-df["Low"].values, distance=5)
    rsi_troughs, _ = find_peaks(-rsi_series.values, distance=5)

    matches = _match_extrema(price_troughs, rsi_troughs, 5)
    rsi_indices = [m[1] for m in matches]
    assert len(rsi_indices) == len(set(rsi_indices)), (
        f"RSI extremum reused: {rsi_indices}"
    )
    assert rsi_indices == sorted(rsi_indices), (
        f"Temporal order violated: {rsi_indices}"
    )
