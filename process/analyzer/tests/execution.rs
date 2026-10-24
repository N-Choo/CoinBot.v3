#![cfg(feature = "harness")]

use analyzer::exchange::timeframes::Timeframe;
use analyzer::harness::dataset::load_csv;
use analyzer::harness::replay::ReplaySignal;
use analyzer::harness::sim::{simulate, SimSignal};
use analyzer::harness::sweep::execution::{
    evaluate_execution_config, simulate_execution, EntryMode, ExecModel, ExecOutcome,
};
use analyzer::harness::sweep::partition::chronological_splits;
use analyzer::harness::sweep::sampling;
use analyzer::strategy::types::{Candle, Direction, Signal};
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

fn replay_signal(index: usize, side: Direction, entry: i64) -> ReplaySignal {
    ReplaySignal {
        candle_index: index,
        signal_id: format!("sig-{index}-{entry}"),
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
    let signals = vec![replay_signal(0, Direction::Long, 100)];
    simulate_execution(&candles, &signals, 1000, 0, &limit_model(1, 2, 0))
}

fn run_short(candle_one: Candle) -> ExecOutcome {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle_one,
        candle(2, 99, 101, 98, 100),
    ];
    let signals = vec![replay_signal(0, Direction::Short, 100)];
    simulate_execution(&candles, &signals, 1000, 0, &limit_model(1, 2, 0))
}

#[test]
fn market_model_matches_existing_simulator_on_reversal() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 101, 106, 100, 105),
        candle(2, 105, 106, 94, 95),
        candle(3, 95, 100, 90, 92),
    ];
    let replay_signals = vec![
        replay_signal(0, Direction::Long, 100),
        replay_signal(2, Direction::Short, 95),
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

    assert_eq!(exec.trades.len(), 2);
    assert_eq!(legacy.trades.len(), 1);
    let exec_reversal = &exec.trades[0];
    let legacy_reversal = &legacy.trades[0];
    assert_eq!(exec_reversal.fill, legacy_reversal.fill);
    assert_eq!(exec_reversal.exit, legacy_reversal.exit);
    assert_eq!(exec_reversal.entry_index, legacy_reversal.entry_index);
    assert_eq!(exec_reversal.exit_index, legacy_reversal.exit_index);
    assert_eq!(exec_reversal.gross_pnl, legacy_reversal.pnl);
    assert_eq!(exec_reversal.net_pnl, legacy_reversal.pnl);
    assert_eq!(exec_reversal.reason, "reversal");
    assert_eq!(exec.trades[1].reason, "eod");
    assert_eq!(exec.expired, 0);
    assert_eq!(
        exec.filled + exec.expired + exec.skipped_funds + exec.same_side,
        exec.signals_total
    );
}

#[test]
fn same_side_signal_is_ignored_without_adding_a_position() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 100, 101, 99, 100),
        candle(3, 100, 101, 99, 100),
    ];
    let signals = vec![
        replay_signal(0, Direction::Long, 100),
        replay_signal(2, Direction::Long, 100),
    ];
    let model = ExecModel {
        label: "market_zero".into(),
        entry: EntryMode::Market,
        maker_bps: Decimal::ZERO,
        taker_bps: Decimal::ZERO,
        slip_bps: Decimal::ZERO,
    };

    let outcome = simulate_execution(&candles, &signals, 1000, 0, &model);

    assert_eq!(outcome.signals_total, 2);
    assert_eq!(outcome.filled, 1);
    assert_eq!(outcome.same_side, 1);
    assert_eq!(outcome.expired, 0);
    assert_eq!(outcome.trades.len(), 1);
    assert_eq!(outcome.trades[0].reason, "eod");
}

#[test]
fn opposite_signal_reverses_the_open_position_at_the_fill_price() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 100, 101, 99, 100),
        candle(3, 110, 111, 109, 110),
    ];
    let signals = vec![
        replay_signal(0, Direction::Long, 100),
        replay_signal(2, Direction::Short, 100),
    ];
    let model = ExecModel {
        label: "market_zero".into(),
        entry: EntryMode::Market,
        maker_bps: Decimal::ZERO,
        taker_bps: Decimal::ZERO,
        slip_bps: Decimal::ZERO,
    };

    let outcome = simulate_execution(&candles, &signals, 1000, 0, &model);

    assert_eq!(outcome.signals_total, 2);
    assert_eq!(outcome.filled, 2);
    assert_eq!(outcome.same_side, 0);
    assert_eq!(outcome.expired, 0);
    assert_eq!(outcome.trades.len(), 2);
    let reversal = &outcome.trades[0];
    assert_eq!(reversal.side, Direction::Long);
    assert_eq!(reversal.reason, "reversal");
    assert_eq!(reversal.entry_index, 1);
    assert_eq!(reversal.exit_index, 3);
    assert_eq!(reversal.exit, d(110));
    let eod = &outcome.trades[1];
    assert_eq!(eod.side, Direction::Short);
    assert_eq!(eod.reason, "eod");
    assert_eq!(eod.fill, d(110));
}

