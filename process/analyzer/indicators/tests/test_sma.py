import pandas as pd
from analyzer.indicators.sma import calculate_sma


def test_calculate_sma_constant_series_matches_const():
    close = pd.Series([100.0] * 10, name="Close")
    sma = calculate_sma(close, period=5)
    assert sma.iloc[-1] == 100.0


def test_calculate_sma_average_of_last_period():
    close = pd.Series([1.0, 2.0, 3.0, 4.0, 5.0, 6.0], name="Close")
    sma = calculate_sma(close, period=5)
    assert sma.iloc[-1] == 4.0  # (2+3+4+5+6)/5


def test_calculate_sma_first_values_are_nan():
    close = pd.Series([1.0, 2.0, 3.0, 4.0, 5.0], name="Close")
    sma = calculate_sma(close, period=5)
    assert sma.iloc[3] != sma.iloc[3]  # NaN (index<period-1)
    assert sma.iloc[4] == 3.0