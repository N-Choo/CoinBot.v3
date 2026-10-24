use crate::harness::dataset::Dataset;
use crate::harness::experiment::SliceOutcome;
use crate::harness::replay::build_replay;
use crate::harness::sim::{
    unrealized_pnl, AccountStep, ClosedTrade, OpenStep, Position, SimOutcome, SimSignal,
};
use crate::harness::sweep::costs::CostScenario;
use crate::harness::sweep::evaluate::metrics_from_outcome;
use crate::harness::sweep::partition::{slice_with_warmup, Slice};
use crate::strategy::config::Config;
use crate::strategy::indicators::ewm;
use crate::strategy::types::{Candle, Direction};
use rust_decimal::Decimal;
use serde::Serialize;

pub const REASON_TP: &str = "TP";
pub const REASON_GAP_TP: &str = "gap_tp";
pub const REASON_SL: &str = "SL";
pub const REASON_GAP_SL: &str = "gap_sl";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AtrVariant {
    pub label: String,
    pub enabled: bool,
    pub period: usize,
    pub sl_multiplier: Decimal,
    pub tp_multiplier: Decimal,
    pub pct_high: Decimal,
    pub high_adjust: Decimal,
    pub sl_max_pct: Decimal,
}

/// Average true range using the same exponential smoothing as the old
/// analyzer code: `ewm(alpha = 1 / period, adjust = false)` seeded at the
/// first true range.
pub fn atr_series(candles: &[Candle], period: usize) -> Vec<Option<Decimal>> {
    if period == 0 {
        return vec![None; candles.len()];
    }
    let mut true_range = vec![None; candles.len()];
    for (index, candle) in candles.iter().enumerate() {
        let high_low = candle.high - candle.low;
        if index == 0 {
            true_range[index] = Some(high_low);
        } else {
            let previous_close = candles[index - 1].close;
            let high_close = (candle.high - previous_close).abs();
            let low_close = (candle.low - previous_close).abs();
            true_range[index] = Some(high_low.max(high_close).max(low_close));
        }
    }
    ewm(&true_range, period).unwrap_or_else(|_| vec![None; candles.len()])
}

fn atr_levels(
    variant: &AtrVariant,
    side: Direction,
    close: Decimal,
    atr: Decimal,
) -> Option<(Decimal, Decimal)> {
    if !variant.enabled || atr <= Decimal::ZERO || close <= Decimal::ZERO {
        return None;
    }
    let atr_pct = atr / close * Decimal::from(100);
    let mut sl_multiplier = variant.sl_multiplier;
    let mut tp_multiplier = variant.tp_multiplier;
    if variant.high_adjust != Decimal::ONE && atr_pct > variant.pct_high {
        sl_multiplier *= variant.high_adjust;
        tp_multiplier *= variant.high_adjust;
    }
    let sl_distance = atr * sl_multiplier;
    let tp_distance = atr * tp_multiplier;
    let (mut sl, tp) = match side {
        Direction::Long => (close - sl_distance, close + tp_distance),
        Direction::Short => (close + sl_distance, close - tp_distance),
    };
    if variant.sl_max_pct > Decimal::ZERO {
        match side {
            Direction::Long => {
                let max_sl = close * (Decimal::ONE - variant.sl_max_pct);
                if sl < max_sl {
                    sl = max_sl;
                }
            }
            Direction::Short => {
                let max_sl = close * (Decimal::ONE + variant.sl_max_pct);
                if sl > max_sl {
                    sl = max_sl;
                }
            }
        }
    }
    if sl <= Decimal::ZERO || tp <= Decimal::ZERO {
        return None;
    }
    Some((sl, tp))
}

#[derive(Debug, Clone)]
struct AtrPosition {
    side: Direction,
    signal_id: String,
    signal_entry: Decimal,
    fill: Decimal,
    e: Decimal,
    sl: Option<Decimal>,
    tp: Option<Decimal>,
    entry_index: usize,
}

