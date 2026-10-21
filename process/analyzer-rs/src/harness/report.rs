use crate::harness::dataset::Provenance;
use crate::harness::replay::{Replay, ReplayEvent};
use crate::harness::sim::SimOutcome;
use crate::harness::stats::SimSummary;
use crate::strategy::config::Config;
use serde_json::{json, Value};
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
                    "action": format!("{:?}", s.signal.action),
                    "reason": s.signal.reason,
                    "signal_entry": s.signal.entry_price.to_string(),
                    "sl": s.signal.sl_price.to_string(),
                    "tp": s.signal.tp_price.to_string(),
                    "divergence_index": s.signal.divergence_index,
                    "divergence_open_time_ms": s.signal.divergence_open_time_ms,
                    "age": s.signal.divergence_age_candles,
                })).collect::<Vec<_>>(),
                "trades": input.sim.trades.iter().map(|t| json!({
                    "entry_index": t.entry_index,
                    "exit_index": t.exit_index,
                    "side": format!("{:?}", t.side),
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
                    "side": format!("{:?}", p.side),
                    "signal_entry": p.signal_entry.to_string(),
                    "fill": p.fill.to_string(),
                    "E": p.e.to_string(),
                    "sl": p.sl.to_string(),
                    "tp": p.tp.to_string(),
                })).collect::<Vec<_>>(),
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
        "returns_label": "gross simulated, excluding fees and slippage",
    })
}

pub fn write_report(payload: &Value, out: &Path) -> std::io::Result<()> {
    let template = include_str!("../../templates/replay.html");
    let json = serde_json::to_string(payload).map_err(std::io::Error::other)?;
    // Keep the embedded JSON from terminating the surrounding <script> element.
    let escaped = json.replace("</", "<\\/");
    let html = template.replace("__REPLAY_DATA__", &escaped);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(out, html)
}
