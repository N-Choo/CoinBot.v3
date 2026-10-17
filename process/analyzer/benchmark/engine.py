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


def realized_pnl(
    entry_side: str,
    entry_basis: float,
    close_proceeds: float,
    fees_quote: float,
) -> float:
    """Realized PnL of a closed round trip, in quote currency, net of fees.

    For a short, ``entry_basis`` is the proceeds of selling the borrowed base;
    it profits when the close is cheaper.
    """
    if entry_side == "sell":
        return entry_basis - close_proceeds - fees_quote
    return close_proceeds - entry_basis - fees_quote


def cap_loss(raw_pnl: float, committed: float) -> float:
    """A loss cannot exceed the capital committed to the round trip."""
    return max(raw_pnl, -committed)


def close_credit(entry_funds: float, pnl: float) -> float:
    """Credit returned to the contract on close: committed fund plus PnL."""
    return entry_funds + pnl
