use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Side {
    #[serde(rename = "LONG")]
    Long,
    #[serde(rename = "SHORT")]
    Short,
    #[serde(rename = "NO-TRADE")]
    NoTrade,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Rsi {
    pub period: u32,
    pub value: f64,
    pub regime: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Prediction {
    pub symbol: String,
    pub side: Side,
    #[serde(default)]
    pub confidence: Option<f64>,
    #[serde(default)]
    pub entry: Option<f64>,
    #[serde(default)]
    pub invalidation: Option<f64>,
    #[serde(default)]
    pub target: Option<f64>,
    #[serde(default)]
    pub rsi: Option<Rsi>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidPrediction {
    pub symbol: String,
    pub side: Side,
    pub confidence: f64,
    pub entry: f64,
    pub invalidation: f64,
    pub target: f64,
    pub rsi: Rsi,
    pub reason: String,
}

pub fn normalize_symbol(symbol: &str) -> String {
    symbol.trim().replace('/', "-").to_uppercase()
}

/// Replace control characters with spaces and cap the result at `max_chars`.
pub fn sanitize_text(input: &str, max_chars: usize) -> String {
    input
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(max_chars)
        .collect()
}

pub fn extract_json(text: &str) -> Option<&str> {
    if let Some(start) = text.find("```json") {
        let after = start + "```json".len();
        let rest = &text[after..];
        if let Some(end) = rest.find("```") {
            return Some(rest[..end].trim());
        }
    }

    let start = text.find('{')?;
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (i, &byte) in bytes.iter().enumerate().skip(start) {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }

        match byte {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[start..=i]);
                }
            }
            _ => {}
        }
    }

    None
}

pub fn price_ok(entry: f64, last: f64, max_drift_pct: f64) -> bool {
    last.is_finite()
        && entry.is_finite()
        && last > 0.0
        && ((entry - last).abs() / last) <= max_drift_pct / 100.0
}

