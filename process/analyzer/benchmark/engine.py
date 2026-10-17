"""Pure port of the trade-engine's order/exit math plus a replay driver.

Mirrors process/trade-engine:
  order_budget    <- trader/sizing.rs
  evaluate_exit   <- position_exit/decision.rs
  evaluate_exit_candle <- position_exit/decision.rs (candle form)
  realized_pnl / cap_loss / close_credit <- reconciler/pnl.rs
  SimEngine       <- signal_handler + position_exit
"""

from __future__ import annotations

from dataclasses import dataclass
import math

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


@dataclass
class Position:
    """An open position simulated from one analyzer signal."""

    side: str
    entry_price: float
    E: float
    tp: float
    sl: float
    entry_idx: int
    entry_date: str


@dataclass
class ClosedTrade:
    """A realized round trip."""

    side: str
    entry_price: float
    exit_price: float
    E: float
    pnl: float
    pnl_pct: float
    reason: str
    entry_idx: int
    entry_date: str
    exit_idx: int
    exit_date: str


class SimEngine:
    """Replay driver for one contract: funds plus open positions.

    Faithful to `signal_handler` + `position_exit`: a position opens on every
    signal while `available_fund >= order_budget(init_fund)` (so at most two
    concurrent), and closes only when a candle's range reaches its TP or SL.
    """

    def __init__(self, init_fund: float, fee_rate: float) -> None:
        self.init_fund = init_fund
        self.available_fund = init_fund
        self.total_traded_fund = 0.0
        self.fee_rate = fee_rate
        self.positions: list[Position] = []
        self.closed: list[ClosedTrade] = []

    @property
    def budget(self) -> float:
        return order_budget(self.init_fund)

    def on_signal(
        self,
        side: str,
        entry_price: float,
        tp: float,
        sl: float,
        idx: int,
        date: str,
    ) -> bool:
        """Process one analyzer signal; open a position if funds allow."""
        if side not in ("buy", "sell"):
            return False
        if (
            not math.isfinite(entry_price)
            or not math.isfinite(tp)
            or not math.isfinite(sl)
            or entry_price <= 0
            or tp <= 0
            or sl <= 0
        ):
            return False
        budget = self.budget
        if self.available_fund < budget:
            return False
        self.available_fund -= budget
        self.total_traded_fund += budget
        self.positions.append(
            Position(side, entry_price, budget, tp, sl, idx, date)
        )
        return True

    def on_candle(self, idx: int, high: float, low: float, date: str) -> None:
        """Check open positions against candle ``idx`` and close TP/SL hits."""
        still_open: list[Position] = []
        for pos in self.positions:
            if pos.entry_idx >= idx:
                still_open.append(pos)
                continue
            reason = evaluate_exit_candle(pos.side, high, low, pos.tp, pos.sl)
            if reason is None:
                still_open.append(pos)
                continue
            self._close(pos, reason, idx, date)
        self.positions = still_open

    def _close(self, pos: Position, reason: str, idx: int, date: str) -> None:
        exit_price = pos.tp if reason == TAKE_PROFIT else pos.sl
        proceeds = pos.E * (exit_price / pos.entry_price)
        fees = self.fee_rate * pos.E + self.fee_rate * proceeds
        pnl = cap_loss(realized_pnl(pos.side, pos.E, proceeds, fees), pos.E)
        self.available_fund += close_credit(pos.E, pnl)
        self.closed.append(
            ClosedTrade(
                side=pos.side,
                entry_price=pos.entry_price,
                exit_price=exit_price,
                E=pos.E,
                pnl=pnl,
                pnl_pct=(pnl / pos.E * 100) if pos.E else 0.0,
                reason=reason,
                entry_idx=pos.entry_idx,
                entry_date=pos.entry_date,
                exit_idx=idx,
                exit_date=date,
            )
        )
