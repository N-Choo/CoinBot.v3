mod common;

use analyzer_rs::strategy::divergence::detect;
use analyzer_rs::strategy::indicators::atr::atr;
use analyzer_rs::strategy::indicators::rsi::rsi;
use analyzer_rs::strategy::indicators::sma::sma;
use common::{assert_opt_close, load_fixture};

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

    for i in 0..candles.len() {
        assert_opt_close(rsi_values[i], f.indicators.rsi[i].as_deref(), 1e-9);
        assert_opt_close(sma_values[i], f.indicators.sma[i].as_deref(), 1e-9);
        assert_opt_close(atr_values[i], f.indicators.atr[i].as_deref(), 1e-9);
        assert_eq!(
            divs.regular_bullish[i],
            f.divergences.regular_bullish[i] == 1
        );
        assert_eq!(
            divs.regular_bearish[i],
            f.divergences.regular_bearish[i] == 1
        );
        assert_eq!(divs.hidden_bullish[i], f.divergences.hidden_bullish[i] == 1);
        assert_eq!(divs.hidden_bearish[i], f.divergences.hidden_bearish[i] == 1);
    }
}
