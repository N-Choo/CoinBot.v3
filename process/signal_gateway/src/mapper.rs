use chrono::{DateTime, TimeDelta, Utc};
use share::Signal as ProtoSignal;

use crate::prediction::{Side, ValidPrediction};
use crate::scheduler;

pub const SCHEMA_VERSION: u32 = 1;

pub fn signal_id(candle_open_ms: i64, ticker: &str) -> String {
    format!("sg:{candle_open_ms}:{ticker}")
}

/// Map a validated prediction to a wire signal. Returns `None` for
/// `NO-TRADE` and when the expiry would overflow the clock.
pub fn to_proto(
    prediction: &ValidPrediction,
    candle_open: DateTime<Utc>,
    emitted_at: DateTime<Utc>,
    seq: u64,
    ttl: TimeDelta,
) -> Option<ProtoSignal> {
    let open_ms = candle_open.timestamp_millis();
    let trigger = scheduler::candle_close(candle_open);
    let expires = emitted_at.checked_add_signed(ttl)?;
    let (action, label) = match prediction.side {
        Side::Long => ("BUY", "LONG"),
        Side::Short => ("SELL", "SHORT"),
        Side::NoTrade => return None,
    };

    let reason = format!(
        "agent {label} conf={:.2} rsi={:.1}({}) {}",
        prediction.confidence, prediction.rsi.value, prediction.rsi.regime, prediction.reason
    );
    let reason = crate::prediction::sanitize_text(&reason, 500);

    Some(ProtoSignal {
        ticker: prediction.symbol.replace('-', "/"),
        action: action.to_string(),
        entry_price: prediction.entry,
        schema_version: SCHEMA_VERSION,
        signal_id: signal_id(open_ms, &prediction.symbol),
        seq,
        expires_at_ms: expires.timestamp_millis(),
        trigger_time_ms: trigger.timestamp_millis(),
        sl_price: prediction.invalidation,
        tp_price: prediction.target,
        zone_id: String::new(),
        reason,
    })
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

    fn emitted() -> DateTime<Utc> {
        candle() + TimeDelta::hours(4) + TimeDelta::seconds(90)
    }

    #[test]
    fn maps_long_to_buy() {
        let signal = to_proto(&prediction(), candle(), emitted(), 7, TimeDelta::hours(4)).unwrap();
        assert_eq!(signal.action, "BUY");
        assert_eq!(signal.ticker, "BTC/USDT");
        assert_eq!(signal.entry_price, 100.0);
        assert_eq!(signal.sl_price, 95.0);
        assert_eq!(signal.tp_price, 110.0);
        assert_eq!(signal.schema_version, SCHEMA_VERSION);
        assert_eq!(signal.seq, 7);
        assert_eq!(signal.zone_id, "");
        assert_eq!(signal.reason, "agent LONG conf=0.72 rsi=63.4(bullish) news");
    }

    #[test]
    fn maps_short_to_sell() {
        let mut p = prediction();
        p.side = Side::Short;
        p.entry = 100.0;
        p.invalidation = 105.0;
        p.target = 90.0;
        let signal = to_proto(&p, candle(), emitted(), 1, TimeDelta::hours(4)).unwrap();
        assert_eq!(signal.action, "SELL");
        assert_eq!(signal.sl_price, 105.0);
        assert_eq!(signal.tp_price, 90.0);
    }

    #[test]
    fn signal_id_is_side_independent() {
        let open = candle();
        let mut p = prediction();
        let long = to_proto(&p, open, emitted(), 1, TimeDelta::hours(4)).unwrap();
        p.side = Side::Short;
        p.invalidation = 105.0;
        p.target = 90.0;
        let short = to_proto(&p, open, emitted(), 2, TimeDelta::hours(4)).unwrap();
        assert_eq!(long.signal_id, short.signal_id);
        assert_eq!(long.signal_id, "sg:1794009600000:BTC-USDT");
    }

    #[test]
    fn expiry_uses_emission_time_and_ttl() {
        let signal = to_proto(&prediction(), candle(), emitted(), 1, TimeDelta::hours(1)).unwrap();
        assert_eq!(
            signal.expires_at_ms,
            (emitted() + TimeDelta::hours(1)).timestamp_millis()
        );
        assert_eq!(signal.trigger_time_ms, 1794024000000);
        assert_ne!(signal.expires_at_ms, signal.trigger_time_ms);
    }

    #[test]
    fn reason_is_truncated_to_five_hundred_chars() {
        let mut p = prediction();
        p.reason = "x".repeat(1000);
        let signal = to_proto(&p, candle(), emitted(), 1, TimeDelta::hours(4)).unwrap();
        assert_eq!(signal.reason.chars().count(), 500);
        assert!(
            signal
                .reason
                .starts_with("agent LONG conf=0.72 rsi=63.4(bullish) ")
        );
    }

    #[test]
    fn reason_strips_newlines_from_untrusted_text() {
        let mut p = prediction();
        p.reason = "line1\nline2".into();
        let signal = to_proto(&p, candle(), emitted(), 1, TimeDelta::hours(4)).unwrap();
        assert!(!signal.reason.contains('\n'));
        assert!(signal.reason.contains("line1 line2"));
    }

    #[test]
    fn no_trade_is_not_mapped() {
        let mut p = prediction();
        p.side = Side::NoTrade;
        assert!(to_proto(&p, candle(), emitted(), 1, TimeDelta::hours(4)).is_none());
    }

    #[test]
    fn overflowing_expiry_is_not_mapped() {
        assert!(to_proto(&prediction(), candle(), emitted(), 1, TimeDelta::MAX).is_none());
    }
}
