import pandas as pd

from analyzer.indicators.rsi import detect_rsi_divergences
from analyzer.indicators.sma import calculate_sma
from analyzer.indicators.atr import calculate_atr
from analyzer.models.types import Signal


def _compute_price_diff(df: pd.DataFrame, lookback: int = 7) -> float:
    """Weekly price change as a fraction."""
    if len(df) < lookback:
        return 0.0
    recent = df["Close"].iloc[-1]
    past = df["Close"].iloc[-lookback]
    return (recent - past) / past if past != 0 else 0.0


def generate_signals(
    df: pd.DataFrame,
    ticker: str,
    rsi_period: int = 14,
    rsi_order: int = 5,
    price_diff_threshold: float = 0.05,
    sma_enabled: bool = False,
    sma_period: int = 20,
    atr_period: int = 14,
    atr_sl_multiplier: float = 2.0,
    atr_tp_multiplier: float = 3.0,
    atr_pct_high: float = 5.0,
    atr_high_adjust: float = 1.5,
    atr_sl_max_pct: float = 0.0,
) -> list[Signal]:
    """Generate ONE trading signal from the most recent RSI divergence.

    Scans divergence results backwards and returns only the latest
    actionable signal. Regular divergences take priority over hidden.

    When ``sma_enabled`` is True, a trend filter is applied via a Simple
    Moving Average: buy signals are only kept when price trades above the
    SMA (uptrend), and sell signals only when price trades below the SMA
    (downtrend).
    """
    result = detect_rsi_divergences(df, rsi_period=rsi_period, order=rsi_order)

    price_diff = _compute_price_diff(df)
    latest_close = df["Close"].iloc[-1]
    latest_rsi = result["RSI"].iloc[-1]

    # pick the single most recent divergence, scanning from newest → oldest
    reg_bull = 0
    hid_bull = 0
    reg_bear = 0
    hid_bear = 0

    for idx in range(len(result) - 1, -1, -1):
        row = result.iloc[idx]
        if reg_bull == 0 and row["Reg_Bullish_Div"] == 1:
            reg_bull = idx
        if hid_bull == 0 and row["Hid_Bullish_Div"] == 1:
            hid_bull = idx
        if reg_bear == 0 and row["Reg_Bearish_Div"] == 1:
            reg_bear = idx
        if hid_bear == 0 and row["Hid_Bearish_Div"] == 1:
            hid_bear = idx

        if reg_bull and reg_bear and hid_bull and hid_bear:
            break

    candidates: list[tuple[int, str, str, float]] = []

    if reg_bull:
        c = 0.8 if latest_rsi < 30 else 0.6
        if price_diff < -price_diff_threshold:
            c = min(c + 0.1, 1.0)
        candidates.append((reg_bull, "buy", "regular_bullish_divergence", c))
    if reg_bear:
        c = 0.8 if latest_rsi > 70 else 0.6
        if price_diff > price_diff_threshold:
            c = min(c + 0.1, 1.0)
        candidates.append((reg_bear, "sell", "regular_bearish_divergence", c))
    if hid_bull:
        c = 0.5
        if price_diff < -price_diff_threshold:
            c = min(c + 0.1, 1.0)
        candidates.append((hid_bull, "buy", "hidden_bullish_divergence", c))
    if hid_bear:
        c = 0.5
        if price_diff > price_diff_threshold:
            c = min(c + 0.1, 1.0)
        candidates.append((hid_bear, "sell", "hidden_bearish_divergence", c))

    if not candidates:
        return []

    if sma_enabled and len(df) >= sma_period:
        sma = calculate_sma(df["Close"], period=sma_period).iloc[-1]
        trend_filtered = [
            cand
            for cand in candidates
            if (cand[1] == "buy" and latest_close > sma)
            or (cand[1] == "sell" and latest_close < sma)
        ]
        if not trend_filtered:
            return []
        candidates = trend_filtered

    candidates.sort(key=lambda x: x[0], reverse=True)
    best = candidates[0]

    atr_val = 0.0
    if len(df) >= atr_period:
        atr_val = float(calculate_atr(df, period=atr_period).iloc[-1])

    sl_price = tp_price = 0.0
    if atr_val > 0 and latest_close > 0:
        atr_pct = (atr_val / latest_close) * 100
        sl_mult = atr_sl_multiplier
        tp_mult = atr_tp_multiplier
        if atr_pct > atr_pct_high:
            sl_mult *= atr_high_adjust
            tp_mult *= atr_high_adjust
        if best[1] == "buy":
            sl_price = latest_close - atr_val * sl_mult
            tp_price = latest_close + atr_val * tp_mult
            if atr_sl_max_pct > 0:
                max_sl = latest_close * (1 - atr_sl_max_pct)
                sl_price = max(sl_price, max_sl)
        else:
            sl_price = latest_close + atr_val * sl_mult
            tp_price = latest_close - atr_val * tp_mult
            if atr_sl_max_pct > 0:
                max_sl = latest_close * (1 + atr_sl_max_pct)
                sl_price = min(sl_price, max_sl)

    return [
        Signal(
            ticker=ticker,
            action=best[1],
            confidence=best[3],
            entry_price=latest_close,
            reason=best[2],
            sl_price=sl_price,
            tp_price=tp_price,
        )
    ]
