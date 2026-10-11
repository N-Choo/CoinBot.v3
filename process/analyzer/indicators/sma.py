import pandas as pd


def calculate_sma(series: pd.Series, period: int = 20) -> pd.Series:
    """Calculates Simple Moving Average."""
    return series.rolling(window=period).mean()