use crate::harness::dataset::Provenance;
use crate::harness::replay::{Replay, ReplayEvent, ReplaySignal};
use crate::harness::sim::{unrealized_pnl, AccountStep, Position, SimOutcome};
use crate::harness::stats::SimSummary;
use crate::strategy::config::Config;
use crate::strategy::types::Direction;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;

pub struct PayloadInput<'a> {
    pub symbol: &'a str,
    pub replay: &'a Replay,
    pub sim: &'a SimOutcome,
    pub summary: &'a SimSummary,
    pub provenance: &'a Provenance,
}

pub fn build_payload(
    inputs: &[PayloadInput<'_>],
    config: &Config,
    generated_at: String,
    failures: Vec<Value>,
) -> Value {
    let symbols: Vec<Value> = inputs
        .iter()
        .map(|input| {
            let last_close = input
                .replay
                .candles
                .last()
                .map(|candle| candle.close)
                .unwrap_or_default();
            json!({
                "symbol": input.symbol,
                "candles": input.replay.candles.iter().map(|c| json!({
                    "t": c.open_time_ms,
                    "o": c.open.to_string(),
                    "h": c.high.to_string(),
                    "l": c.low.to_string(),
                    "c": c.close.to_string(),
                })).collect::<Vec<_>>(),
                "rsi": input.replay.rsi.iter().map(|v| v.map(|d| d.to_string())).collect::<Vec<_>>(),
                "sma": input.replay.sma.iter().map(|v| v.map(|d| d.to_string())).collect::<Vec<_>>(),
                "events": input.replay.events.iter().map(event_json).collect::<Vec<_>>(),
                "signals": input.replay.signals.iter().map(|s| json!({
                    "candle_index": s.candle_index,
                    "signal_id": s.signal_id,
                    "action": direction_str(s.signal.action),
                    "reason": s.signal.reason,
                    "confidence": s.signal.confidence.to_string(),
                    "signal_entry": s.signal.entry_price.to_string(),
                    "sl": s.signal.sl_price.to_string(),
                    "tp": s.signal.tp_price.to_string(),
                    "divergence_index": s.signal.divergence_index,
                    "divergence_kind": s.signal.divergence_kind.as_str(),
                    "divergence_open_time_ms": s.signal.divergence_open_time_ms,
                    "signal_candle_open_time_ms": s.signal.signal_candle_open_time_ms,
                    "age": s.signal.divergence_age_candles,
                })).collect::<Vec<_>>(),
                "trades": input.sim.trades.iter().map(|t| json!({
                    "entry_index": t.entry_index,
                    "exit_index": t.exit_index,
                    "side": direction_str(t.side),
                    "signal_id": t.signal_id,
                    "signal_entry": t.signal_entry.to_string(),
                    "fill": t.fill.to_string(),
                    "exit": t.exit.to_string(),
                    "E": t.e.to_string(),
                    "pnl": t.pnl.to_string(),
                    "pnl_pct": t.pnl_pct.to_string(),
                    "risk_reward": t.risk_reward.map(|v| v.to_string()),
                    "reason": t.reason,
                })).collect::<Vec<_>>(),
                "open_positions": input.sim.open_positions.iter().map(|p| json!({
                    "entry_index": p.entry_index,
                    "side": direction_str(p.side),
                    "signal_id": p.signal_id,
                    "signal_entry": p.signal_entry.to_string(),
                    "fill": p.fill.to_string(),
                    "E": p.e.to_string(),
                    "sl": p.sl.to_string(),
                    "tp": p.tp.to_string(),
                    "unrealized": position_unrealized(p, last_close).to_string(),
                })).collect::<Vec<_>>(),
                "account": account_payload(&input.replay.signals, input.sim, &input.sim.account),
                "provenance": input.provenance,
                "summary": summary_json(input.summary, input.provenance.gaps),
            })
        })
        .collect();

    json!({
        "generated_at": generated_at,
        "config": {
            // timeframe, limit, init_fund, and as_of_ms are overwritten by the CLI.
            "timeframe": "",
            "limit": 0,
            "window": config.window,
            "init_fund": "1000",
            "as_of_ms": 0,
            "fees": false,
            "rsi_period": config.rsi_period,
            "rsi_order": config.rsi_order,
            "price_diff_threshold": config.price_diff_threshold.to_string(),
            "sma_enabled": config.sma_enabled,
            "sma_period": config.sma_period,
            "atr_period": config.atr_period,
            "atr_sl_multiplier": config.atr_sl_multiplier.to_string(),
            "atr_tp_multiplier": config.atr_tp_multiplier.to_string(),
            "atr_pct_high": config.atr_pct_high.to_string(),
            "atr_high_adjust": config.atr_high_adjust.to_string(),
            "atr_sl_max_pct": config.atr_sl_max_pct.to_string(),
        },
        "symbols": symbols,
        "failures": failures,
    })
}

fn direction_str(direction: Direction) -> &'static str {
    match direction {
        Direction::Long => "Long",
        Direction::Short => "Short",
    }
}

