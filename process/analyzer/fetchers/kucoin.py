from datetime import datetime

import pandas as pd
import requests

BASE_URL = "https://api.kucoin.com"


class KuCoinFetchError(Exception):
    pass


def _parse_kline_response(raw: dict) -> pd.DataFrame:
    """Convert KuCoin kline JSON to DataFrame with named columns.

    KuCoin kline format: [time, open, close, high, low, volume, turnover]
    """
    data = raw.get("data", [])
    if not data:
        raise KuCoinFetchError("No kline data returned")

    rows = []
    for entry in data:
        rows.append(
            {
                "Timestamp": datetime.fromtimestamp(int(entry[0])),
                "Open": float(entry[1]),
                "Close": float(entry[2]),
                "High": float(entry[3]),
                "Low": float(entry[4]),
                "Volume": float(entry[5]),
            }
        )

    df = pd.DataFrame(rows)
    df = df.sort_values("Timestamp").reset_index(drop=True)
    return df


def fetch_klines(
    symbol: str,
    timeframe: str = "1day",
    base_url: str = BASE_URL,
) -> pd.DataFrame:
    """Fetch OHLCV klines from KuCoin REST API.

    Args:
        symbol: e.g. "BTC-USDT"
        timeframe: "1min", "3min", "5min", "15min", "30min", "1hour",
                   "2hour", "4hour", "6hour", "8hour", "12hour",
                   "1day", "1week"
        base_url: KuCoin API base URL

    Returns:
        DataFrame with columns: Timestamp, Open, Close, High, Low, Volume

    Raises:
        KuCoinFetchError: on HTTP errors or API error responses
    """
    url = f"{base_url}/api/v1/market/candles"
    params = {"type": timeframe, "symbol": symbol}

    resp = requests.get(url, params=params, timeout=30)

    if resp.status_code != 200:
        raise KuCoinFetchError(
            f"HTTP {resp.status_code} fetching klines for {symbol}: {resp.text[:200]}"
        )

    body = resp.json()
    code = body.get("code", "")

    if code != "200000":
        msg = body.get("msg", "Unknown error")
        raise KuCoinFetchError(f"KuCoin API error for {symbol}: {msg}")

    return _parse_kline_response(body)
