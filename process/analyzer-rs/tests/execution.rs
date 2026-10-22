#![cfg(feature = "harness")]

use analyzer_rs::exchange::timeframes::Timeframe;
use analyzer_rs::harness::dataset::load_csv;
use analyzer_rs::harness::replay::ReplaySignal;
use analyzer_rs::harness::sim::{simulate, SimSignal};
use analyzer_rs::harness::sweep::execution::{
    evaluate_execution_config, simulate_execution, EntryMode, ExecModel, ExecOutcome,
};
use analyzer_rs::harness::sweep::partition::chronological_splits;
use analyzer_rs::harness::sweep::sampling;
use analyzer_rs::strategy::types::{Candle, Direction, DivergenceKind, Signal};
use rust_decimal::Decimal;
use std::path::Path;

fn d(value: i64) -> Decimal {
    Decimal::from(value)
}

fn candle(i: usize, open: i64, high: i64, low: i64, close: i64) -> Candle {
    Candle {
        open_time_ms: i as i64 * 14_400_000,
        open: d(open),
        high: d(high),
        low: d(low),
        close: d(close),
        volume: d(1),
    }
}

fn replay_signal(index: usize, side: Direction, entry: i64, sl: i64, tp: i64) -> ReplaySignal {
    ReplaySignal {
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
            divergence_open_time_ms: index as i64 * 14_400_000,
            divergence_age_candles: 0,
            signal_candle_open_time_ms: index as i64 * 14_400_000,
        },
    }
}

fn sim_signal(replay: &ReplaySignal) -> SimSignal {
    SimSignal {
        candle_index: replay.candle_index,
        signal: replay.signal.clone(),
        signal_id: replay.signal_id.clone(),
    }
}

fn limit_model(valid_candles: usize, buffer_bps: i64, offset_bps: i64) -> ExecModel {
    ExecModel {
        label: format!("limit_{valid_candles}c_b{buffer_bps}_o{offset_bps}"),
        entry: EntryMode::Limit {
            valid_candles,
            buffer_bps: d(buffer_bps),
            offset_bps: d(offset_bps),
        },
        maker_bps: d(2),
        taker_bps: d(10),
        slip_bps: d(5),
    }
}

fn priced_model(maker_bps: i64, taker_bps: i64, slip_bps: i64) -> ExecModel {
    ExecModel {
        label: "priced".into(),
        entry: EntryMode::Limit {
            valid_candles: 1,
            buffer_bps: d(2),
            offset_bps: Decimal::ZERO,
        },
        maker_bps: d(maker_bps),
        taker_bps: d(taker_bps),
        slip_bps: d(slip_bps),
    }
}

fn run_long(candle_one: Candle) -> ExecOutcome {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle_one,
        candle(2, 99, 101, 98, 100),
    ];
    let signals = vec![replay_signal(0, Direction::Long, 100, 90, 120)];
    simulate_execution(&candles, &signals, 1000, 0, &limit_model(1, 2, 0))
}

fn run_short(candle_one: Candle) -> ExecOutcome {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle_one,
        candle(2, 99, 101, 98, 100),
    ];
    let signals = vec![replay_signal(0, Direction::Short, 100, 110, 80)];
    simulate_execution(&candles, &signals, 1000, 0, &limit_model(1, 2, 0))
}