pub fn validate(
    prediction: &Prediction,
    requested_symbol: &str,
    min_confidence: f64,
) -> Result<Option<ValidPrediction>, String> {
    if normalize_symbol(&prediction.symbol) != normalize_symbol(requested_symbol) {
        return Err(format!(
            "symbol mismatch: {} != {}",
            prediction.symbol, requested_symbol
        ));
    }

    if prediction.side == Side::NoTrade {
        return Ok(None);
    }

    let confidence = prediction.confidence.ok_or("missing confidence")?;
    if !confidence.is_finite() || !(0.0..=1.0).contains(&confidence) {
        return Err("confidence must be finite and in [0, 1]".into());
    }
    if confidence < min_confidence {
        return Err(format!(
            "confidence {confidence} below minimum {min_confidence}"
        ));
    }

    let entry = prediction.entry.ok_or("missing entry")?;
    let invalidation = prediction.invalidation.ok_or("missing invalidation")?;
    let target = prediction.target.ok_or("missing target")?;

    for (name, value) in [
        ("entry", entry),
        ("invalidation", invalidation),
        ("target", target),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(format!("{name} must be finite and > 0"));
        }
    }

    match prediction.side {
        Side::Long if !(invalidation < entry && entry < target) => {
            return Err("LONG levels must satisfy invalidation < entry < target".into());
        }
        Side::Short if !(target < entry && entry < invalidation) => {
            return Err("SHORT levels must satisfy target < entry < invalidation".into());
        }
        _ => {}
    }

    let rsi = prediction.rsi.as_ref().ok_or("missing rsi")?;
    if rsi.period == 0 || rsi.period > 200 {
        return Err("rsi.period must be in 1..=200".into());
    }
    if !rsi.value.is_finite() || !(0.0..=100.0).contains(&rsi.value) {
        return Err("rsi.value must be finite and in [0, 100]".into());
    }
    if !["bullish", "neutral-to-bullish", "bearish"].contains(&rsi.regime.as_str()) {
        return Err(format!("unknown rsi.regime {:?}", rsi.regime));
    }

    let reason = prediction.reason.as_deref().unwrap_or("").trim();
    if reason.is_empty() {
        return Err("reason must not be empty".into());
    }

    Ok(Some(ValidPrediction {
        symbol: normalize_symbol(&prediction.symbol),
        side: prediction.side,
        confidence,
        entry,
        invalidation,
        target,
        rsi: rsi.clone(),
        reason: reason.to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_json() -> &'static str {
        r#"{
            "symbol": "BTC-USDT",
            "side": "LONG",
            "confidence": 0.72,
            "entry": 68100.0,
            "invalidation": 65900.0,
            "target": 72500.0,
            "rsi": {"period": 21, "value": 63.4, "regime": "bullish"},
            "reason": "ETF inflows"
        }"#
    }

    fn prediction(text: &str) -> Prediction {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn extracts_fenced_json() {
        let text = format!("Here you go:\n```json\n{}\n```\nDone", valid_json());
        assert_eq!(extract_json(&text).unwrap(), valid_json());
    }

    #[test]
    fn extracts_bare_object() {
        let text = format!("Result: {}", valid_json());
        assert_eq!(extract_json(&text).unwrap(), valid_json());
    }

    #[test]
    fn returns_none_without_json() {
        assert!(extract_json("no object here").is_none());
    }

    #[test]
    fn accepts_valid_long() {
        let valid = validate(&prediction(valid_json()), "BTC-USDT", 0.55)
            .unwrap()
            .unwrap();
        assert_eq!(valid.side, Side::Long);
        assert_eq!(valid.confidence, 0.72);
        assert_eq!(valid.symbol, "BTC-USDT");
    }

    #[test]
    fn accepts_valid_short() {
        let json = valid_json()
            .replace("\"LONG\"", "\"SHORT\"")
            .replace("\"invalidation\": 65900.0", "\"invalidation\": 70500.0")
            .replace("\"target\": 72500.0", "\"target\": 66000.0");
        let valid = validate(&prediction(&json), "BTC-USDT", 0.55)
            .unwrap()
            .unwrap();
        assert_eq!(valid.side, Side::Short);
    }

    #[test]
    fn no_trade_publishes_nothing() {
        let json = valid_json()
            .replace("\"LONG\"", "\"NO-TRADE\"")
            .replace("\"confidence\": 0.72,", "")
            .replace("\"entry\": 68100.0,", "")
            .replace("\"invalidation\": 65900.0,", "")
            .replace("\"target\": 72500.0,", "");
        let result = validate(&prediction(&json), "BTC-USDT", 0.55).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn rejects_symbol_mismatch() {
        let err = validate(&prediction(valid_json()), "ETH-USDT", 0.55).unwrap_err();
        assert!(err.contains("symbol mismatch"));
    }

    #[test]
    fn rejects_low_confidence() {
        let err = validate(&prediction(valid_json()), "BTC-USDT", 0.9).unwrap_err();
        assert!(err.contains("below minimum"));
    }

    #[test]
    fn rejects_wrong_long_ordering() {
        let json = valid_json().replace("\"invalidation\": 65900.0", "\"invalidation\": 70000.0");
        let err = validate(&prediction(&json), "BTC-USDT", 0.55).unwrap_err();
        assert!(err.contains("invalidation < entry < target"));
    }

    #[test]
    fn rejects_wrong_short_ordering() {
        let json = valid_json()
            .replace("\"LONG\"", "\"SHORT\"")
            .replace("\"invalidation\": 65900.0", "\"invalidation\": 66000.0")
            .replace("\"target\": 72500.0", "\"target\": 70000.0");
        let err = validate(&prediction(&json), "BTC-USDT", 0.55).unwrap_err();
        assert!(err.contains("target < entry < invalidation"));
    }

    #[test]
    fn rejects_bad_rsi_regime() {
        let json = valid_json().replace("\"bullish\"", "\"overbought\"");
        let err = validate(&prediction(&json), "BTC-USDT", 0.55).unwrap_err();
        assert!(err.contains("rsi.regime"));
    }

    #[test]
    fn price_check_uses_percent_drift() {
        assert!(price_ok(100.0, 101.0, 5.0));
        assert!(!price_ok(100.0, 106.0, 5.0));
        assert!(!price_ok(100.0, 0.0, 5.0));
    }

    #[test]
    fn normalizes_slash_and_case() {
        assert_eq!(normalize_symbol(" btc/usdt "), "BTC-USDT");
    }

    #[test]
    fn sanitizes_control_characters_and_truncates() {
        let sanitized = sanitize_text("a\nb\tc\u{1b}[31m", 100);
        assert_eq!(sanitized, "a b c [31m");
        assert_eq!(sanitize_text("abcdef", 3), "abc");
    }
}
