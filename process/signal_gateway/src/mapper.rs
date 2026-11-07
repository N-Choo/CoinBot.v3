use chrono::{DateTime, TimeDelta, Utc};
use share::Signal as ProtoSignal;

use crate::prediction::{Side, ValidPrediction};

pub const SCHEMA_VERSION: u32 = 1;

pub fn signal_id(candle_open_ms: i64, ticker: &str) -> String {
    format!("sg:{candle_open_ms}:{ticker}")
}

pub fn to_proto(
    prediction: &ValidPrediction,
    candle_open: DateTime<Utc>,
    seq: u64,
    ttl: TimeDelta,
) -> ProtoSignal {
    let open_ms = candle_open.timestamp_millis();
    let close_ms = (candle_open + TimeDelta::hours(4)).timestamp_millis();
    let (action, label) = match prediction.side {
        Side::Long => ("BUY", "LONG"),
        Side::Short => ("SELL", "SHORT"),
        Side::NoTrade => unreachable!("NO-TRADE is not mapped"),
    };

    let reason = format!(
        "agent {label} conf={:.2} rsi={:.1}({}) {}",
        prediction.confidence, prediction.rsi.value, prediction.rsi.regime, prediction.reason
    );

    ProtoSignal {
        ticker: prediction.symbol.clone(),
        action: action.to_string(),
        entry_price: prediction.entry,
        schema_version: SCHEMA_VERSION,
        signal_id: signal_id(open_ms, &prediction.symbol),
        seq,
        expires_at_ms: (candle_open + ttl).timestamp_millis(),
        trigger_time_ms: close_ms,
        sl_price: prediction.invalidation,
        tp_price: prediction.target,
        zone_id: String::new(),
        reason: reason.chars().take(500).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prediction::Rsi;
    use chrono::TimeZone;

    fn prediction() -> ValidPrediction {
        ValidPrediction {
            symbol: "BTC-USDT".into(),
            side: Side::Long,
            confidence: 0.72,
            entry: 100.0,
            invalidation: 95.0,
            target: 110.0,
            rsi: Rsi {
                period: 21,
                value: 63.4,
                regime: "bullish".into(),
            },
            reason: "news".into(),
        }
    }

    fn candle() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 7, 0, 0, 0).unwrap()
    }

    #[test]
    fn maps_long_to_buy() {
        let signal = to_proto(&prediction(), candle(), 7, TimeDelta::hours(4));
        assert_eq!(signal.action, "BUY");
        assert_eq!(signal.ticker, "BTC-USDT");
        assert_eq!(signal.entry_price, 100.0);
        assert_eq!(signal.sl_price, 95.0);
        assert_eq!(signal.tp_price, 110.0);
        assert_eq!(signal.schema_version, SCHEMA_VERSION);
        assert_eq!(signal.seq, 7);
    }

    #[test]
    fn maps_short_to_sell() {
        let mut p = prediction();
        p.side = Side::Short;
        p.entry = 100.0;
        p.invalidation = 105.0;
        p.target = 90.0;
        let signal = to_proto(&p, candle(), 1, TimeDelta::hours(4));
        assert_eq!(signal.action, "SELL");
    }

    #[test]
    fn signal_id_is_side_independent() {
        let open = candle();
        let mut p = prediction();
        let long = to_proto(&p, open, 1, TimeDelta::hours(4));
        p.side = Side::Short;
        p.invalidation = 105.0;
        p.target = 90.0;
        let short = to_proto(&p, open, 2, TimeDelta::hours(4));
        assert_eq!(long.signal_id, short.signal_id);
        assert_eq!(long.signal_id, "sg:1794009600000:BTC-USDT");
    }

    #[test]
    fn expiry_uses_the_configured_ttl() {
        let signal = to_proto(&prediction(), candle(), 1, TimeDelta::hours(4));
        assert_eq!(signal.expires_at_ms, 1794024000000);
        assert_eq!(signal.trigger_time_ms, 1794024000000);
    }

    #[test]
    fn reason_is_truncated_to_five_hundred_chars() {
        let mut p = prediction();
        p.reason = "x".repeat(1000);
        let signal = to_proto(&p, candle(), 1, TimeDelta::hours(4));
        assert_eq!(signal.reason.chars().count(), 500);
    }
}