#[test]
fn market_model_matches_existing_simulator_on_short_gap() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        // Gaps through the short stop at the next open.
        candle(2, 112, 115, 111, 114),
        candle(3, 114, 116, 110, 111),
    ];
    let replay_signals = vec![replay_signal(0, Direction::Short, 100, 105, 90)];
    let sim_signals: Vec<SimSignal> = replay_signals.iter().map(sim_signal).collect();
    let model = ExecModel {
        label: "market_zero".into(),
        entry: EntryMode::Market,
        maker_bps: Decimal::ZERO,
        taker_bps: Decimal::ZERO,
        slip_bps: Decimal::ZERO,
    };

    let exec = simulate_execution(&candles, &replay_signals, 1000, 0, &model);
    let legacy = simulate(&candles, &sim_signals, 1000, 0);

    assert_eq!(exec.trades.len(), legacy.trades.len());
    assert_eq!(exec.trades.len(), 1);
    for (exec_trade, legacy_trade) in exec.trades.iter().zip(legacy.trades.iter()) {
        assert_eq!(exec_trade.fill, legacy_trade.fill);
        assert_eq!(exec_trade.exit, legacy_trade.exit);
        assert_eq!(exec_trade.entry_index, legacy_trade.entry_index);
        assert_eq!(exec_trade.exit_index, legacy_trade.exit_index);
        assert_eq!(exec_trade.gross_pnl, legacy_trade.pnl);
        assert_eq!(exec_trade.net_pnl, legacy_trade.pnl);
    }
    assert_eq!(exec.trades[0].reason, "gap_sl");
    assert_eq!(exec.expired, 0);
}

#[test]
fn market_model_matches_existing_simulator_gross() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 101, 106, 100, 105),
        candle(2, 105, 106, 94, 95),
        candle(3, 95, 100, 90, 92),
    ];
    let replay_signals = vec![
        replay_signal(0, Direction::Long, 100, 98, 104),
        replay_signal(2, Direction::Long, 95, 90, 120),
    ];
    let sim_signals: Vec<SimSignal> = replay_signals.iter().map(sim_signal).collect();
    let model = ExecModel {
        label: "market_zero".into(),
        entry: EntryMode::Market,
        maker_bps: Decimal::ZERO,
        taker_bps: Decimal::ZERO,
        slip_bps: Decimal::ZERO,
    };

    let exec = simulate_execution(&candles, &replay_signals, 1000, 0, &model);
    let legacy = simulate(&candles, &sim_signals, 1000, 0);

    assert_eq!(exec.trades.len(), legacy.trades.len());
    assert!(legacy.open_positions.is_empty());
    for (exec_trade, legacy_trade) in exec.trades.iter().zip(legacy.trades.iter()) {
        assert_eq!(exec_trade.fill, legacy_trade.fill);
        assert_eq!(exec_trade.exit, legacy_trade.exit);
        assert_eq!(exec_trade.entry_index, legacy_trade.entry_index);
        assert_eq!(exec_trade.exit_index, legacy_trade.exit_index);
        assert_eq!(exec_trade.gross_pnl, legacy_trade.pnl);
        assert_eq!(exec_trade.net_pnl, legacy_trade.pnl);
    }
    assert_eq!(exec.expired, 0);
}

#[test]
fn long_limit_fills_on_open_through_and_on_touch() {
    // Opens through the buffered threshold: fill at the open.
    let open_through = run_long(candle(1, 99, 100, 98, 99));
    assert_eq!(open_through.filled, 1);
    assert_eq!(open_through.trades.len(), 1);
    assert_eq!(open_through.trades[0].fill, d(99));

    // Opens above the threshold, then trades through it: fill at the limit.
    let touch = run_long(candle(1, 100, 100, 99, 100));
    assert_eq!(touch.filled, 1);
    assert_eq!(touch.trades[0].fill, d(100));
}

#[test]
fn long_limit_expires_when_not_touched() {
    let outcome = run_long(candle(1, 100, 101, 99, 100));
    // The low 99 touches the buffered threshold; use a candle that never reaches it.
    assert_eq!(outcome.filled, 1);

    let no_touch = run_long(candle(1, 100, 101, 100, 100));
    assert!(no_touch.trades.is_empty());
    assert_eq!(no_touch.filled, 0);
    assert_eq!(no_touch.expired, 1);
}

#[test]
fn short_limit_fills_on_open_through_and_on_touch() {
    // Opens through the buffered threshold: fill at the open.
    let open_through = run_short(candle(1, 101, 102, 101, 101));
    assert_eq!(open_through.filled, 1);
    assert_eq!(open_through.trades[0].fill, d(101));

    // Opens below the threshold, then trades through it: fill at the limit.
    let touch = run_short(candle(1, 99, 101, 99, 99));
    assert_eq!(touch.filled, 1);
    assert_eq!(touch.trades[0].fill, d(100));

    let no_touch = run_short(candle(1, 98, 99, 98, 98));
    assert!(no_touch.trades.is_empty());
    assert_eq!(no_touch.expired, 1);
}

