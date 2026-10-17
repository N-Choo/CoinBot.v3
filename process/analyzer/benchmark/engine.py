"""Pure port of the trade-engine's order/exit math.

Mirrors process/trade-engine:
  order_budget    <- trader/sizing.rs
  evaluate_exit   <- position_exit/decision.rs
PnL math (reconciler/pnl.rs) and the SimEngine replay driver are added later.
"""

from __future__ import annotations

from dataclasses import dataclass

TAKE_PROFIT = "TP"
STOP_LOSS = "SL"


def order_budget(init_fund: float) -> float:
    """Per-order budget: half of a contract's initial fund."""
    return init_fund / 2


def evaluate_exit(side: str, last: float, tp: float, sl: float) -> str | None:
    """Decide whether ``side``'s position reached ``tp`` or ``sl`` at ``last``.

    Long (``buy``) exits high on TP / low on SL; short (``sell``) is inverted.
    TP is checked first, matching the engine.
    """
    if side == "buy":
        if last >= tp:
            return TAKE_PROFIT
        if last <= sl:
            return STOP_LOSS
    elif side == "sell":
        if last <= tp:
            return TAKE_PROFIT
        if last >= sl:
            return STOP_LOSS
    return None


def evaluate_exit_candle(
    side: str, high: float, low: float, tp: float, sl: float
) -> str | None:
    """Map the engine's last-price rule onto a daily candle's range.

    The engine polls last price continuously; over one candle the price spans
    ``[low, high]``, so a level is reached when the candle range crosses it.

    A candle exposes no intrabar path, so when both levels fall inside the
    range we assume TP first (the engine's own ordering) — a deliberate,
    optimistic simplification. Callers must only evaluate candles strictly
    *after* a position's entry candle; evaluating the entry candle itself
    would introduce look-ahead.
    """
    if side == "buy":
        if high >= tp:
            return TAKE_PROFIT
        if low <= sl:
            return STOP_LOSS
    elif side == "sell":
        if low <= tp:
            return TAKE_PROFIT
        if high >= sl:
            return STOP_LOSS
    return None