pub fn simulate_atr(
    candles: &[Candle],
    signals: &[SimSignal],
    variant: &AtrVariant,
    init_fund: i64,
    window: usize,
) -> SimOutcome {
    let init_fund = Decimal::from(init_fund);
    let budget = init_fund / Decimal::from(2);
    let atr = if variant.enabled {
        atr_series(candles, variant.period)
    } else {
        Vec::new()
    };
    let mut available = init_fund;
    let mut positions: Vec<AtrPosition> = Vec::new();
    let mut trades: Vec<ClosedTrade> = Vec::new();
    let mut account: Vec<AccountStep> = Vec::new();
    let mut skipped_funds = 0usize;

    for (index, candle) in candles.iter().enumerate() {
        if index < window {
            continue;
        }

        if let Some(sim_signal) = signals
            .iter()
            .find(|signal| signal.candle_index + 1 == index)
        {
            let signal = &sim_signal.signal;

            // A reversal closes the opposite side at the fill open. A
            // same-side signal is a no-op, matching the old simulator.
            let mut kept = Vec::new();
            for position in positions.drain(..) {
                if position.side == signal.action {
                    kept.push(position);
                } else {
                    close_position(
                        &mut trades,
                        &mut available,
                        position,
                        candle.open,
                        "reversal",
                        index,
                    );
                }
            }
            positions = kept;

            let already_target = positions
                .iter()
                .any(|position| position.side == signal.action);
            if !already_target {
                if available < budget {
                    skipped_funds += 1;
                } else {
                    let (sl, tp) = if variant.enabled {
                        let signal_atr = atr
                            .get(sim_signal.candle_index)
                            .copied()
                            .flatten()
                            .unwrap_or(Decimal::ZERO);
                        atr_levels(variant, signal.action, signal.entry_price, signal_atr)
                            .map(|(sl, tp)| (Some(sl), Some(tp)))
                            .unwrap_or((None, None))
                    } else {
                        (None, None)
                    };
                    available -= budget;
                    positions.push(AtrPosition {
                        side: signal.action,
                        signal_id: sim_signal.signal_id.clone(),
                        signal_entry: signal.entry_price,
                        fill: candle.open,
                        e: budget,
                        sl,
                        tp,
                        entry_index: index,
                    });
                }
            }
        }

        // Stops and targets are checked on the entry candle as well as later
        // candles. Gap exits use the open; otherwise SL is checked before TP.
        let mut still_open = Vec::new();
        for position in positions.drain(..) {
            match exit_for(&position, candle) {
                Some((reason, exit)) => {
                    close_position(&mut trades, &mut available, position, exit, &reason, index);
                }
                None => still_open.push(position),
            }
        }
        positions = still_open;

        let close = candle.close;
        let mut reserved = Decimal::ZERO;
        let mut unrealized = Decimal::ZERO;
        let mut open = Vec::with_capacity(positions.len());
        for position in &positions {
            reserved += position.e;
            let value = unrealized_pnl(position.side, position.fill, close, position.e);
            unrealized += value;
            open.push(OpenStep {
                signal_id: position.signal_id.clone(),
                unrealized: value,
            });
        }
        account.push(AccountStep {
            index,
            available_fund: available,
            reserved,
            unrealized,
            equity: available + reserved + unrealized,
            open,
        });
    }

    let unfilled_at_end = signals
        .iter()
        .filter(|signal| signal.candle_index + 1 >= candles.len())
        .count();

    SimOutcome {
        trades,
        open_positions: positions
            .into_iter()
            .map(|position| Position {
                side: position.side,
                signal_id: position.signal_id,
                signal_entry: position.signal_entry,
                fill: position.fill,
                e: position.e,
                entry_index: position.entry_index,
            })
            .collect(),
        account,
        available_fund: available,
        skipped_funds,
        unfilled_at_end,
    }
}

