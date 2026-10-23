#![cfg(feature = "harness")]

use analyzer::harness::sim::{simulate, SimSignal, REASON_REVERSAL};
use analyzer::strategy::types::{Candle, Direction, Signal};
use rust_decimal::Decimal;

fn d(value: i64) -> Decimal {
    Decimal::from(value)
}

fn candle(i: usize, open: i64, high: i64, low: i64, close: i64) -> Candle {
    Candle {
        open_time_ms: i as i64 * 86_400_000,
        open: d(open),
        high: d(high),
        low: d(low),
        close: d(close),
        volume: d(1),
    }
}

fn signal(
    index: usize,
    side: Direction,
    entry: i64,
    sl: Option<i64>,
    tp: Option<i64>,
) -> SimSignal {
    SimSignal {
        candle_index: index,
        signal_id: format!("sig-{index}-{}", entry),
        signal: Signal {
            ticker: "TEST-USDT".into(),
            action: side,
            confidence: Decimal::ONE,
            entry_price: d(entry),
            reason: if side == Direction::Long {
                "rsi_long_cross"
            } else {
                "rsi_short_cross"
            }
            .into(),
            sl_price: sl.map(d),
            tp_price: tp.map(d),
            signal_candle_open_time_ms: index as i64 * 86_400_000,
        },
    }
}

#[test]
fn long_with_no_levels_is_held_until_a_reversal() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 101, 102, 100, 101),
        candle(2, 90, 91, 89, 90),
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Long, 100, None, None),
            signal(1, Direction::Short, 101, None, None),
        ],
        1000,
        0,
    );

    assert_eq!(outcome.trades.len(), 1);
    let trade = &outcome.trades[0];
    assert_eq!(trade.reason, REASON_REVERSAL);
    assert_eq!(trade.side, Direction::Long);
    // The entry fills at candle 1's open (101), not at the signal entry (100).
    assert_eq!(trade.fill, d(101));
    assert_eq!(trade.exit, d(90));
    assert_eq!(trade.e, d(500));
    // Realized PnL marks from the fill: 500 * exit / fill - 500.
    let expected_pnl = d(500) * (d(90) / d(101)) - d(500);
    assert_eq!(trade.pnl, expected_pnl);

    // Closing the long releases its fund, then the reversal opens the short.
    // Mirror the simulator's addition order so Decimal rounding matches exactly.
    let expected_available = d(500) + (d(500) + expected_pnl) - d(500);
    assert_eq!(outcome.available_fund, expected_available);
    assert_eq!(outcome.open_positions.len(), 1);
    assert_eq!(outcome.open_positions[0].side, Direction::Short);
    assert_eq!(outcome.open_positions[0].fill, d(90));
    assert_eq!(outcome.skipped_funds, 0);

    let last = outcome.account.last().expect("account step");
    assert_eq!(last.index, 2);
    assert_eq!(last.available_fund, expected_available);
    assert_eq!(last.reserved, d(500));
    assert_eq!(last.unrealized, Decimal::ZERO);
    assert_eq!(last.equity, expected_available + d(500));
}

#[test]
fn short_with_no_levels_reverses_at_the_open_and_reports_pnl() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 90, 91, 89, 90),
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Short, 100, None, None),
            signal(1, Direction::Long, 100, None, None),
        ],
        1000,
        0,
    );

    assert_eq!(outcome.trades.len(), 1);
    let trade = &outcome.trades[0];
    assert_eq!(trade.reason, REASON_REVERSAL);
    assert_eq!(trade.side, Direction::Short);
    // The short fills at candle 1's open (100), matching the signal entry here.
    assert_eq!(trade.fill, d(100));
    assert_eq!(trade.exit, d(90));
    assert_eq!(trade.e, d(500));
    assert_eq!(trade.pnl, d(50));

    assert_eq!(outcome.available_fund, d(550));
    assert_eq!(outcome.open_positions.len(), 1);
    assert_eq!(outcome.open_positions[0].side, Direction::Long);
    assert_eq!(outcome.open_positions[0].fill, d(90));

    let last = outcome.account.last().expect("account step");
    assert_eq!(last.index, 2);
    assert_eq!(last.available_fund, d(550));
    assert_eq!(last.reserved, d(500));
    assert_eq!(last.unrealized, Decimal::ZERO);
    assert_eq!(last.equity, d(1050));
}

