mod common;

use analyzer_rs::strategy::divergence::detect;
use analyzer_rs::strategy::indicators::atr::atr;
use analyzer_rs::strategy::indicators::rsi::rsi;
use analyzer_rs::strategy::indicators::sma::sma;
use common::{assert_opt_close_at, load_fixture};

const TOLERANCE: f64 = 1e-9;

#[test]
fn indicators_match_the_python_oracle() {
    let f = load_fixture("synthetic_1day");
    let candles = f.candles();
    let closes: Vec<Option<rust_decimal::Decimal>> =
        candles.iter().map(|c| Some(c.close)).collect();
    let cfg = f.config();

    let rsi_values = rsi(&closes, cfg.rsi_period).unwrap();
    let sma_values = sma(&closes, cfg.sma_period).unwrap();
    let atr_values = atr(&candles, cfg.atr_period).unwrap();
    let divs = detect(&candles, &cfg).unwrap();

    let n = candles.len();
    assert_eq!(rsi_values.len(), n, "rsi length");
    assert_eq!(sma_values.len(), n, "sma length");
    assert_eq!(atr_values.len(), n, "atr length");
    assert_eq!(divs.rsi.len(), n, "divergence rsi length");
    assert_eq!(divs.regular_bullish.len(), n, "regular_bullish length");
    assert_eq!(divs.regular_bearish.len(), n, "regular_bearish length");
    assert_eq!(divs.hidden_bullish.len(), n, "hidden_bullish length");
    assert_eq!(divs.hidden_bearish.len(), n, "hidden_bearish length");
    assert_eq!(f.indicators.rsi.len(), n, "fixture rsi length");
    assert_eq!(f.indicators.sma.len(), n, "fixture sma length");
    assert_eq!(f.indicators.atr.len(), n, "fixture atr length");
    assert_eq!(
        f.divergences.regular_bullish.len(),
        n,
        "fixture regular_bullish length"
    );
    assert_eq!(
        f.divergences.regular_bearish.len(),
        n,
        "fixture regular_bearish length"
    );
    assert_eq!(
        f.divergences.hidden_bullish.len(),
        n,
        "fixture hidden_bullish length"
    );
    assert_eq!(
        f.divergences.hidden_bearish.len(),
        n,
        "fixture hidden_bearish length"
    );

    for (name, flags) in [
        ("regular_bullish", &f.divergences.regular_bullish),
        ("regular_bearish", &f.divergences.regular_bearish),
        ("hidden_bullish", &f.divergences.hidden_bullish),
        ("hidden_bearish", &f.divergences.hidden_bearish),
    ] {
        let mut positives = 0;
        for (i, &value) in flags.iter().enumerate() {
            assert!(
                matches!(value, 0 | 1),
                "{name}[{i}] must be 0 or 1, got {value}"
            );
            if value == 1 {
                positives += 1;
            }
        }
        assert!(positives > 0, "{name} must contain at least one positive");
    }

    for i in 0..n {
        assert_opt_close_at(
            rsi_values[i],
            f.indicators.rsi[i].as_deref(),
            TOLERANCE,
            &format!("rsi[{i}]"),
        );
        assert_opt_close_at(
            sma_values[i],
            f.indicators.sma[i].as_deref(),
            TOLERANCE,
            &format!("sma[{i}]"),
        );
        assert_opt_close_at(
            atr_values[i],
            f.indicators.atr[i].as_deref(),
            TOLERANCE,
            &format!("atr[{i}]"),
        );
        assert_opt_close_at(
            divs.rsi[i],
            f.indicators.rsi[i].as_deref(),
            TOLERANCE,
            &format!("divergence_rsi[{i}]"),
        );
        assert_eq!(
            divs.regular_bullish[i],
            f.divergences.regular_bullish[i] == 1,
            "regular_bullish[{i}]"
        );
        assert_eq!(
            divs.regular_bearish[i],
            f.divergences.regular_bearish[i] == 1,
            "regular_bearish[{i}]"
        );
        assert_eq!(
            divs.hidden_bullish[i],
            f.divergences.hidden_bullish[i] == 1,
            "hidden_bullish[{i}]"
        );
        assert_eq!(
            divs.hidden_bearish[i],
            f.divergences.hidden_bearish[i] == 1,
            "hidden_bearish[{i}]"
        );
    }
}
