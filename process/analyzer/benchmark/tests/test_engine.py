import pytest

from analyzer.benchmark.engine import (
    STOP_LOSS,
    TAKE_PROFIT,
    evaluate_exit,
    evaluate_exit_candle,
    order_budget,
)


def test_order_budget_is_half_the_fund():
    assert order_budget(1000.0) == 500.0


def test_evaluate_exit_long_exits_on_tp_and_sl():
    tp, sl = 110.0, 90.0
    assert evaluate_exit("buy", 110.0, tp, sl) == TAKE_PROFIT
    assert evaluate_exit("buy", 120.0, tp, sl) == TAKE_PROFIT
    assert evaluate_exit("buy", 90.0, tp, sl) == STOP_LOSS
    assert evaluate_exit("buy", 100.0, tp, sl) is None


def test_evaluate_exit_short_exits_on_tp_and_sl():
    tp, sl = 90.0, 110.0
    assert evaluate_exit("sell", 90.0, tp, sl) == TAKE_PROFIT
    assert evaluate_exit("sell", 110.0, tp, sl) == STOP_LOSS
    assert evaluate_exit("sell", 100.0, tp, sl) is None


def test_evaluate_exit_unknown_side_never_exits():
    assert evaluate_exit("hold", 999.0, 1.0, 2.0) is None


def test_evaluate_exit_candle_checks_tp_before_sl():
    # Candle spans both levels: the engine looks at TP first.
    assert evaluate_exit_candle("buy", high=115.0, low=85.0, tp=110.0, sl=90.0) == TAKE_PROFIT


def test_evaluate_exit_candle_long_hits_sl():
    assert evaluate_exit_candle("buy", high=101.0, low=89.0, tp=110.0, sl=90.0) == STOP_LOSS


def test_evaluate_exit_candle_short_inverts_levels():
    assert evaluate_exit_candle("sell", high=115.0, low=85.0, tp=90.0, sl=110.0) == TAKE_PROFIT
    assert evaluate_exit_candle("sell", high=115.0, low=95.0, tp=90.0, sl=110.0) == STOP_LOSS


def test_evaluate_exit_candle_flat_is_none():
    assert evaluate_exit_candle("buy", high=105.0, low=95.0, tp=110.0, sl=90.0) is None


def test_evaluate_exit_candle_boundary_equality_hits():
    # Reaching the level exactly counts as a hit.
    assert evaluate_exit_candle("buy", high=110.0, low=95.0, tp=110.0, sl=90.0) == TAKE_PROFIT
    assert evaluate_exit_candle("buy", high=105.0, low=90.0, tp=110.0, sl=90.0) == STOP_LOSS


def test_evaluate_exit_candle_unknown_side_never_exits():
    assert evaluate_exit_candle("hold", high=999.0, low=0.0, tp=1.0, sl=2.0) is None


from analyzer.benchmark.engine import cap_loss, close_credit, realized_pnl


def test_realized_pnl_is_net_of_fees_for_both_sides():
    assert realized_pnl("buy", 100.0, 120.0, 2.0) == pytest.approx(18.0)
    assert realized_pnl("sell", 100.0, 80.0, 2.0) == pytest.approx(18.0)


def test_realized_pnl_short_can_lose_more_than_capital():
    assert realized_pnl("sell", 100.0, 250.0, 2.0) == pytest.approx(-152.0)


def test_cap_loss_is_capped_at_committed_capital():
    assert cap_loss(18.0, 100.0) == pytest.approx(18.0)
    assert cap_loss(-50.0, 100.0) == pytest.approx(-50.0)
    assert cap_loss(-250.0, 100.0) == pytest.approx(-100.0)


def test_close_credit_is_entry_fund_plus_pnl():
    assert close_credit(100.0, 18.0) == pytest.approx(118.0)
    assert close_credit(100.0, -50.0) == pytest.approx(50.0)