#[test]
fn same_side_signal_is_ignored() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 101, 102, 100, 101),
        candle(2, 102, 103, 101, 102),
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Long, 100, None, None),
            signal(1, Direction::Long, 101, None, None),
        ],
        1000,
        0,
    );

    assert_eq!(outcome.trades.len(), 0);
    assert_eq!(outcome.open_positions.len(), 1);
    assert_eq!(outcome.open_positions[0].fill, d(101));
    assert_eq!(outcome.available_fund, d(500));
    assert_eq!(outcome.skipped_funds, 0);
}

#[test]
fn long_entry_fills_at_next_open_and_exits_at_tp() {
    let candles = vec![candle(0, 100, 101, 99, 100), candle(1, 101, 106, 100, 105)];
    let outcome = simulate(
        &candles,
        &[signal(0, Direction::Long, 100, Some(98), Some(104))],
        1000,
        0,
    );

    assert_eq!(outcome.trades.len(), 1);
    assert_eq!(outcome.trades[0].fill, d(101));
    assert_eq!(outcome.trades[0].exit, d(104));
    assert_eq!(outcome.trades[0].reason, "TP");
}

#[test]
fn short_pnl_is_profitable_and_losing() {
    let candles = vec![candle(0, 100, 101, 99, 100), candle(1, 100, 101, 94, 95)];
    let profitable = simulate(
        &candles,
        &[signal(0, Direction::Short, 100, Some(105), Some(95))],
        1000,
        0,
    );
    assert_eq!(profitable.trades.len(), 1);
    assert!(profitable.trades[0].pnl > Decimal::ZERO);

    let candles = vec![candle(0, 100, 101, 99, 100), candle(1, 100, 106, 99, 105)];
    let losing = simulate(
        &candles,
        &[signal(0, Direction::Short, 100, Some(105), Some(95))],
        1000,
        0,
    );
    assert_eq!(losing.trades.len(), 1);
    assert!(losing.trades[0].pnl < Decimal::ZERO);
}

#[test]
fn sl_first_when_both_levels_are_in_range() {
    let candles = vec![candle(0, 100, 101, 99, 100), candle(1, 100, 106, 97, 105)];
    let outcome = simulate(
        &candles,
        &[signal(0, Direction::Long, 100, Some(98), Some(104))],
        1000,
        0,
    );

    assert_eq!(outcome.trades[0].reason, "SL");
    assert_eq!(outcome.trades[0].exit, d(98));
}

#[test]
fn gap_beyond_the_stop_fills_at_the_open_for_long_and_short() {
    let candles = vec![candle(0, 100, 101, 99, 100), candle(1, 95, 96, 90, 92)];
    let long = simulate(
        &candles,
        &[signal(0, Direction::Long, 100, Some(98), Some(110))],
        1000,
        0,
    );
    assert_eq!(long.trades[0].reason, "gap_sl");
    assert_eq!(long.trades[0].fill, d(95));
    assert_eq!(long.trades[0].exit, d(95));

    let candles = vec![candle(0, 100, 101, 99, 100), candle(1, 106, 107, 105, 106)];
    let short = simulate(
        &candles,
        &[signal(0, Direction::Short, 100, Some(105), Some(90))],
        1000,
        0,
    );
    assert_eq!(short.trades[0].reason, "gap_sl");
    assert_eq!(short.trades[0].fill, d(106));
    assert_eq!(short.trades[0].exit, d(106));
}