#[test]
fn buffer_requires_trade_through() {
    // A long whose low only reaches the limit (100) does not fill: the 2 bps buffer
    // requires the price to trade through 99.98.
    let long = run_long(candle(1, 100, 101, 100, 100));
    assert!(long.trades.is_empty(), "long must not fill at the limit");
    assert_eq!(long.expired, 1);

    // A short whose high only reaches the limit (100) likewise does not fill.
    let short = run_short(candle(1, 99, 100, 98, 99));
    assert!(short.trades.is_empty(), "short must not fill at the limit");
    assert_eq!(short.expired, 1);
}

#[test]
fn limit_entry_suppresses_same_candle_target() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        // Fills at 100; the high would hit TP 105 on the same candle.
        candle(1, 100, 106, 99, 105),
        // Exits at SL on the following candle.
        candle(2, 100, 101, 90, 91),
    ];
    let signals = vec![replay_signal(0, Direction::Long, 100, 95, 105)];
    let outcome = simulate_execution(&candles, &signals, 1000, 0, &limit_model(1, 2, 0));

    assert_eq!(outcome.trades.len(), 1);
    let trade = &outcome.trades[0];
    assert_eq!(trade.entry_index, 1);
    assert_eq!(trade.exit_index, 2);
    assert_eq!(trade.reason, "SL");
    assert_eq!(trade.exit, d(95));
}

#[test]
fn limit_entry_allows_same_candle_stop_after_fill() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        // Fills at the limit 100, then trades down through SL 95 on the same candle.
        candle(1, 100, 101, 94, 96),
        candle(2, 96, 100, 90, 95),
    ];
    let signals = vec![replay_signal(0, Direction::Long, 100, 95, 120)];
    let outcome = simulate_execution(&candles, &signals, 1000, 0, &limit_model(1, 2, 0));

    assert_eq!(outcome.trades.len(), 1);
    let trade = &outcome.trades[0];
    assert_eq!(trade.entry_index, 1);
    assert_eq!(trade.exit_index, 1);
    assert_eq!(trade.reason, "SL");
    assert_eq!(trade.exit, d(95));
}

#[test]
fn limit_entry_allows_same_candle_stop_after_open_fill() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        // Fills at the open 99 (price improvement), then trades down through SL 95.
        candle(1, 99, 100, 94, 96),
        candle(2, 96, 100, 90, 95),
    ];
    let signals = vec![replay_signal(0, Direction::Long, 100, 95, 120)];
    let outcome = simulate_execution(&candles, &signals, 1000, 0, &limit_model(1, 2, 0));

    assert_eq!(outcome.trades.len(), 1);
    let trade = &outcome.trades[0];
    assert_eq!(trade.entry_index, 1);
    assert_eq!(trade.exit_index, 1);
    assert_eq!(trade.reason, "SL");
    assert_eq!(trade.exit, d(95));
}

#[test]
fn short_limit_entry_allows_same_candle_stop_after_fill() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        // Fills at the limit 100, then trades up through SL 105 on the same candle.
        candle(1, 100, 106, 99, 104),
        candle(2, 104, 110, 100, 108),
    ];
    let signals = vec![replay_signal(0, Direction::Short, 100, 105, 80)];
    let outcome = simulate_execution(&candles, &signals, 1000, 0, &limit_model(1, 2, 0));

    assert_eq!(outcome.trades.len(), 1);
    let trade = &outcome.trades[0];
    assert_eq!(trade.entry_index, 1);
    assert_eq!(trade.exit_index, 1);
    assert_eq!(trade.reason, "SL");
    assert_eq!(trade.exit, d(105));
}

