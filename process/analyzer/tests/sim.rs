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

fn signal(index: usize, side: Direction, entry: i64) -> SimSignal {
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
            signal_candle_open_time_ms: index as i64 * 86_400_000,
        },
    }
}

#[test]
fn long_is_held_until_a_reversal() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 101, 102, 100, 101),
        candle(2, 90, 91, 89, 90),
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Long, 100),
            signal(1, Direction::Short, 101),
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
fn short_reverses_at_the_open_and_reports_pnl() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 90, 91, 89, 90),
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Short, 100),
            signal(1, Direction::Long, 100),
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
            signal(0, Direction::Long, 100),
            signal(1, Direction::Long, 101),
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
fn strict_alternation_accepts_only_the_first_of_two_long_signals_then_a_short() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 100, 101, 99, 100),
        candle(3, 90, 91, 89, 90),
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Long, 100),
            signal(1, Direction::Long, 100),
            signal(2, Direction::Short, 100),
        ],
        1000,
        0,
    );

    // The second long is rejected by strict alternation, so only the first
    // long was ever opened. The short then reverses it and is accepted.
    assert_eq!(outcome.trades.len(), 1);
    assert_eq!(outcome.trades[0].reason, REASON_REVERSAL);
    assert_eq!(outcome.trades[0].side, Direction::Long);
    assert_eq!(outcome.trades[0].signal_id, "sig-0-100");
    assert_eq!(outcome.trades[0].entry_index, 1);
    assert_eq!(outcome.trades[0].exit_index, 3);
    assert_eq!(outcome.open_positions.len(), 1);
    assert_eq!(outcome.open_positions[0].side, Direction::Short);
    assert_eq!(outcome.open_positions[0].signal_id, "sig-2-100");
    assert_eq!(outcome.open_positions[0].entry_index, 3);
    // The long lost 50 on the 500 committed, then the short committed 500.
    assert_eq!(outcome.available_fund, d(450));
    assert_eq!(outcome.skipped_funds, 0);
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
        &[
            signal(0, Direction::Short, 100),
            signal(1, Direction::Long, 250),
        ],
        1000,
        0,
    );

    assert_eq!(outcome.trades.len(), 1);
    let trade = &outcome.trades[0];
    assert_eq!(trade.reason, REASON_REVERSAL);
    assert_eq!(trade.e, d(500));
    assert_eq!(trade.pnl, d(-500));
    // The reversal then commits the remaining budget to the long.
    assert_eq!(outcome.available_fund, Decimal::ZERO);
    assert_eq!(outcome.open_positions.len(), 1);
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
            signal(0, Direction::Long, 100),
            signal(1, Direction::Long, 100),
            signal(2, Direction::Long, 100),
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
        candle(2, 100, 101, 99, 100),
        candle(3, 100, 101, 99, 100),
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Long, 100),
            signal(2, Direction::Short, 100),
        ],
        1000,
        0,
    );

    assert_eq!(outcome.trades.len(), 1);
    assert_eq!(outcome.trades[0].entry_index, 1);
    assert_eq!(outcome.trades[0].exit_index, 3);
    assert_eq!(outcome.open_positions.len(), 1);
    assert_eq!(outcome.open_positions[0].entry_index, 3);
    assert_eq!(outcome.available_fund, d(500));
    assert_eq!(outcome.skipped_funds, 0);
}

#[test]
fn signal_on_the_final_candle_is_unfilled_at_end() {
    let candles = vec![candle(0, 100, 101, 99, 100), candle(1, 100, 101, 99, 100)];
    let outcome = simulate(&candles, &[signal(1, Direction::Long, 100)], 1000, 0);

    assert!(outcome.trades.is_empty());
    assert!(outcome.open_positions.is_empty());
    assert_eq!(outcome.unfilled_at_end, 1);
}

#[test]
fn trades_and_positions_carry_signal_id() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 101, 102, 100, 101),
        candle(2, 105, 106, 104, 105),
    ];
    let mut long = signal(0, Direction::Long, 100);
    long.signal_id = "sig-a".into();
    let mut short = signal(1, Direction::Short, 105);
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
    let mut first = signal(0, Direction::Long, 100);
    first.signal_id = "sig-a".into();
    let mut second = signal(1, Direction::Short, 105);
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
            signal(0, Direction::Long, 100),
            signal(1, Direction::Short, 105),
            signal(2, Direction::Long, 95),
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
fn summary_reports_exact_metrics() {
    use analyzer::harness::stats::summarize;

    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 80, 81, 79, 80),
        candle(3, 72, 73, 71, 72),
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Long, 100),
            signal(1, Direction::Short, 80),
            signal(2, Direction::Long, 72),
        ],
        1000,
        0,
    );
    let summary = summarize(Decimal::from(1000), &outcome, d(72), 3, 4);

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
    assert_eq!(summary.time_in_market_pct, d(50));
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
            signal(0, Direction::Long, 100),
            signal(1, Direction::Short, 100),
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
fn max_drawdown_pct_uses_the_peak_at_the_worst_drop() {
    use analyzer::harness::stats::summarize;

    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 300, 301, 299, 300),
        candle(3, 360, 361, 359, 360),
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Long, 100),
            signal(1, Direction::Short, 300),
            signal(2, Direction::Long, 360),
        ],
        1000,
        0,
    );
    let summary = summarize(Decimal::from(1000), &outcome, d(360), 3, 4);

    assert_eq!(summary.trades, 2);
    assert_eq!(summary.wins, 1);
    assert_eq!(summary.losses, 1);
    assert_eq!(summary.max_drawdown, d(100));
    // 100 drawdown from the 2000 peak is 5%, not 10% of the initial 1000.
    assert_eq!(summary.max_drawdown_pct, d(5));
}

#[test]
fn win_rate_counts_breakeven_trades_in_the_denominator() {
    use analyzer::harness::stats::summarize;

    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 120, 121, 119, 120),
        candle(3, 120, 121, 119, 120),
        candle(4, 90, 91, 89, 90),
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Long, 100),
            signal(1, Direction::Short, 120),
            signal(2, Direction::Long, 120),
            signal(3, Direction::Short, 90),
        ],
        1000,
        0,
    );
    assert_eq!(outcome.trades.len(), 3);
    assert_eq!(outcome.trades[0].pnl, d(100));
    assert_eq!(outcome.trades[1].pnl, Decimal::ZERO);
    assert_eq!(outcome.trades[2].pnl, d(-125));

    let summary = summarize(Decimal::from(1000), &outcome, d(90), 4, 5);
    let expected_win_rate = Decimal::from(1) / Decimal::from(3) * Decimal::from(100);
    assert_eq!(summary.wins, 1);
    assert_eq!(summary.losses, 1);
    assert_eq!(summary.breakeven, 1);
    assert_eq!(summary.win_rate, expected_win_rate);
}
