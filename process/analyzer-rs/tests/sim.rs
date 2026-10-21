#![cfg(feature = "harness")]

use analyzer_rs::harness::sim::{simulate, SimSignal};
use analyzer_rs::strategy::types::{Candle, Direction, DivergenceKind, Signal};
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

fn signal(index: usize, side: Direction, entry: i64, sl: i64, tp: i64) -> SimSignal {
    SimSignal {
        candle_index: index,
        signal_id: format!("sig-{index}-{entry}"),
        signal: Signal {
            ticker: "TEST-USDT".into(),
            action: side,
            confidence: Decimal::new(6, 1),
            entry_price: d(entry),
            reason: "regular_bullish_divergence".into(),
            sl_price: d(sl),
            tp_price: d(tp),
            divergence_index: 0,
            divergence_kind: DivergenceKind::RegularBullish,
            divergence_open_time_ms: index as i64 * 86_400_000,
            divergence_age_candles: 0,
            signal_candle_open_time_ms: index as i64 * 86_400_000,
        },
    }
}

#[test]
fn long_entry_fills_at_next_open_and_exits_at_tp() {
    let candles = vec![candle(0, 100, 101, 99, 100), candle(1, 101, 106, 100, 105)];
    let outcome = simulate(
        &candles,
        &[signal(0, Direction::Long, 100, 98, 104)],
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
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 94, 95),
        candle(2, 95, 100, 94, 99),
        candle(3, 100, 108, 99, 107),
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Short, 100, 105, 95),
            signal(2, Direction::Short, 99, 105, 94),
        ],
        1000,
        0,
    );
    assert_eq!(outcome.trades.len(), 2);
    assert!(outcome.trades[0].pnl > Decimal::ZERO);
    assert!(outcome.trades[1].pnl < Decimal::ZERO);
}

#[test]
fn sl_first_when_both_levels_are_in_range() {
    let candles = vec![candle(0, 100, 101, 99, 100), candle(1, 100, 106, 97, 105)];
    let outcome = simulate(
        &candles,
        &[signal(0, Direction::Long, 100, 98, 104)],
        1000,
        0,
    );
    assert_eq!(outcome.trades[0].reason, "SL");
    assert_eq!(outcome.trades[0].exit, d(98));
}

#[test]
fn invalid_geometry_is_rejected() {
    let candles = vec![candle(0, 100, 101, 99, 100), candle(1, 101, 102, 100, 101)];
    let outcome = simulate(
        &candles,
        &[signal(0, Direction::Long, 100, 102, 104)],
        1000,
        0,
    );
    assert!(outcome.trades.is_empty());
    assert_eq!(outcome.invalid_levels, 1);
}

#[test]
fn funds_limit_concurrent_positions_and_release_on_next_candle() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 100, 101, 99, 100),
        candle(3, 100, 101, 99, 100),
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Long, 100, 90, 120),
            signal(1, Direction::Long, 100, 90, 120),
            signal(2, Direction::Long, 100, 90, 120),
        ],
        1000,
        0,
    );
    assert_eq!(outcome.trades.len() + outcome.open_positions.len(), 2);
    assert_eq!(outcome.skipped_funds, 1);
}

#[test]
fn summary_stats_use_realized_pnl() {
    use analyzer_rs::harness::stats::summarize;
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 101, 106, 100, 105),
        candle(2, 105, 106, 94, 95),
        candle(3, 95, 100, 94, 99),
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Long, 100, 98, 104),
            signal(1, Direction::Short, 105, 108, 95),
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
fn gap_beyond_the_stop_fills_at_the_open_for_long_and_short() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 95, 96, 90, 92), // long opens below SL 98
    ];
    let long = simulate(
        &candles,
        &[signal(0, Direction::Long, 100, 98, 110)],
        1000,
        0,
    );
    assert_eq!(long.trades[0].reason, "gap_sl");
    assert_eq!(long.trades[0].fill, d(95));
    assert_eq!(long.trades[0].exit, d(95));

    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 106, 107, 105, 106), // short opens above SL 105
    ];
    let short = simulate(
        &candles,
        &[signal(0, Direction::Short, 100, 105, 90)],
        1000,
        0,
    );
    assert_eq!(short.trades[0].reason, "gap_sl");
    assert_eq!(short.trades[0].fill, d(106));
    assert_eq!(short.trades[0].exit, d(106));
}

