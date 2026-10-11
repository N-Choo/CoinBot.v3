import pandas as pd
from scipy.signal import find_peaks


def calculate_rsi(series: pd.Series, period: int = 14) -> pd.Series:
    """Calculates standard Wilder's RSI."""
    delta = series.diff()
    gain = delta.where(delta > 0, 0.0)
    loss = -delta.where(delta < 0, 0.0)

    avg_gain = gain.ewm(alpha=1 / period, adjust=False).mean()
    avg_loss = loss.ewm(alpha=1 / period, adjust=False).mean()

    rs = avg_gain / (avg_loss + 1e-10)  # Avoid division by zero
    return 100 - (100 / (1 + rs))


def _match_extrema(price_extrema, rsi_extrema, order):
    """
    Build 1-to-1 matches between price and RSI extrema.
    Each RSI extremum is used at most once.
    Temporal ordering is enforced: rsi_idx strictly increases.
    """
    matches = []  # list of (price_idx, rsi_idx)
    rsi_used = set()

    for p_idx in price_extrema:
        best_r = -1
        best_dist = float("inf")
        prev_rsi = matches[-1][1] if matches else -1

        for r_idx in rsi_extrema:
            if r_idx in rsi_used:
                continue
            if r_idx <= prev_rsi:
                continue
            dist = abs(r_idx - p_idx)
            if dist <= order and dist < best_dist:
                best_dist = dist
                best_r = r_idx

        if best_r != -1:
            matches.append((p_idx, best_r))
            rsi_used.add(best_r)

    return matches


def detect_rsi_divergences(
    df: pd.DataFrame, rsi_period: int = 14, order: int = 5
) -> pd.DataFrame:
    """
    Detects Regular and Hidden RSI Divergences with fixed pairing logic.

    Parameters:
    -----------
    df : pd.DataFrame
        DataFrame with 'High', 'Low', 'Close' columns.
    rsi_period : int
        Lookback window for RSI calculation.
    order : int
        Distance parameter for local peak/trough confirmation.
    """
    data = df.copy()
    data["RSI"] = calculate_rsi(data["Close"], period=rsi_period)

    data["Reg_Bullish_Div"] = 0
    data["Reg_Bearish_Div"] = 0
    data["Hid_Bullish_Div"] = 0
    data["Hid_Bearish_Div"] = 0

    price_troughs, _ = find_peaks(-data["Low"].values, distance=order)
    price_peaks, _ = find_peaks(data["High"].values, distance=order)
    rsi_troughs, _ = find_peaks(-data["RSI"].values, distance=order)
    rsi_peaks, _ = find_peaks(data["RSI"].values, distance=order)

    # Guard: not enough extrema to compare
    if len(price_troughs) < 2 or len(rsi_troughs) < 2:
        if len(price_peaks) < 2 or len(rsi_peaks) < 2:
            return data

    # Bullish Divergences (Trough Comparisons)
    matched_troughs = _match_extrema(price_troughs, rsi_troughs, order)
    for i in range(1, len(matched_troughs)):
        prev_p_idx, prev_rsi_idx = matched_troughs[i - 1]
        curr_p_idx, curr_rsi_idx = matched_troughs[i]

        p_prev = data["Low"].iloc[prev_p_idx]
        p_curr = data["Low"].iloc[curr_p_idx]
        rsi_prev = data["RSI"].iloc[prev_rsi_idx]
        rsi_curr = data["RSI"].iloc[curr_rsi_idx]

        if p_curr < p_prev and rsi_curr > rsi_prev:
            data.iloc[curr_p_idx, data.columns.get_loc("Reg_Bullish_Div")] = 1
        elif p_curr > p_prev and rsi_curr < rsi_prev:
            data.iloc[curr_p_idx, data.columns.get_loc("Hid_Bullish_Div")] = 1

    # Bearish Divergences (Peak Comparisons)
    matched_peaks = _match_extrema(price_peaks, rsi_peaks, order)
    for i in range(1, len(matched_peaks)):
        prev_p_idx, prev_rsi_idx = matched_peaks[i - 1]
        curr_p_idx, curr_rsi_idx = matched_peaks[i]

        p_prev = data["High"].iloc[prev_p_idx]
        p_curr = data["High"].iloc[curr_p_idx]
        rsi_prev = data["RSI"].iloc[prev_rsi_idx]
        rsi_curr = data["RSI"].iloc[curr_rsi_idx]

        if p_curr > p_prev and rsi_curr < rsi_prev:
            data.iloc[curr_p_idx, data.columns.get_loc("Reg_Bearish_Div")] = 1
        elif p_curr < p_prev and rsi_curr > rsi_prev:
            data.iloc[curr_p_idx, data.columns.get_loc("Hid_Bearish_Div")] = 1

    return data
