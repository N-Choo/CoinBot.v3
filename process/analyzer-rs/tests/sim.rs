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
    let summary = summarize(Decimal::from(1000), &outcome, Decimal::from(99), 4);
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
