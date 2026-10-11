"""Console layout: timestamp-only formatter and per-ticker tree glyphs."""
import io
import logging
import re
from unittest import mock

import pytest

from analyzer.config import Config
from analyzer.fetchers.kucoin import KuCoinFetchError, _parse_kline_response
from analyzer.main import _TimeFormatter, analyze_ticker
from analyzer.models.types import Signal

MOCK_KLINE_RAW = {
    "code": "200000",
    "data": [
        [str(1720000000 + i * 86400), "100.0", "101.0", "102.0", "99.0", "105.0", "1500.0"]
        for i in range(200)
    ],
}

ANSI = re.compile("\x1b\\[[0-9;]*m")


@pytest.fixture
def stream():
    logger = logging.getLogger("analyzer.main")
    old_handlers, old_propagate, old_level = (
        logger.handlers,
        logger.propagate,
        logger.level,
    )
    buf = io.StringIO()
    handler = logging.StreamHandler(buf)
    handler.setFormatter(_TimeFormatter())
    logger.handlers = [handler]
    logger.setLevel(logging.INFO)
    logger.propagate = False
    try:
        yield buf
    finally:
        logger.handlers = old_handlers
        logger.propagate = old_propagate
        logger.setLevel(old_level)


def _plain(buf: io.StringIO) -> str:
    return ANSI.sub("", buf.getvalue())


def test_mid_ticker_uses_branch_glyph(stream):
    mock_redis = mock.MagicMock()
    with mock.patch("analyzer.main.fetch_klines") as mock_fetch:
        mock_fetch.return_value = _parse_kline_response(MOCK_KLINE_RAW)
        analyze_ticker("BTC/USDT", Config(), mock_redis, branch="├")
    assert re.search(
        r"^\d{2}:\d{2}:\d{2}  ├ BTC/USDT\s+\$[\d,]+\.\d{2}\s+.*\d+c\s+\d+▲ \d+▼\s+\d+ms$",
        _plain(stream),
        re.M,
    ), _plain(stream)


def test_last_ticker_uses_corner_glyph(stream):
    mock_redis = mock.MagicMock()
    with mock.patch("analyzer.main.fetch_klines") as mock_fetch:
        mock_fetch.return_value = _parse_kline_response(MOCK_KLINE_RAW)
        analyze_ticker("BTC/USDT", Config(), mock_redis, branch="└")
    assert re.search(r"^\d{2}:\d{2}:\d{2}  └ BTC/USDT ", _plain(stream), re.M), _plain(stream)


def test_branch_defaults_to_last(stream):
    mock_redis = mock.MagicMock()
    with mock.patch("analyzer.main.fetch_klines") as mock_fetch:
        mock_fetch.return_value = _parse_kline_response(MOCK_KLINE_RAW)
        analyze_ticker("BTC/USDT", Config(), mock_redis)
    assert "└ BTC/USDT" in _plain(stream)


def test_signal_row_is_nested_under_ticker(stream):
    signal = Signal(
        ticker="BTC/USDT",
        action="buy",
        confidence=0.6,
        entry_price=83086.5,
        sl_price=78899.8,
        tp_price=89366.55,
        reason="hidden_bullish_divergence",
    )
    mock_redis = mock.MagicMock()
    with (
        mock.patch("analyzer.main.fetch_klines") as mock_fetch,
        mock.patch("analyzer.main.generate_signals", return_value=[signal]),
    ):
        mock_fetch.return_value = _parse_kline_response(MOCK_KLINE_RAW)
        analyze_ticker("BTC/USDT", Config(), mock_redis, branch="├")
    assert re.search(
        r"^\d{2}:\d{2}:\d{2}  │  └ ▲ BUY  conf 0.60  @\$83,086\.50  "
        r"SL \$78,899\.80  TP \$89,366\.55  hidden_bullish_divergence$",
        _plain(stream),
        re.M,
    ), _plain(stream)
    mock_redis.publish.assert_called_once()


def test_fetch_error_renders_error_row(stream):
    mock_redis = mock.MagicMock()
    with mock.patch("analyzer.main.fetch_klines", side_effect=KuCoinFetchError("boom")):
        analyze_ticker("BTC/USDT", Config(), mock_redis, branch="├")
    out = _plain(stream)
    assert re.search(
        r"^\d{2}:\d{2}:\d{2}  ├ BTC/USDT\s+ERROR\s+fetch failed — boom$", out, re.M
    ), out
    mock_redis.publish.assert_not_called()


def test_unexpected_error_keeps_traceback(stream):
    mock_redis = mock.MagicMock()
    with mock.patch("analyzer.main.fetch_klines", side_effect=ValueError("kaboom")):
        analyze_ticker("BTC/USDT", Config(), mock_redis, branch="├")
    out = _plain(stream)
    assert "unexpected error — kaboom" in out
    assert "Traceback" in out
    assert "ValueError: kaboom" in out
    mock_redis.publish.assert_not_called()


def test_insufficient_candles_renders_warn_row(stream):
    short = {"code": "200000", "data": MOCK_KLINE_RAW["data"][:5]}
    mock_redis = mock.MagicMock()
    with mock.patch("analyzer.main.fetch_klines") as mock_fetch:
        mock_fetch.return_value = _parse_kline_response(short)
        analyze_ticker("BTC/USDT", Config(), mock_redis, branch="├")
    out = _plain(stream)
    assert re.search(
        r"^\d{2}:\d{2}:\d{2}  ├ BTC/USDT\s+WARN\s+only \d+ candles \(need >14\)$",
        out,
        re.M,
    ), out
    mock_redis.publish.assert_not_called()
