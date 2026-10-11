from dataclasses import dataclass, field
from datetime import datetime


@dataclass
class Signal:
    ticker: str
    action: str
    confidence: float
    entry_price: float
    reason: str = ""
    sl_price: float = 0.0
    tp_price: float = 0.0

    def to_dict(self) -> dict:
        return {
            "ticker": self.ticker,
            "action": self.action,
            "confidence": self.confidence,
            "entry_price": self.entry_price,
            "reason": self.reason,
            "sl_price": self.sl_price,
            "tp_price": self.tp_price,
        }


@dataclass
class OHLCV:
    timestamp: datetime
    open: float
    high: float
    low: float
    close: float
    volume: float
