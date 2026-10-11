import json
from unittest import mock
from datetime import datetime

import pandas as pd
from analyzer.fetchers.kucoin import (
    fetch_klines,
    KuCoinFetchError,
    _parse_kline_response,
)


def test_parse_kline_response_converts_fields():
    # KuCoin kline format: [time, open, close, high, low, volume, turnover]
    raw = {
        "code": "200000",
        "data": [
            ["1720000000", "100.5", "102.0", "101.0", "99.0", "105.0", "1500.5"],
            ["1720000001", "101.0", "103.0", "102.0", "100.0", "106.0", "2000.0"],
        ],
    }

    df = _parse_kline_response(raw)

    assert len(df) == 2
    assert df.iloc[0]["Open"] == 100.5
    assert df.iloc[0]["High"] == 101.0
    assert df.iloc[0]["Low"] == 99.0
    assert df.iloc[0]["Close"] == 102.0
    assert df.iloc[0]["Volume"] == 105.0
    assert df.iloc[1]["Close"] == 103.0
    assert df.iloc[1]["Volume"] == 106.0


def test_fetch_klines_returns_dataframe():
    # KuCoin kline format: [time, open, close, high, low, volume, turnover]
    mock_response = {
        "code": "200000",
        "data": [
            ["1720000000", "100.0", "102.0", "101.0", "99.0", "105.0", "1500.0"],
            ["1720000001", "101.0", "103.0", "102.0", "100.0", "106.0", "2000.0"],
        ],
    }

    with mock.patch("requests.get") as mock_get:
        mock_get.return_value.status_code = 200
        mock_get.return_value.json.return_value = mock_response

        df = fetch_klines("BTC-USDT", "1day")

    assert len(df) == 2
    assert list(df.columns) == [
        "Timestamp", "Open", "Close", "High", "Low", "Volume"
    ]


def test_fetch_klines_raises_on_error_response():
    with mock.patch("requests.get") as mock_get:
        mock_get.return_value.status_code = 200
        mock_get.return_value.json.return_value = {
            "code": "400000",
            "msg": "Invalid symbol",
        }

        try:
            fetch_klines("INVALID", "1day")
            assert False, "Expected KuCoinFetchError"
        except KuCoinFetchError as e:
            assert "Invalid symbol" in str(e)


def test_fetch_klines_raises_on_http_error():
    with mock.patch("requests.get") as mock_get:
        mock_get.return_value.status_code = 500
        mock_get.return_value.text = "Internal Server Error"

        try:
            fetch_klines("BTC-USDT", "1day")
            assert False, "Expected KuCoinFetchError"
        except KuCoinFetchError:
            pass
