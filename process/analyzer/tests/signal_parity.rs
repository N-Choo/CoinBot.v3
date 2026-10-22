mod common;

use analyzer_rs::strategy::generator::signals;
use analyzer_rs::strategy::types::Direction;
use common::{assert_opt_close_at, load_fixture};

const TOLERANCE: f64 = 1e-9;

#[test]
fn signals_match_the_python_oracle_for_every_window() {
    let mut total_positives = 0;

    for fixture_name in common::fixture_names() {
        let f = load_fixture(&fixture_name);
        let candles = f.candles();
        let cfg = f.config();
        assert!(
            !f.signals.is_empty(),
            "{fixture_name} signals must not be empty"
        );
        assert_eq!(f.window, cfg.window, "{fixture_name} fixture/config window");
        assert!(f.window > 0, "{fixture_name} window must be positive");
        assert!(
            candles.len() >= f.window,
            "{fixture_name} has {} candles, fewer than window {}",
            candles.len(),
            f.window
        );
        assert_eq!(
            f.signals.len(),
            candles.len() - f.window + 1,
            "{fixture_name} window count"
        );
        let positives = f
            .signals
            .iter()
            .filter(|item| item.signal.is_some())
            .count();
        let negatives = f.signals.len() - positives;
        total_positives += positives;

        // Keep the synthetic fixture's exact counts as a regression check.
        if fixture_name == "synthetic_1day" {
            assert_eq!(positives, 158, "{fixture_name} positive signal count");
            assert_eq!(negatives, 143, "{fixture_name} None signal count");
        }

        for (index, expected) in f.signals.iter().enumerate() {
            assert_eq!(
                expected.window_end,
                f.window - 1 + index,
                "{fixture_name} window_end sequence"
            );
            let end = expected.window_end;
            assert!(
                end + 1 >= f.window,
                "{fixture_name} window_end={end} is before the first full window"
            );
            assert!(
                end < candles.len(),
                "{fixture_name} window_end={end} exceeds candles"
            );
            let start = end + 1 - f.window;
            let window = &candles[start..=end];
            let actual = signals(window, &f.symbol, &cfg).unwrap_or_else(|e| {
                panic!("{fixture_name} window_end={end}: signals failed: {e:?}")
            });

            match (&expected.signal, actual) {
                (None, None) => {}
                (Some(want), Some(got)) => {
                    let label = format!("{fixture_name} window_end={end}");
                    let expected_action = match want.action.as_str() {
                        "buy" => Direction::Long,
                        "sell" => Direction::Short,
                        other => panic!("{label} unexpected action {other:?}"),
                    };
                    assert_eq!(got.action, expected_action, "{label} action");
                    assert_eq!(got.reason, want.reason, "{label} reason");
                    assert_eq!(got.ticker, f.symbol, "{label} ticker");
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
                (want, got) => {
                    panic!("{fixture_name} window_end={end}: got {got:?}, want {want:?}")
                }
            }
        }
    }

    assert!(
        total_positives > 0,
        "all fixtures together must contain at least one positive signal"
    );
}