#[test]
fn gap_beyond_the_target_fills_at_the_open_for_long_and_short() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 106, 107, 105, 106), // long opens above TP 104
    ];
    let long = simulate(
        &candles,
        &[signal(0, Direction::Long, 100, 98, 104)],
        1000,
        0,
    );
    assert_eq!(long.trades[0].reason, "gap_tp");
    assert_eq!(long.trades[0].fill, d(106));
    assert_eq!(long.trades[0].exit, d(106));

    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 90, 91, 89, 90), // short opens below TP 95
    ];
    let short = simulate(
        &candles,
        &[signal(0, Direction::Short, 100, 105, 95)],
        1000,
        0,
    );
    assert_eq!(short.trades[0].reason, "gap_tp");
    assert_eq!(short.trades[0].fill, d(90));
    assert_eq!(short.trades[0].exit, d(90));
}

#[test]
fn released_funds_fill_the_next_signal_instead_of_skipping() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 100, 106, 99, 105),
        candle(3, 100, 101, 99, 100),
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Long, 100, 90, 105),
            signal(1, Direction::Long, 100, 90, 120),
            signal(2, Direction::Long, 100, 90, 120),
        ],
        1000,
        0,
    );
    // The signal at candle 2 fills on candle 3 using funds released when the first
    // position exits at TP on candle 2.
    assert_eq!(outcome.trades.len(), 1);
    assert_eq!(outcome.trades[0].entry_index, 1);
    assert_eq!(outcome.open_positions.len(), 2);
    assert!(outcome.open_positions.iter().any(|p| p.entry_index == 3));
    assert_eq!(outcome.skipped_funds, 0);
}

#[test]
fn summary_reports_exact_risk_and_exposure_metrics() {
    use analyzer_rs::harness::stats::summarize;
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
            signal(0, Direction::Long, 100, 80, 110),
            signal(2, Direction::Long, 100, 80, 110),
        ],
        1000,
        0,
    );
    let summary = summarize(Decimal::from(1000), &outcome, d(110), 4, 5);
    assert_eq!(summary.trades, 2);
    assert_eq!(summary.profit_factor, Some(Decimal::new(5, 1)));
    assert!(!summary.profit_factor_is_infinite);
    assert_eq!(summary.avg_win, d(50));
    assert_eq!(summary.avg_loss, d(-100));
    assert_eq!(summary.max_drawdown, d(100));
    assert_eq!(summary.max_drawdown_pct, d(10));
    assert_eq!(summary.exposure_candles, 2);
}

#[test]
fn exposure_counts_open_positions_from_last_index() {
    use analyzer_rs::harness::stats::summarize;
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 100, 111, 99, 110), // first trade hits TP 110
        candle(3, 100, 101, 99, 100),
    ];
    // Signal at 0 enters at 1 and exits at 2 (duration 1); the signal at 1 opens at 2
    // and is still open at last_index 3 (duration 1).
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Long, 100, 90, 110),
            signal(1, Direction::Long, 100, 90, 120),
        ],
        1000,
        0,
    );
    let summary = summarize(Decimal::from(1000), &outcome, Decimal::from(100), 3, 4);
    // Closed 1->2 = 1 candle, open 2->3 = 1.
    assert_eq!(summary.exposure_candles, 2);
}

