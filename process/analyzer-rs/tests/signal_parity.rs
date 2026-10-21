mod common;

use analyzer_rs::strategy::generator::signals;
use analyzer_rs::strategy::types::Direction;
use common::{assert_opt_close_at, load_fixture};

const TOLERANCE: f64 = 1e-9;

#[test]
fn signals_match_the_python_oracle_for_every_window() {
    let f = load_fixture("synthetic_1day");
    let candles = f.candles();
    let cfg = f.config();
    assert!(!f.signals.is_empty());
    assert_eq!(f.window, cfg.window);

    for expected in &f.signals {
        let end = expected.window_end;
        let start = end + 1 - f.window;
        let window = &candles[start..=end];
        let actual = signals(window, &f.symbol, &cfg).unwrap();

        match (&expected.signal, actual) {
            (None, None) => {}
            (Some(want), Some(got)) => {
                let label = format!("window_end={end}");
                assert_eq!(
                    got.action,
                    if want.action == "buy" {
                        Direction::Long
                    } else {
                        Direction::Short
                    },
                    "{label} action"
                );
                assert_eq!(got.reason, want.reason, "{label} reason");
                assert_eq!(got.divergence_kind.as_str(), want.kind, "{label} kind");
                assert_opt_close_at(
                    Some(got.confidence),
                    Some(want.confidence.as_str()),
                    TOLERANCE,
                    &format!("{label} confidence"),
                );
                assert_opt_close_at(
                    Some(got.entry_price),
                    Some(want.entry.as_str()),
                    TOLERANCE,
                    &format!("{label} entry"),
                );
                assert_opt_close_at(
                    Some(got.sl_price),
                    Some(want.sl.as_str()),
                    TOLERANCE,
                    &format!("{label} sl"),
                );
                assert_opt_close_at(
                    Some(got.tp_price),
                    Some(want.tp.as_str()),
                    TOLERANCE,
                    &format!("{label} tp"),
                );
                assert_eq!(
                    got.divergence_index, want.divergence_index,
                    "{label} divergence_index"
                );
                assert_eq!(
                    got.divergence_open_time_ms, want.divergence_open_time_ms,
                    "{label} divergence_open_time_ms"
                );
                assert_eq!(
                    got.divergence_age_candles, want.divergence_age_candles,
                    "{label} divergence_age_candles"
                );
                assert_eq!(
                    got.signal_candle_open_time_ms, want.signal_candle_open_time_ms,
                    "{label} signal_candle_open_time_ms"
                );
            }
            (want, got) => panic!("window_end={end}: got {got:?}, want {want:?}"),
        }
    }
}