/// Builds the per-step account payload, enriching each open position with the
/// side, fill, size, SL, and TP needed by the read-only UI.
fn account_payload(
    signals: &[ReplaySignal],
    sim: &SimOutcome,
    account: &[AccountStep],
) -> Vec<Value> {
    let signals_by_id: HashMap<&str, &ReplaySignal> =
        signals.iter().map(|s| (s.signal_id.as_str(), s)).collect();

    // Closed trades do not carry SL/TP, but they do carry the fill and size
    // that were active while the position was open. Build a unified lookup for
    // those fields so every account step can render its open rows without JS
    // recomputation. SL/TP come from the originating signal.
    let mut position_meta_by_id: HashMap<&str, (&Decimal, &Decimal, Direction, usize)> =
        HashMap::new();
    for position in &sim.open_positions {
        position_meta_by_id.insert(
            position.signal_id.as_str(),
            (
                &position.fill,
                &position.e,
                position.side,
                position.entry_index,
            ),
        );
    }
    for trade in &sim.trades {
        position_meta_by_id.insert(
            trade.signal_id.as_str(),
            (&trade.fill, &trade.e, trade.side, trade.entry_index),
        );
    }

    account
        .iter()
        .map(|a| {
            json!({
                "i": a.index,
                "available_fund": a.available_fund.to_string(),
                "reserved": a.reserved.to_string(),
                "unrealized": a.unrealized.to_string(),
                "equity": a.equity.to_string(),
                "open": a.open.iter().map(|o| {
                    let (fill, e, side, entry_index) = position_meta_by_id
                        .get(o.signal_id.as_str())
                        .map(|(f, sz, s, i)| (Some(*f), Some(*sz), Some(*s), Some(*i)))
                        .unwrap_or((None, None, None, None));
                    let signal = signals_by_id.get(o.signal_id.as_str()).copied();
                    json!({
                        "signal_id": o.signal_id,
                        "side": side.map(direction_str),
                        "fill": fill.map(|v| v.to_string()),
                        "e": e.map(|v| v.to_string()),
                        "entry_index": entry_index,
                        "sl": signal.map(|s| s.signal.sl_price.to_string()),
                        "tp": signal.map(|s| s.signal.tp_price.to_string()),
                        "unrealized": o.unrealized.to_string(),
                    })
                }).collect::<Vec<_>>(),
            })
        })
        .collect()
}

fn position_unrealized(position: &Position, last_close: Decimal) -> Decimal {
    unrealized_pnl(position.side, position.fill, last_close, position.e)
}

fn event_json(event: &ReplayEvent) -> Value {
    match event {
        ReplayEvent::Warmup {
            candle_index,
            seeded_keys,
        } => json!({"i": candle_index, "kind": "warmup", "seeded_keys": seeded_keys}),
        ReplayEvent::Signal {
            candle_index,
            signal_index,
        } => json!({"i": candle_index, "kind": "signal", "signal_index": signal_index}),
        ReplayEvent::Suppressed {
            candle_index,
            reason,
        } => json!({"i": candle_index, "kind": "suppressed", "reason": reason}),
    }
}

fn summary_json(summary: &SimSummary, gaps: usize) -> Value {
    let profit_factor = if let Some(ratio) = summary.profit_factor {
        ratio.to_string()
    } else if summary.profit_factor_is_infinite {
        "inf".to_string()
    } else {
        "null".to_string()
    };
    json!({
        "init_fund": summary.init_fund.to_string(),
        "available_fund": summary.available_fund.to_string(),
        "reserved": summary.reserved.to_string(),
        "realized_pnl": summary.realized_pnl.to_string(),
        "unrealized_pnl": summary.unrealized_pnl.to_string(),
        "equity": summary.equity.to_string(),
        "gross_realized_pnl": summary.realized_pnl.to_string(),
        "profit_factor": profit_factor,
        "max_drawdown": summary.max_drawdown.to_string(),
        "max_drawdown_pct": summary.max_drawdown_pct.to_string(),
        "trades": summary.trades,
        "wins": summary.wins,
        "losses": summary.losses,
        "breakeven": summary.breakeven,
        "win_rate": summary.win_rate.to_string(),
        "avg_win": summary.avg_win.to_string(),
        "avg_loss": summary.avg_loss.to_string(),
        "exposure_candles": summary.exposure_candles,
        "time_in_market_pct": summary.time_in_market_pct.to_string(),
        "open_count": summary.open_count,
        "skipped_funds": summary.skipped_funds,
        "unfilled_at_end": summary.unfilled_at_end,
        "invalid_levels": summary.invalid_levels,
        "gaps": gaps,
        "tp_hits": summary.tp_hits,
        "sl_hits": summary.sl_hits,
        "other_exits": summary.other_exits,
        "avg_risk_reward": summary.avg_risk_reward.map(|v| v.to_string()),
        "returns_label": "gross simulated, excluding fees and slippage",
    })
}