#[test]
fn gap_beyond_the_target_fills_at_the_open_for_long_and_short() {
    let candles = vec![candle(0, 100, 101, 99, 100), candle(1, 106, 107, 105, 106)];
    let long = simulate(
        &candles,
        &[signal(0, Direction::Long, 100, Some(98), Some(104))],
        1000,
        0,
    );
    assert_eq!(long.trades[0].reason, "gap_tp");
    assert_eq!(long.trades[0].fill, d(106));
    assert_eq!(long.trades[0].exit, d(106));

    let candles = vec![candle(0, 100, 101, 99, 100), candle(1, 90, 91, 89, 90)];
    let short = simulate(
        &candles,
        &[signal(0, Direction::Short, 100, Some(105), Some(95))],
        1000,
        0,
    );
    assert_eq!(short.trades[0].reason, "gap_tp");
    assert_eq!(short.trades[0].fill, d(90));
    assert_eq!(short.trades[0].exit, d(90));
}

#[test]
fn loss_is_capped_at_the_committed_fund() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 250, 260, 240, 250),
    ];
    let outcome = simulate(
        &candles,
        &[signal(0, Direction::Short, 100, Some(200), Some(50))],
        1000,
        0,
    );

    assert_eq!(outcome.trades.len(), 1);
    assert_eq!(outcome.trades[0].reason, "gap_sl");
    assert_eq!(outcome.trades[0].e, d(500));
    assert_eq!(outcome.trades[0].pnl, d(-500));
    assert_eq!(outcome.available_fund, d(500));
}

#[test]
fn same_side_signals_do_not_overcommit_funds() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 100, 101, 99, 100),
        candle(3, 100, 101, 99, 100),
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Long, 100, Some(90), Some(120)),
            signal(1, Direction::Long, 100, Some(90), Some(120)),
            signal(2, Direction::Long, 100, Some(90), Some(120)),
        ],
        1000,
        0,
    );

    assert_eq!(outcome.trades.len(), 0);
    assert_eq!(outcome.open_positions.len(), 1);
    assert_eq!(outcome.available_fund, d(500));
    // The engine holds at most one position per side and ignores same-side
    // signals, so it never overcommits the budget or skips a funded entry.
    assert_eq!(outcome.skipped_funds, 0);

    let last = outcome.account.last().expect("account step");
    assert_eq!(last.available_fund, d(500));
    assert_eq!(last.reserved, d(500));
}

#[test]
fn reversal_releases_funds_for_the_target_side_without_skipping() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 100, 106, 99, 105),
        candle(3, 100, 101, 99, 100),
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Long, 100, Some(90), Some(105)),
            signal(2, Direction::Short, 100, None, None),
        ],
        1000,
        0,
    );

    assert_eq!(outcome.trades.len(), 1);
    assert_eq!(outcome.trades[0].entry_index, 1);
    assert_eq!(outcome.trades[0].exit_index, 2);
    assert_eq!(outcome.open_positions.len(), 1);
    assert_eq!(outcome.open_positions[0].entry_index, 3);
    assert_eq!(outcome.available_fund, d(525));
    assert_eq!(outcome.skipped_funds, 0);
}

#[test]
fn signal_on_the_final_candle_is_unfilled_at_end() {
    let candles = vec![candle(0, 100, 101, 99, 100), candle(1, 100, 101, 99, 100)];
    let outcome = simulate(
        &candles,
        &[signal(1, Direction::Long, 100, Some(98), Some(104))],
        1000,
        0,
    );

    assert!(outcome.trades.is_empty());
    assert!(outcome.open_positions.is_empty());
    assert_eq!(outcome.unfilled_at_end, 1);
}

#[test]
fn risk_reward_uses_the_fill_and_is_none_after_a_gap() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 100, 111, 99, 110),
    ];
    let normal = simulate(
        &candles,
        &[signal(0, Direction::Long, 100, Some(95), Some(110))],
        1000,
        0,
    );
    assert_eq!(normal.trades[0].risk_reward, Some(d(2)));

    let candles = vec![candle(0, 100, 101, 99, 100), candle(1, 106, 107, 105, 106)];
    let gapped = simulate(
        &candles,
        &[signal(0, Direction::Long, 100, Some(98), Some(104))],
        1000,
        0,
    );
    assert_eq!(gapped.trades[0].reason, "gap_tp");
    assert_eq!(gapped.trades[0].risk_reward, None);
}

