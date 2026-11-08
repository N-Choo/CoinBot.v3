use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use share::{Ack, ProcessError, Signal as ProtoSignal};
use signal_gateway::agent_client::AgentClient;
use signal_gateway::config::{Config, PriceCheck};
use signal_gateway::price::LastPriceSource;
use signal_gateway::publisher::SignalPublisher;
use signal_gateway::runner::{Gateway, RetryPolicy};
use signal_gateway::state::State;

type Harness = (
    Gateway,
    Arc<Mutex<Vec<String>>>,
    Arc<Mutex<Vec<ProtoSignal>>>,
);

struct FakeAgent {
    calls: Arc<Mutex<Vec<String>>>,
    response: String,
    fail: bool,
}

#[async_trait]
impl AgentClient for FakeAgent {
    async fn predict(
        &self,
        symbol: &str,
        _candle_open: chrono::DateTime<Utc>,
    ) -> Result<String, ProcessError> {
        self.calls.lock().unwrap().push(symbol.to_string());
        if self.fail {
            return Err(ProcessError::Network("agent down".into()));
        }
        Ok(self.response.replace("{symbol}", symbol))
    }
}

struct FakePrice;

#[async_trait]
impl LastPriceSource for FakePrice {
    async fn last_price(&self, _symbol: &str) -> Result<f64, ProcessError> {
        Ok(100.0)
    }
}

struct FakePublisher {
    sent: Arc<Mutex<Vec<ProtoSignal>>>,
}

#[async_trait]
impl SignalPublisher for FakePublisher {
    async fn publish(&self, signal: ProtoSignal) -> Result<Ack, ProcessError> {
        self.sent.lock().unwrap().push(signal);
        Ok(Ack {
            accepted: true,
            reason: "handled".into(),
        })
    }
}

fn config(dir: &std::path::Path) -> Config {
    let mut cfg = Config::from_lookup(|_| None).unwrap();
    cfg.state_path = dir.join("state.json");
    cfg.audit_path = dir.join("audit.jsonl");
    cfg.price_check = PriceCheck::Strict;
    cfg.min_confidence = 0.55;
    cfg
}

fn long_json() -> &'static str {
    r#"{
      "symbol": "{symbol}",
      "side": "LONG",
      "confidence": 0.72,
      "entry": 100.0,
      "invalidation": 95.0,
      "target": 110.0,
      "rsi": {"period": 21, "value": 63.4, "regime": "bullish"},
      "reason": "news"
    }"#
}

fn build(dir: &std::path::Path, state: State, response: &str, fail: bool) -> Harness {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let sent = Arc::new(Mutex::new(Vec::new()));
    let gateway = Gateway::new(
        config(dir),
        state,
        Arc::new(FakeAgent {
            calls: calls.clone(),
            response: response.into(),
            fail,
        }),
        Arc::new(FakePublisher { sent: sent.clone() }),
        Arc::new(FakePrice),
        RetryPolicy {
            agent_delays: vec![],
            grpc_delays: vec![],
        },
    );
    (gateway, calls, sent)
}

fn no_trade_json() -> &'static str {
    r#"{"symbol": "{symbol}", "side": "NO-TRADE"}"#
}

#[tokio::test]
async fn processes_symbols_in_rotated_order() {
    let dir = tempfile::tempdir().unwrap();
    let state = State {
        cursor: 1,
        ..State::default()
    };
    let (mut gateway, calls, _) = build(dir.path(), state, long_json(), false);

    let candle = Utc.with_ymd_and_hms(2026, 11, 7, 0, 0, 0).unwrap();
    gateway
        .run_cycle(&["A-USDT".into(), "B-USDT".into(), "C-USDT".into()], candle)
        .await
        .unwrap();

    assert_eq!(*calls.lock().unwrap(), vec!["B-USDT", "C-USDT", "A-USDT"]);
    assert_eq!(gateway.state.cursor, 2);
}

#[tokio::test]
async fn no_trade_marks_processed_without_publishing() {
    let dir = tempfile::tempdir().unwrap();
    let (mut gateway, _, sent) = build(dir.path(), State::default(), no_trade_json(), false);
    let candle = Utc.with_ymd_and_hms(2026, 11, 7, 0, 0, 0).unwrap();

    gateway.run_cycle(&["A-USDT".into()], candle).await.unwrap();

    assert!(sent.lock().unwrap().is_empty());
    assert!(
        gateway
            .state
            .is_processed("A-USDT", candle.timestamp_millis())
    );
}

#[tokio::test]
async fn valid_prediction_is_published_once() {
    let dir = tempfile::tempdir().unwrap();
    let (mut gateway, _, sent) = build(dir.path(), State::default(), long_json(), false);
    let candle = Utc.with_ymd_and_hms(2026, 11, 7, 0, 0, 0).unwrap();

    gateway.run_cycle(&["A-USDT".into()], candle).await.unwrap();

    let sent = sent.lock().unwrap();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].action, "BUY");
    assert_eq!(
        sent[0].signal_id,
        format!("sg:{}:A-USDT", candle.timestamp_millis())
    );
}

#[tokio::test]
async fn agent_failure_leaves_symbol_unprocessed() {
    let dir = tempfile::tempdir().unwrap();
    let (mut gateway, _, sent) = build(dir.path(), State::default(), long_json(), true);
    let candle = Utc.with_ymd_and_hms(2026, 11, 7, 0, 0, 0).unwrap();

    gateway.run_cycle(&["A-USDT".into()], candle).await.unwrap();

    assert!(sent.lock().unwrap().is_empty());
    assert!(
        !gateway
            .state
            .is_processed("A-USDT", candle.timestamp_millis())
    );
}

#[tokio::test]
async fn repeated_candle_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let candle = Utc.with_ymd_and_hms(2026, 11, 7, 0, 0, 0).unwrap();
    let mut state = State::default();
    state.mark_processed("A-USDT", candle.timestamp_millis());
    let (mut gateway, calls, _) = build(dir.path(), state, long_json(), false);

    gateway.run_cycle(&["A-USDT".into()], candle).await.unwrap();

    assert!(calls.lock().unwrap().is_empty());
}