pub fn write_report(payload: &Value, out: &Path) -> std::io::Result<()> {
    let template = include_str!("../../templates/replay.html");
    let json = serde_json::to_string(payload).map_err(std::io::Error::other)?;
    // Escape every `<` so the embedded JSON can never open or close an HTML tag,
    // including tricky forms such as `<!--<script` and case-insensitive `</SCRIPT>`.
    let escaped = json.replace('<', "\\u003c");
    let html = template.replace("__REPLAY_DATA__", &escaped);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(out, html)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::dataset::{EffectiveRange, RequestedRange};
    use crate::harness::sim::{ClosedTrade, Position};
    use rust_decimal::Decimal;

    fn provenance() -> Provenance {
        Provenance {
            source: "test".into(),
            requested: RequestedRange {
                limit: 0,
                timeframe: "1day".into(),
                as_of_ms: 0,
            },
            effective_range: EffectiveRange {
                first_open_time_ms: 0,
                last_open_time_ms: 0,
                candles: 0,
            },
            content_sha256: "0".repeat(64),
            fetched_at_ms: None,
            gaps: 0,
        }
    }

    fn summary(profit_factor: Option<Decimal>, infinite: bool) -> SimSummary {
        SimSummary {
            profit_factor,
            profit_factor_is_infinite: infinite,
            ..SimSummary::default()
        }
    }

    #[test]
    fn summary_profit_factor_branches() {
        let finite = summary_json(&summary(Some(Decimal::new(21, 1)), false), 0);
        assert_eq!(finite["profit_factor"], "2.1");

        let infinite = summary_json(&summary(None, true), 0);
        assert_eq!(infinite["profit_factor"], "inf");

        let absent = summary_json(&summary(None, false), 0);
        assert_eq!(absent["profit_factor"], "null");
    }

    #[test]
    fn open_position_unrealized_marks_at_the_last_close() {
        let long = Position {
            side: Direction::Long,
            signal_id: "sig-long".into(),
            signal_entry: Decimal::from(100),
            fill: Decimal::from(100),
            e: Decimal::from(500),
            sl: Decimal::from(90),
            tp: Decimal::from(120),
            entry_index: 0,
        };
        assert_eq!(
            position_unrealized(&long, Decimal::from(90)),
            Decimal::from(-50)
        );

        let short = Position {
            side: Direction::Short,
            ..long
        };
        assert_eq!(
            position_unrealized(&short, Decimal::from(90)),
            Decimal::from(50)
        );
    }

    #[test]
    fn directions_serialize_explicitly() {
        let provenance = provenance();
        let replay = Replay {
            symbol: "TEST-USDT".into(),
            candles: Vec::new(),
            rsi: Vec::new(),
            sma: Vec::new(),
            events: Vec::new(),
            signals: Vec::new(),
            provenance: provenance.clone(),
        };
        let sim = SimOutcome {
            trades: vec![ClosedTrade {
                side: Direction::Long,
                signal_id: "sig-a".into(),
                signal_entry: Decimal::ONE,
                fill: Decimal::ONE,
                exit: Decimal::ONE,
                e: Decimal::ONE,
                pnl: Decimal::ZERO,
                pnl_pct: Decimal::ZERO,
                risk_reward: None,
                reason: "TP".into(),
                entry_index: 0,
                exit_index: 1,
            }],
            open_positions: vec![Position {
                side: Direction::Short,
                signal_id: "sig-b".into(),
                signal_entry: Decimal::ONE,
                fill: Decimal::ONE,
                e: Decimal::ONE,
                sl: Decimal::ONE,
                tp: Decimal::ONE,
                entry_index: 0,
            }],
            ..SimOutcome::default()
        };
        let summary = SimSummary::default();
        let payload = build_payload(
            &[PayloadInput {
                symbol: "TEST-USDT",
                replay: &replay,
                sim: &sim,
                summary: &summary,
                provenance: &provenance,
            }],
            &Config::default(),
            "2026-10-21T00:00:00Z".into(),
            Vec::new(),
        );

        assert_eq!(payload["symbols"][0]["trades"][0]["side"], "Long");
        assert_eq!(payload["symbols"][0]["open_positions"][0]["side"], "Short");
    }
}