#[test]
fn trades_and_positions_carry_signal_id() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 101, 102, 100, 101),
        candle(2, 105, 106, 104, 105),
    ];
    let mut long = signal(0, Direction::Long, 100, None, None);
    long.signal_id = "sig-a".into();
    let mut short = signal(1, Direction::Short, 105, None, None);
    short.signal_id = "sig-b".into();

    let outcome = simulate(&candles, &[long, short], 1000, 0);
    assert_eq!(outcome.trades[0].signal_id, "sig-a");
    assert_eq!(outcome.open_positions[0].signal_id, "sig-b");
}

#[test]
fn account_series_is_consistent_at_every_step() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 101, 102, 100, 101),
        candle(2, 105, 106, 104, 105),
    ];
    let mut first = signal(0, Direction::Long, 100, None, None);
    first.signal_id = "sig-a".into();
    let mut second = signal(1, Direction::Short, 105, None, None);
    second.signal_id = "sig-b".into();

    let outcome = simulate(&candles, &[first, second], 1000, 0);
    assert_eq!(outcome.account.len(), 3);
    for step in &outcome.account {
        assert_eq!(
            step.equity,
            step.available_fund + step.reserved + step.unrealized
        );
    }
    let last = outcome.account.last().unwrap();
    assert_eq!(last.open.len(), outcome.open_positions.len());
}

#[test]
fn summary_stats_use_realized_pnl() {
    use analyzer::harness::stats::summarize;

    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 101, 106, 100, 105),
        candle(2, 105, 106, 94, 95),
        candle(3, 95, 100, 94, 99),
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Long, 100, Some(98), Some(104)),
            signal(1, Direction::Short, 105, Some(108), Some(95)),
        ],
        1000,
        0,
    );
    let summary = summarize(Decimal::from(1000), &outcome, Decimal::from(99), 3, 4);
    assert_eq!(summary.trades, 2);
    assert_eq!(summary.wins + summary.losses + summary.breakeven, 2);
    assert!(summary.max_drawdown >= Decimal::ZERO);
    assert!(summary.time_in_market_pct >= Decimal::ZERO);
}

#[test]
fn summary_reports_exact_risk_and_exposure_metrics() {
    use analyzer::harness::stats::summarize;

    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 100, 101, 79, 80),
        candle(3, 100, 101, 99, 100),
        candle(4, 100, 111, 99, 110),
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Long, 100, Some(80), Some(110)),
            signal(2, Direction::Long, 100, Some(80), Some(110)),
        ],
        1000,
        0,
    );
    let summary = summarize(Decimal::from(1000), &outcome, d(110), 4, 5);

    assert_eq!(summary.trades, 2);
    assert_eq!(summary.wins, 1);
    assert_eq!(summary.losses, 1);
    assert_eq!(summary.breakeven, 0);
    assert_eq!(summary.profit_factor, Some(Decimal::new(5, 1)));
    assert!(!summary.profit_factor_is_infinite);
    assert_eq!(summary.avg_win, d(50));
    assert_eq!(summary.avg_loss, d(-100));
    assert_eq!(summary.max_drawdown, d(100));
    assert_eq!(summary.max_drawdown_pct, d(10));
    assert_eq!(summary.exposure_candles, 2);
    assert_eq!(summary.time_in_market_pct, d(40));
}

