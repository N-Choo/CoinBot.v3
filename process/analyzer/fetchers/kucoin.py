import time
from datetime import datetime

import pandas as pd
import requests

BASE_URL = "https://api.kucoin.com"
MAX_KLINES = 1500
TIMEFRAME_SECONDS = {
    "1min": 60,
    "3min": 180,
    "5min": 300,
    "15min": 900,
    "30min": 1800,
    "1hour": 3600,
    "2hour": 7200,
    "4hour": 14400,
    "6hour": 21600,
    "8hour": 28800,
    "12hour": 43200,
    "1day": 86400,
    "1week": 604800,
}


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
    timeframe: str = "4hour",
    base_url: str = BASE_URL,
    limit: int | None = None,
) -> pd.DataFrame:
    """Fetch OHLCV klines from KuCoin REST API.

    These candles are the shared spot/margin market data: KuCoin margin trades
    on the same order book as spot and exposes no separate margin kline feed, so
    the positions we open in margin are analyzed from this feed.

    Args:
        symbol: KuCoin margin pair, e.g. "BTC-USDT"
        timeframe: "1min", "3min", "5min", "15min", "30min", "1hour",
                   "2hour", "4hour", "6hour", "8hour", "12hour",
                   "1day", "1week"
        base_url: KuCoin API base URL
        limit: optional number of candles to request. Without it KuCoin returns
            its default 100 candles; with it a ``startAt``/``endAt`` window is
            sent and at most :data:`MAX_KLINES` candles are requested.

    Returns:
        DataFrame with columns: Timestamp, Open, Close, High, Low, Volume

    Raises:
        KuCoinFetchError: on HTTP errors or API error responses
        ValueError: for a non-positive limit or an unsupported timeframe
    """
    url = f"{base_url}/api/v1/market/candles"
    params = {"type": timeframe, "symbol": symbol}

    if limit is not None:
        if limit <= 0:
            raise ValueError("limit must be positive")
        seconds = TIMEFRAME_SECONDS.get(timeframe)
        if seconds is None:
            raise ValueError(f"Unknown timeframe for limit: {timeframe!r}")
        end_at = int(time.time())
        params["endAt"] = end_at
        params["startAt"] = end_at - min(limit, MAX_KLINES) * seconds

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
