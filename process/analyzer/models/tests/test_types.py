from analyzer.models.types import Signal, OHLCV
from datetime import datetime


def test_signal_creation_defaults():
    s = Signal(
        ticker="BTC-USDT",
        action="buy",
        confidence=0.8,
        entry_price=45000.0,
    )
    assert s.ticker == "BTC-USDT"
    assert s.action == "buy"
    assert s.confidence == 0.8
    assert s.entry_price == 45000.0
    assert s.reason == ""
    assert s.sl_price == 0.0
    assert s.tp_price == 0.0


def test_signal_with_reason():
    s = Signal(
        ticker="ETH-USDT",
        action="sell",
        confidence=0.6,
        entry_price=3200.0,
        reason="regular_bearish_divergence",
    )
    assert s.reason == "regular_bearish_divergence"


def test_signal_to_dict():
    s = Signal(
        ticker="BTC-USDT",
        action="buy",
        confidence=0.8,
        entry_price=45000.0,
        reason="bullish_div",
    )
    d = s.to_dict()
    assert d == {
        "ticker": "BTC-USDT",
        "action": "buy",
        "confidence": 0.8,
        "entry_price": 45000.0,
        "reason": "bullish_div",
        "sl_price": 0.0,
        "tp_price": 0.0,
    }


def test_ohlcv_parses_float_fields():
    o = OHLCV(
        timestamp=datetime(2026, 1, 1),
        open=100.0,
        high=110.0,
        low=95.0,
        close=105.0,
        volume=1000.0,
    )
    assert o.close == 105.0
    assert o.high == 110.0