pub fn evaluate_atr_slice(
    dataset: &Dataset,
    cfg: &Config,
    variant: &AtrVariant,
    slice: Slice,
    window: usize,
    init_fund: i64,
    scenario: &CostScenario,
) -> Result<SliceOutcome, String> {
    if window != cfg.window {
        return Err(format!("window {window} != cfg.window {}", cfg.window));
    }
    let partition = slice_with_warmup(&dataset.candles, slice, window);
    let n_candles = partition.candles.len();
    if n_candles == 0 {
        return Err(format!(
            "{}: empty slice {}..{}",
            dataset.symbol, slice.start, slice.end
        ));
    }
    let eval_start = partition.partition_start;
    let last_index = n_candles - 1;
    let replayed_candles = n_candles.saturating_sub(window);
    let slice_dataset = Dataset {
        symbol: dataset.symbol.clone(),
        candles: partition.candles,
        provenance: dataset.provenance.clone(),
    };
    let replay = build_replay(&slice_dataset, cfg)
        .map_err(|error| format!("{}: {error}", dataset.symbol))?;
    let sim_signals: Vec<SimSignal> = replay
        .signals
        .into_iter()
        .filter(|signal| signal.candle_index >= eval_start && signal.candle_index + 1 < n_candles)
        .map(|signal| SimSignal {
            candle_index: signal.candle_index,
            signal: signal.signal,
            signal_id: signal.signal_id,
        })
        .collect();
    let outcome = simulate_atr(
        &slice_dataset.candles,
        &sim_signals,
        variant,
        init_fund,
        window,
    );
    let last_close = slice_dataset
        .candles
        .last()
        .expect("slice always contains candles")
        .close;
    let metrics = metrics_from_outcome(
        &outcome,
        last_close,
        last_index,
        replayed_candles,
        Decimal::from(init_fund),
        scenario,
    );
    Ok(SliceOutcome {
        metrics,
        outcome,
        candles: slice_dataset.candles,
    })
}

fn close_position(
    trades: &mut Vec<ClosedTrade>,
    available: &mut Decimal,
    position: AtrPosition,
    exit: Decimal,
    reason: &str,
    index: usize,
) {
    let proceeds = position.e * (exit / position.fill);
    let mut pnl = match position.side {
        Direction::Long => proceeds - position.e,
        Direction::Short => position.e - proceeds,
    };
    if pnl < -position.e {
        pnl = -position.e;
    }
    *available += position.e + pnl;
    trades.push(ClosedTrade {
        side: position.side,
        signal_id: position.signal_id,
        signal_entry: position.signal_entry,
        fill: position.fill,
        exit,
        e: position.e,
        pnl,
        pnl_pct: if position.e.is_zero() {
            Decimal::ZERO
        } else {
            pnl / position.e * Decimal::from(100)
        },
        reason: reason.to_string(),
        entry_index: position.entry_index,
        exit_index: index,
    });
}

fn exit_for(position: &AtrPosition, candle: &Candle) -> Option<(String, Decimal)> {
    let (Some(sl), Some(tp)) = (position.sl, position.tp) else {
        return None;
    };
    let open = candle.open;
    match position.side {
        Direction::Long => {
            if open <= sl {
                Some((REASON_GAP_SL.into(), open))
            } else if open >= tp {
                Some((REASON_GAP_TP.into(), open))
            } else if candle.low <= sl {
                Some((REASON_SL.into(), sl))
            } else if candle.high >= tp {
                Some((REASON_TP.into(), tp))
            } else {
                None
            }
        }
        Direction::Short => {
            if open >= sl {
                Some((REASON_GAP_SL.into(), open))
            } else if open <= tp {
                Some((REASON_GAP_TP.into(), open))
            } else if candle.high >= sl {
                Some((REASON_SL.into(), sl))
            } else if candle.low <= tp {
                Some((REASON_TP.into(), tp))
            } else {
                None
            }
        }
    }
}

fn on_variant(
    label: &str,
    sl_multiplier: Decimal,
    tp_multiplier: Decimal,
    pct_high: Decimal,
    high_adjust: Decimal,
    sl_max_pct: Decimal,
) -> AtrVariant {
    AtrVariant {
        label: label.to_string(),
        enabled: true,
        period: 14,
        sl_multiplier,
        tp_multiplier,
        pct_high,
        high_adjust,
        sl_max_pct,
    }
}