#[test]
fn max_drawdown_percentage_uses_the_peak_at_the_worst_drop() {
    use analyzer_rs::harness::stats::summarize;
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 151, 99, 150), // win +250, peak 1250
        candle(2, 100, 101, 89, 90),  // loss -50, drop 50 from 1250
        candle(3, 100, 161, 99, 160), // win +300, later peak 1500
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Long, 100, 90, 150),
            signal(1, Direction::Long, 100, 90, 160),
            signal(2, Direction::Long, 100, 90, 160),
        ],
        1000,
        0,
    );
    let summary = summarize(Decimal::from(1000), &outcome, Decimal::from(160), 3, 4);
    // The worst drop is 50 from the first peak (1250), so the percentage is 4%; using
    // the later peak (1500) would incorrectly report 3.33%.
    assert_eq!(summary.trades, 3);
    assert_eq!(summary.wins, 2);
    assert_eq!(summary.losses, 1);
    assert_eq!(summary.max_drawdown, d(50));
    assert_eq!(summary.max_drawdown_pct, d(4));
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
        &[signal(0, Direction::Short, 100, 200, 50)],
        1000,
        0,
    );
    assert_eq!(outcome.trades.len(), 1);
    assert_eq!(outcome.trades[0].reason, "gap_sl");
    assert_eq!(outcome.trades[0].e, d(500));
    // A short filled at 100 and exited at 250 would lose 750; the engine caps it at E.
    assert_eq!(outcome.trades[0].pnl, d(-500));
}

#[test]
fn signal_on_the_final_candle_is_unfilled_at_end() {
    let candles = vec![candle(0, 100, 101, 99, 100), candle(1, 100, 101, 99, 100)];
    let outcome = simulate(
        &candles,
        &[signal(1, Direction::Long, 100, 98, 104)],
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
        &[signal(0, Direction::Long, 100, 95, 110)],
        1000,
        0,
    );
    assert_eq!(normal.trades[0].risk_reward, Some(d(2)));

    let candles = vec![candle(0, 100, 101, 99, 100), candle(1, 106, 107, 105, 106)];
    let gapped = simulate(
        &candles,
        &[signal(0, Direction::Long, 100, 98, 104)],
        1000,
        0,
    );
    assert_eq!(gapped.trades[0].reason, "gap_tp");
    assert_eq!(gapped.trades[0].risk_reward, None);
}

#[test]
fn win_rate_counts_breakeven_trades_in_the_denominator() {
    use analyzer_rs::harness::stats::summarize;
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100), // fill win
        candle(2, 100, 111, 99, 110), // win TP 110
        candle(3, 100, 101, 99, 100), // fill loss
        candle(4, 100, 101, 94, 95),  // loss SL 95
        candle(5, 100, 101, 99, 100), // fill breakeven and gap TP at 100
    ];
    let outcome = simulate(
        &candles,
        &[
            signal(0, Direction::Long, 100, 95, 110),
            signal(2, Direction::Long, 100, 95, 120),
            signal(4, Direction::Long, 99, 98, 100),
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
fn trades_and_positions_carry_signal_id() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 101, 106, 100, 105),
        candle(2, 105, 106, 104, 105),
    ];
    let mut long = signal(0, Direction::Long, 100, 98, 104);
    long.signal_id = "sig-a".into();
    let mut open = signal(1, Direction::Long, 105, 95, 120);
    open.signal_id = "sig-b".into();
    let outcome = simulate(&candles, &[long, open], 1000, 0);
    assert_eq!(outcome.trades[0].signal_id, "sig-a");
    assert_eq!(outcome.open_positions[0].signal_id, "sig-b");
}

#[test]
fn account_series_is_consistent_at_every_step() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 101, 106, 100, 105),
        candle(2, 105, 106, 104, 105),
    ];
    let mut first = signal(0, Direction::Long, 100, 98, 104);
    first.signal_id = "sig-a".into();
    let mut second = signal(1, Direction::Long, 105, 95, 120);
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
fn exit_classification_and_avg_risk_reward() {
    use analyzer_rs::harness::stats::summarize;
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 101, 106, 100, 105), // TP
        candle(2, 105, 106, 94, 95),   // next entry opens here
        candle(3, 95, 100, 90, 92),    // SL
    ];
    let mut a = signal(0, Direction::Long, 100, 98, 104);
    a.signal_id = "sig-a".into();
    let mut b = signal(2, Direction::Long, 95, 90, 120);
    b.signal_id = "sig-b".into();
    let outcome = simulate(&candles, &[a, b], 1000, 0);
    let summary = summarize(Decimal::from(1000), &outcome, Decimal::from(92), 3, 4);
    assert_eq!(summary.tp_hits, 1);
    assert_eq!(summary.sl_hits, 1);
    assert_eq!(summary.other_exits, 0);
    assert!(summary.avg_risk_reward.is_some());
}