#[test]
fn limit_reversal_closes_the_long_at_the_short_limit_fill_price() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        // Long limit fills at the open 99, below its limit of 100.
        candle(1, 99, 100, 98, 99),
        candle(2, 99, 100, 98, 99),
        // Short limit fills at 100 while the candle opens at 98.
        candle(3, 98, 101, 97, 100),
    ];
    let signals = vec![
        replay_signal(0, Direction::Long, 100),
        replay_signal(2, Direction::Short, 100),
    ];
    let outcome = simulate_execution(&candles, &signals, 1000, 0, &limit_model(1, 2, 0));

    assert_eq!(outcome.signals_total, 2);
    assert_eq!(outcome.filled, 2);
    assert_eq!(outcome.same_side, 0);
    assert_eq!(outcome.expired, 0);

    let reversal = &outcome.trades[0];
    let exit_notional = d(500) * (d(100) / d(99));
    let gross_pnl = exit_notional - d(500);
    let entry_cost = d(500) * d(2) / d(10_000);
    let exit_cost = exit_notional * d(15) / d(10_000);
    assert_eq!(reversal.side, Direction::Long);
    assert_eq!(reversal.reason, "reversal");
    assert_eq!(reversal.fill, d(99));
    assert_eq!(reversal.exit, d(100));
    assert_eq!(reversal.entry_index, 1);
    assert_eq!(reversal.exit_index, 3);
    assert_eq!(reversal.gross_pnl, gross_pnl);
    assert_eq!(reversal.net_pnl, gross_pnl - entry_cost - exit_cost);
    assert_eq!(outcome.available_fund, d(1000) + gross_pnl);
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
    // The low 99 touches the buffered threshold.
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
fn limit_entry_holds_until_reversal_or_eod() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        // Fills at 100 and does not exit on its entry candle.
        candle(1, 100, 106, 90, 105),
        // The opposite signal closes it at that candle's limit fill.
        candle(2, 100, 110, 90, 91),
    ];
    let signals = vec![
        replay_signal(0, Direction::Long, 100),
        replay_signal(1, Direction::Short, 105),
    ];
    let outcome = simulate_execution(&candles, &signals, 1000, 0, &limit_model(1, 2, 0));

    assert_eq!(outcome.trades.len(), 2);
    let reversal = &outcome.trades[0];
    assert_eq!(reversal.entry_index, 1);
    assert_eq!(reversal.exit_index, 2);
    assert_eq!(reversal.reason, "reversal");
    assert_eq!(reversal.exit, d(105));
}

#[test]
fn market_cost_is_15_bps_per_side() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 100, 111, 99, 110),
    ];
    let signals = vec![replay_signal(0, Direction::Long, 100)];
    let model = ExecModel {
        label: "market".into(),
        entry: EntryMode::Market,
        maker_bps: d(2),
        taker_bps: d(10),
        slip_bps: d(5),
    };
    let outcome = simulate_execution(&candles, &signals, 1000, 0, &model);

    let trade = &outcome.trades[0];
    assert_eq!(trade.reason, "eod");
    assert_eq!(trade.fill, d(100));
    assert_eq!(trade.exit, d(110));
    assert_eq!(trade.gross_pnl, d(50));
    // E = 500; entry cost 15 bps of 500; exit cost 15 bps of 550.
    assert_eq!(trade.entry_cost, Decimal::new(75, 2));
    assert_eq!(trade.exit_cost, Decimal::new(825, 3));
    assert_eq!(trade.net_pnl, Decimal::new(48425, 3));
}

#[test]
fn limit_entry_pays_maker_while_every_exit_pays_taker() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 100, 111, 99, 110),
    ];
    let signals = vec![replay_signal(0, Direction::Long, 100)];
    let outcome = simulate_execution(&candles, &signals, 1000, 0, &priced_model(2, 10, 5));
    let trade = &outcome.trades[0];
    assert_eq!(trade.reason, "eod");
    assert_eq!(trade.gross_pnl, d(50));
    // Maker entry: 2 bps of 500. Taker exit: 15 bps of 550.
    assert_eq!(trade.entry_cost, Decimal::new(1, 1));
    assert_eq!(trade.exit_cost, Decimal::new(825, 3));
    assert_eq!(trade.net_pnl, Decimal::new(49075, 3));
}

#[test]
fn eod_closes_open_positions_with_taker_cost() {
    let candles = vec![
        candle(0, 100, 101, 99, 100),
        candle(1, 100, 101, 99, 100),
        candle(2, 100, 101, 99, 105),
    ];
    let signals = vec![replay_signal(0, Direction::Long, 100)];
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
        replay_signal(0, Direction::Long, 100),
        // Same-candle twin: the position is open by the time this order is worked.
        replay_signal(0, Direction::Long, 100),
        // Same-side signals while the long is still open: no-ops.
        replay_signal(1, Direction::Long, 100),
        replay_signal(2, Direction::Long, 100),
        replay_signal(2, Direction::Long, 90),
    ];
    let outcome = simulate_execution(&candles, &signals, 1000, 0, &limit_model(1, 2, 0));

    assert_eq!(outcome.signals_total, 5);
    assert_eq!(outcome.filled, 1);
    assert_eq!(outcome.trades.len(), 1);
    assert_eq!(outcome.expired, 0);
    assert_eq!(outcome.skipped_funds, 0);
    assert_eq!(outcome.same_side, 4);
    assert_eq!(
        outcome.filled + outcome.expired + outcome.skipped_funds + outcome.same_side,
        outcome.signals_total
    );
    let capital_fill_rate = Decimal::from(outcome.filled) / Decimal::from(outcome.signals_total);
    assert_eq!(capital_fill_rate, Decimal::new(2, 1));
    let execution_fill_rate = Decimal::from(outcome.filled)
        / Decimal::from(outcome.signals_total - outcome.skipped_funds - outcome.same_side);
    assert_eq!(execution_fill_rate, Decimal::ONE);
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

    let result =
        evaluate_execution_config(&dataset, &baseline.to_config(), &splits, 100, 1000, &models)
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
                    + partition.same_side,
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
