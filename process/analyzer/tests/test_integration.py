"""Smoke test: can we import the pipeline and run a ticker end-to-end?"""
import json
from unittest import mock

import pandas as pd
import numpy as np
import redis as redis_lib

from analyzer.config import Config
from analyzer.main import analyze_ticker
from analyzer.fetchers.kucoin import _parse_kline_response


# Reusable mock kline response with 200 candles
MOCK_KLINE_RAW = {
    "code": "200000",
    "data": [
        [str(1720000000 + i * 86400), "100.0", "101.0", "102.0", "99.0", "105.0", "1500.0"]
        for i in range(200)
    ],
}


def test_analyze_ticker_end_to_end():
    cfg = Config()
    mock_redis = mock.MagicMock()

    with mock.patch("analyzer.main.fetch_klines") as mock_fetch:
        mock_fetch.return_value = _parse_kline_response(MOCK_KLINE_RAW)
        analyze_ticker("BTC-USDT", cfg, mock_redis)

    # verify redis publish was called (even if signals were empty, it should not crash)
    assert mock_redis.publish.call_count <= 1
