import json
from unittest import mock

from analyzer.publisher.redis_pub import publish_signals
from analyzer.models.types import Signal


def test_publish_signals_calls_redis_publish():
    mock_redis = mock.MagicMock()
    signals = [
        Signal(
            ticker="BTC-USDT",
            action="buy",
            confidence=0.8,
            entry_price=45000.0,
            reason="bullish_div",
        ),
    ]

    publish_signals(mock_redis, signals, channel="signals:result")

    mock_redis.publish.assert_called_once()
    call_args = mock_redis.publish.call_args
    assert call_args[0][0] == "signals:result"

    payload = json.loads(call_args[0][1])
    assert len(payload) == 1
    assert payload[0]["ticker"] == "BTC-USDT"
    assert payload[0]["action"] == "buy"


def test_publish_signals_skips_on_empty_list():
    mock_redis = mock.MagicMock()
    publish_signals(mock_redis, [], channel="signals:result")
    mock_redis.publish.assert_not_called()


def test_publish_signals_handles_publish_error():
    mock_redis = mock.MagicMock()
    mock_redis.publish.side_effect = ConnectionError("Redis down")
    signals = [
        Signal(
            ticker="ETH-USDT",
            action="sell",
            confidence=0.6,
            entry_price=3000.0,
        ),
    ]

    try:
        publish_signals(mock_redis, signals, channel="signals:result")
    except ConnectionError:
        assert False, "Should not raise — publisher should swallow errors"
