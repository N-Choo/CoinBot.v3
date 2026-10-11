import pandas as pd
from analyzer.indicators.atr import calculate_atr


def _make_df(high, low, close):
    return pd.DataFrame({"High": high, "Low": low, "Close": close})


def test_atr_flat_prices_returns_high_low():
    df = _make_df([100] * 20, [99] * 20, [99.5] * 20)
    atr = calculate_atr(df, period=14)
    assert abs(atr.iloc[-1] - 1.0) < 0.1


def test_atr_known_range():
    high = [100.0] * 20
    low = [95.0] * 20
    close = [97.0] * 20
    df = _make_df(high, low, close)
    atr = calculate_atr(df, period=14)
    assert 4.5 < atr.iloc[-1] < 5.5


def test_atr_length_matches_input():
    import numpy as np
    n = 30
    df = _make_df(
        np.random.randn(n).cumsum() + 100,
        np.random.randn(n).cumsum() + 98,
        np.random.randn(n).cumsum() + 99,
    )
    atr = calculate_atr(df, period=14)
    assert len(atr) == n