#[test]
fn retest_limit_beyond_stop_is_rejected() {
    let long_candles = vec![
        candle(0, 1000, 1001, 999, 1000),
        candle(1, 997, 998, 996, 997),
        candle(2, 997, 998, 996, 997),
    ];
    // limit = 1000 * (1 - 0.0025) = 997.5, which is below the long stop at 998.
    let long_signals = vec![replay_signal(0, Direction::Long, 1000, 998, 1200)];
    let long = simulate_execution(
        &long_candles,
        &long_signals,
        1000,
        0,
        &limit_model(2, 2, 25),
    );
    assert!(long.trades.is_empty());
    assert_eq!(long.filled, 0);
    assert_eq!(long.invalid_levels, 1);

    let short_candles = vec![
        candle(0, 1000, 1001, 999, 1000),
        candle(1, 1003, 1004, 1002, 1003),
        candle(2, 1003, 1004, 1002, 1003),
    ];
    // limit = 1000 * (1 + 0.0025) = 1002.5, which is above the short stop at 1002.
    let short_signals = vec![replay_signal(0, Direction::Short, 1000, 1002, 800)];
    let short = simulate_execution(
        &short_candles,
        &short_signals,
        1000,
        0,
        &limit_model(2, 2, 25),
    );
    assert!(short.trades.is_empty());
    assert_eq!(short.filled, 0);
    assert_eq!(short.invalid_levels, 1);
}

#[test]
fn market_cost_is_15_bps_per_side() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 100, 111, 99, 110),
    ];
    let signals = vec![replay_signal(0, Direction::Long, 100, 90, 110)];
    let model = ExecModel {
        label: "market".into(),
        entry: EntryMode::Market,
        maker_bps: d(2),
        taker_bps: d(10),
        slip_bps: d(5),
    };
    let outcome = simulate_execution(&candles, &signals, 1000, 0, &model);

    let trade = &outcome.trades[0];
    assert_eq!(trade.fill, d(100));
    assert_eq!(trade.exit, d(110));
    assert_eq!(trade.gross_pnl, d(50));
    // E = 500; entry cost 15 bps of 500; exit cost 15 bps of 550.
    assert_eq!(trade.entry_cost, Decimal::new(75, 2));
    assert_eq!(trade.exit_cost, Decimal::new(825, 3));
    assert_eq!(trade.net_pnl, Decimal::new(48425, 3));
}

#[test]
fn limit_entry_and_tp_pay_maker_while_sl_pays_taker() {
    // Limit entry, TP exit: maker on both sides.
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 100, 111, 99, 110),
    ];
    let signals = vec![replay_signal(0, Direction::Long, 100, 90, 110)];
    let maker = simulate_execution(&candles, &signals, 1000, 0, &priced_model(2, 10, 5));
    let trade = &maker.trades[0];
    assert_eq!(trade.reason, "TP");
    assert_eq!(trade.gross_pnl, d(50));
    assert_eq!(trade.entry_cost, Decimal::new(1, 1));
    assert_eq!(trade.exit_cost, Decimal::new(11, 2));
    assert_eq!(trade.net_pnl, Decimal::new(4979, 2));

    // Limit entry, SL exit: maker entry, taker exit.
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 100, 101, 95, 96),
    ];
    let signals = vec![replay_signal(0, Direction::Long, 100, 95, 120)];
    let taker = simulate_execution(&candles, &signals, 1000, 0, &priced_model(2, 10, 5));
    let trade = &taker.trades[0];
    assert_eq!(trade.reason, "SL");
    assert_eq!(trade.gross_pnl, d(-25));
    assert_eq!(trade.entry_cost, Decimal::new(1, 1));
    // Exit notional 475, taker 15 bps.
    assert_eq!(trade.exit_cost, Decimal::new(7125, 4));
    assert_eq!(trade.net_pnl, Decimal::new(-258125, 4));
}