#[test]
fn exposure_counts_closed_and_open_positions() {
    use analyzer::harness::stats::summarize;

    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 100, 101, 99, 100),
        candle(3, 100, 101, 99, 100),
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Long, 100, None, None),
            signal(1, Direction::Short, 100, None, None),
        ],
        1000,
        0,
    );
    let summary = summarize(Decimal::from(1000), &outcome, Decimal::from(100), 3, 4);

    assert_eq!(summary.trades, 1);
    assert_eq!(summary.open_count, 1);
    assert_eq!(summary.exposure_candles, 2);
    assert_eq!(summary.time_in_market_pct, d(50));
}

#[test]
fn max_drawdown_percentage_uses_the_peak_at_the_worst_drop() {
    use analyzer::harness::stats::summarize;

    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 151, 99, 150),
        candle(2, 100, 101, 89, 90),
        candle(3, 100, 161, 99, 160),
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Long, 100, Some(90), Some(150)),
            signal(1, Direction::Long, 100, Some(90), Some(160)),
            signal(2, Direction::Long, 100, Some(90), Some(160)),
        ],
        1000,
        0,
    );
    let summary = summarize(Decimal::from(1000), &outcome, Decimal::from(160), 3, 4);

    assert_eq!(summary.trades, 3);
    assert_eq!(summary.wins, 2);
    assert_eq!(summary.losses, 1);
    assert_eq!(summary.max_drawdown, d(50));
    assert_eq!(summary.max_drawdown_pct, d(4));
}

#[test]
fn win_rate_counts_breakeven_trades_in_the_denominator() {
    use analyzer::harness::stats::summarize;

    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 100, 111, 99, 110),
        candle(3, 100, 101, 99, 100),
        candle(4, 100, 101, 94, 95),
        candle(5, 100, 101, 99, 100),
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Long, 100, Some(95), Some(110)),
            signal(2, Direction::Long, 100, Some(95), Some(120)),
            signal(4, Direction::Long, 99, Some(98), Some(100)),
        ],
        1000,
        0,
    );
    assert_eq!(outcome.trades.len(), 3);
    assert_eq!(outcome.trades[0].pnl, d(50));
    assert_eq!(outcome.trades[1].pnl, d(-25));
    assert_eq!(outcome.trades[2].pnl, Decimal::ZERO);

    let summary = summarize(Decimal::from(1000), &outcome, d(100), 5, 6);
    let expected_win_rate = Decimal::from(1) / Decimal::from(3) * Decimal::from(100);
    assert_eq!(summary.wins, 1);
    assert_eq!(summary.losses, 1);
    assert_eq!(summary.breakeven, 1);
    assert_eq!(summary.win_rate, expected_win_rate);
}

#[test]
fn exit_classification_and_avg_risk_reward() {
    use analyzer::harness::stats::summarize;

    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 101, 106, 100, 105),
        candle(2, 105, 106, 94, 95),
        candle(3, 95, 100, 90, 92),
    ];
    let mut a = signal(0, Direction::Long, 100, Some(98), Some(104));
    a.signal_id = "sig-a".into();
    let mut b = signal(2, Direction::Long, 95, Some(90), Some(120));
    b.signal_id = "sig-b".into();

    let outcome = simulate(&candles, &[a, b], 1000, 0);
    let summary = summarize(Decimal::from(1000), &outcome, Decimal::from(92), 3, 4);

    assert_eq!(summary.tp_hits, 1);
    assert_eq!(summary.sl_hits, 1);
    assert_eq!(summary.other_exits, 0);
    assert_eq!(summary.avg_risk_reward, Some(d(3)));
}

#[test]
fn avg_risk_reward_is_none_without_levels() {
    use analyzer::harness::stats::summarize;

    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 100, 101, 99, 100),
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Long, 100, None, None),
            signal(1, Direction::Short, 100, None, None),
        ],
        1000,
        0,
    );
    let summary = summarize(Decimal::from(1000), &outcome, Decimal::from(100), 2, 3);

    assert_eq!(summary.trades, 1);
    assert_eq!(summary.tp_hits, 0);
    assert_eq!(summary.sl_hits, 0);
    assert_eq!(summary.other_exits, 1);
    assert_eq!(summary.avg_risk_reward, None);
}