pub fn atr_variants() -> Vec<AtrVariant> {
    vec![
        AtrVariant {
            label: "ATR off".into(),
            enabled: false,
            period: 14,
            sl_multiplier: Decimal::ZERO,
            tp_multiplier: Decimal::ZERO,
            pct_high: Decimal::ZERO,
            high_adjust: Decimal::ONE,
            sl_max_pct: Decimal::ZERO,
        },
        on_variant(
            "ATR14 1.5x/2.0x",
            Decimal::new(15, 1),
            Decimal::from(2),
            Decimal::ZERO,
            Decimal::ONE,
            Decimal::ZERO,
        ),
        on_variant(
            "ATR14 2.0x/3.0x",
            Decimal::from(2),
            Decimal::from(3),
            Decimal::ZERO,
            Decimal::ONE,
            Decimal::ZERO,
        ),
        on_variant(
            "ATR14 2.5x/4.0x",
            Decimal::new(25, 1),
            Decimal::from(4),
            Decimal::ZERO,
            Decimal::ONE,
            Decimal::ZERO,
        ),
        on_variant(
            "ATR14 2.0x/3.0x high-vol x1.5",
            Decimal::from(2),
            Decimal::from(3),
            Decimal::from(5),
            Decimal::new(15, 1),
            Decimal::ZERO,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::types::Signal;

    fn d(value: i64) -> Decimal {
        Decimal::from(value)
    }

    fn candle(i: usize, open: i64, high: i64, low: i64, close: i64) -> Candle {
        Candle {
            open_time_ms: i as i64 * 60_000,
            open: d(open),
            high: d(high),
            low: d(low),
            close: d(close),
            volume: Decimal::ONE,
        }
    }

    fn signal(index: usize, action: Direction) -> SimSignal {
        SimSignal {
            candle_index: index,
            signal: Signal {
                ticker: "TEST-USDT".into(),
                action,
                confidence: Decimal::ONE,
                entry_price: d(100),
                reason: "test".into(),
                signal_candle_open_time_ms: 0,
            },
            signal_id: format!("sig-{index}"),
        }
    }

    fn variant(sl: i64, tp: i64) -> AtrVariant {
        AtrVariant {
            label: "test".into(),
            enabled: true,
            period: 1,
            sl_multiplier: d(sl),
            tp_multiplier: d(tp),
            pct_high: Decimal::ZERO,
            high_adjust: Decimal::ONE,
            sl_max_pct: Decimal::ZERO,
        }
    }

    fn off_variant() -> AtrVariant {
        AtrVariant {
            label: "off".into(),
            enabled: false,
            ..variant(1, 1)
        }
    }

    #[test]
    fn atr_series_matches_the_wilder_ewm_formula() {
        let closes = ["10", "11", "10.5", "11.5", "12", "11", "12.5", "13"];
        let highs = ["11", "12", "11.5", "12.5", "13", "12", "13.5", "14"];
        let lows = ["9", "10", "10", "10.5", "11", "10.5", "11.5", "12"];
        let candles: Vec<Candle> = (0..closes.len())
            .map(|i| Candle {
                open_time_ms: i as i64,
                open: closes[i].parse().unwrap(),
                high: highs[i].parse().unwrap(),
                low: lows[i].parse().unwrap(),
                close: closes[i].parse().unwrap(),
                volume: Decimal::ONE,
            })
            .collect();
        let out = atr_series(&candles, 3);
        let expected = [
            "2.0",
            "2.0",
            "1.8333333333333335",
            "1.888888888888889",
            "1.925925925925926",
            "1.7839506172839505",
            "2.022633744855967",
            "2.0150891632373114",
        ];
        for (i, want) in expected.iter().enumerate() {
            let got = out[i].expect("atr");
            let want: Decimal = want.parse().unwrap();
            assert!(
                (got - want).abs() <= Decimal::new(1, 9),
                "i={i} got={got} want={want}"
            );
        }
    }

    #[test]
    fn long_take_profit_fills_at_the_level_on_the_entry_candle() {
        let candles = vec![
            candle(0, 100, 101, 99, 100),
            candle(1, 100, 103, 99, 101),
            candle(2, 101, 102, 100, 101),
        ];
        let outcome = simulate_atr(
            &candles,
            &[signal(0, Direction::Long)],
            &variant(1, 1),
            1000,
            0,
        );
        assert_eq!(outcome.trades.len(), 1);
        let trade = &outcome.trades[0];
        assert_eq!(trade.reason, REASON_TP);
        assert_eq!(trade.exit, d(102));
        assert_eq!(trade.entry_index, 1);
    }

    #[test]
    fn stop_wins_when_both_levels_are_touched() {
        let candles = vec![candle(0, 100, 101, 99, 100), candle(1, 100, 103, 97, 100)];
        let outcome = simulate_atr(
            &candles,
            &[signal(0, Direction::Long)],
            &variant(1, 1),
            1000,
            0,
        );
        assert_eq!(outcome.trades.len(), 1);
        assert_eq!(outcome.trades[0].reason, REASON_SL);
        assert_eq!(outcome.trades[0].exit, d(98));
    }

    #[test]
    fn gap_exits_use_the_open_before_intraday_levels() {
        let gap_sl = vec![candle(0, 100, 101, 99, 100), candle(1, 95, 99, 94, 96)];
        let sl = simulate_atr(
            &gap_sl,
            &[signal(0, Direction::Long)],
            &variant(1, 1),
            1000,
            0,
        );
        assert_eq!(sl.trades[0].reason, REASON_GAP_SL);
        assert_eq!(sl.trades[0].exit, d(95));

        let gap_tp = vec![candle(0, 100, 101, 99, 100), candle(1, 105, 106, 104, 105)];
        let tp = simulate_atr(
            &gap_tp,
            &[signal(0, Direction::Long)],
            &variant(1, 1),
            1000,
            0,
        );
        assert_eq!(tp.trades[0].reason, REASON_GAP_TP);
        assert_eq!(tp.trades[0].exit, d(105));
    }

    #[test]
    fn atr_off_never_hits_stops() {
        let candles = vec![candle(0, 100, 101, 99, 100), candle(1, 100, 200, 1, 100)];
        let outcome = simulate_atr(
            &candles,
            &[signal(0, Direction::Long)],
            &off_variant(),
            1000,
            0,
        );
        assert!(outcome.trades.is_empty());
        assert_eq!(outcome.open_positions.len(), 1);
    }

    #[test]
    fn a_reversal_exits_at_the_next_open() {
        let candles = vec![
            candle(0, 100, 101, 99, 100),
            candle(1, 100, 101, 99, 100),
            candle(2, 110, 111, 109, 110),
        ];
        let signals = [signal(0, Direction::Long), signal(1, Direction::Short)];
        let outcome = simulate_atr(&candles, &signals, &off_variant(), 1000, 0);
        assert_eq!(outcome.trades.len(), 1);
        assert_eq!(outcome.trades[0].reason, "reversal");
        assert_eq!(outcome.trades[0].exit, d(110));
        assert_eq!(outcome.open_positions.len(), 1);
        assert_eq!(outcome.open_positions[0].side, Direction::Short);
    }

    #[test]
    fn same_side_signals_do_not_add_a_second_position() {
        let candles = vec![
            candle(0, 100, 101, 99, 100),
            candle(1, 100, 101, 99, 100),
            candle(2, 101, 102, 100, 101),
        ];
        let signals = [signal(0, Direction::Long), signal(1, Direction::Long)];
        let outcome = simulate_atr(&candles, &signals, &off_variant(), 1000, 0);
        assert!(outcome.trades.is_empty());
        assert_eq!(outcome.open_positions.len(), 1);
    }

    #[test]
    fn high_volatility_adjusts_both_levels_and_has_no_cap() {
        let candles = vec![candle(0, 100, 106, 94, 100), candle(1, 100, 101, 99, 100)];
        let variant = AtrVariant {
            label: "high".into(),
            enabled: true,
            period: 1,
            sl_multiplier: d(2),
            tp_multiplier: d(3),
            pct_high: d(5),
            high_adjust: Decimal::new(15, 1),
            sl_max_pct: Decimal::ZERO,
        };
        let outcome = simulate_atr(&candles, &[], &variant, 1000, 0);
        assert!(outcome.trades.is_empty());
        // Levels are exercised through the signal path; ATR is 12 here (12% of close).
        let levels = super::atr_levels(&variant, Direction::Long, d(100), d(12)).expect("levels");
        assert_eq!(levels, (d(64), d(154)));
    }

    #[test]
    fn variant_list_has_off_and_the_three_atr_settings() {
        let variants = atr_variants();
        assert!(variants[0].label.to_lowercase().contains("off"));
        assert!(!variants[0].enabled);
        let on: Vec<&AtrVariant> = variants.iter().filter(|v| v.enabled).collect();
        assert_eq!(on.len(), 4);
        assert!(on
            .iter()
            .any(|v| v.sl_multiplier == Decimal::new(15, 1) && v.tp_multiplier == d(2)));
        assert!(on
            .iter()
            .any(|v| v.sl_multiplier == d(2) && v.tp_multiplier == d(3)));
        assert!(on
            .iter()
            .any(|v| v.sl_multiplier == Decimal::new(25, 1) && v.tp_multiplier == d(4)));
        assert!(on.iter().any(|v| v.pct_high == d(5)
            && v.high_adjust == Decimal::new(15, 1)
            && v.sl_max_pct == Decimal::ZERO));
    }
}