#[test]
fn eod_closes_open_positions_with_taker_cost() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 100, 101, 99, 105),
    ];
    let signals = vec![replay_signal(0, Direction::Long, 100, 90, 120)];
    let outcome = simulate_execution(&candles, &signals, 1000, 0, &limit_model(2, 2, 0));

    assert_eq!(outcome.trades.len(), 1);
    let trade = &outcome.trades[0];
    assert_eq!(trade.reason, "eod");
    assert_eq!(trade.entry_index, 1);
    assert_eq!(trade.exit_index, 2);
    assert_eq!(trade.exit, d(105));
    assert_eq!(trade.gross_pnl, d(25));
    assert_eq!(trade.entry_cost, Decimal::new(1, 1));
    assert_eq!(trade.exit_cost, Decimal::new(7875, 4));
    assert_eq!(trade.net_pnl, Decimal::new(241125, 4));
}

#[test]
fn fill_counts_add_up() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 100, 101, 99, 100),
        candle(3, 100, 101, 95, 100),
    ];
    let signals = vec![
        // Fills on candle 1.
        replay_signal(0, Direction::Long, 100, 90, 150),
        // Invalid geometry: SL above entry for a long.
        replay_signal(0, Direction::Long, 100, 102, 150),
        // Fills on candle 2, consuming the second half of the fund.
        replay_signal(1, Direction::Long, 100, 90, 150),
        // Meets its fill condition on candle 3 but no funds remain.
        replay_signal(2, Direction::Long, 100, 90, 150),
        // Never touches its limit: expires at the end of the data.
        replay_signal(2, Direction::Long, 90, 80, 130),
    ];
    let outcome = simulate_execution(&candles, &signals, 1000, 0, &limit_model(1, 2, 0));

    assert_eq!(outcome.signals_total, 5);
    assert_eq!(outcome.filled, 2);
    assert_eq!(outcome.trades.len(), 2);
    assert_eq!(outcome.expired, 1);
    assert_eq!(outcome.skipped_funds, 1);
    assert_eq!(outcome.invalid_levels, 1);
    assert_eq!(
        outcome.filled + outcome.expired + outcome.skipped_funds + outcome.invalid_levels,
        outcome.signals_total
    );
    let capital_fill_rate = Decimal::from(outcome.filled) / Decimal::from(outcome.signals_total);
    assert_eq!(capital_fill_rate, Decimal::new(4, 1));
    let execution_fill_rate = Decimal::from(outcome.filled)
        / Decimal::from(outcome.signals_total - outcome.skipped_funds - outcome.invalid_levels);
    assert_eq!(execution_fill_rate, Decimal::from(2) / Decimal::from(3));
}

#[test]
fn evaluate_execution_config_produces_validation_and_holdout_only() {
    let dataset = load_csv(
        "SYN",
        Path::new("tests/fixtures/synthetic_1day.csv"),
        Timeframe::D1,
        i64::MAX / 2,
        0,
        0,
        100,
        0,
    )
    .expect("fixture loads");
    let splits = chronological_splits(dataset.candles.len());
    let baseline = sampling::baseline();
    let models = ExecModel::focused();

    let result = evaluate_execution_config(
        &dataset,
        &baseline.to_config(),
        baseline.max_age,
        &splits,
        100,
        1000,
        &models,
    )
    .expect("evaluation runs");

    assert_eq!(result.len(), models.len());
    for model in &models {
        let partitions = result
            .get(&model.label)
            .unwrap_or_else(|| panic!("missing model {}", model.label));
        assert_eq!(partitions.len(), 2);
        assert!(partitions.contains_key("validation"));
        assert!(partitions.contains_key("holdout"));
        for partition in partitions.values() {
            assert_eq!(
                partition.filled
                    + partition.expired
                    + partition.skipped_funds
                    + partition.invalid_levels,
                partition.signals
            );
            assert!(partition.capital_fill_rate >= Decimal::ZERO);
            assert!(partition.capital_fill_rate <= Decimal::ONE);
            assert!(partition.execution_fill_rate >= Decimal::ZERO);
            assert!(partition.execution_fill_rate <= Decimal::ONE);
            assert!(partition.execution_fill_rate >= partition.capital_fill_rate);
            assert_eq!(
                partition.metrics.net_pnl,
                partition.metrics.closed_net_pnl + partition.metrics.open_net_pnl
            );
        }
    }
}
