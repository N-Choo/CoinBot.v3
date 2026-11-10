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

struct FakePrice {
    price: f64,
    fail: bool,
}

#[async_trait]
impl LastPriceSource for FakePrice {
    async fn last_price(&self, _symbol: &str) -> Result<f64, ProcessError> {
        if self.fail {
            return Err(ProcessError::Network("price down".into()));
        }
        Ok(self.price)
    }
}

struct FakePublisher {
    sent: Arc<Mutex<Vec<ProtoSignal>>>,
    fail: bool,
    ack: Ack,
}

#[async_trait]
impl SignalPublisher for FakePublisher {
    async fn publish(&self, signal: ProtoSignal) -> Result<Ack, ProcessError> {
        if self.fail {
            return Err(ProcessError::Network("publisher down".into()));
        }
        self.sent.lock().unwrap().push(signal);
        Ok(self.ack.clone())
    }
}

struct Fakes {
    response: String,
    agent_fail: bool,
    price: f64,
    price_fail: bool,
    publisher_fail: bool,
    ack: Ack,
}

impl Default for Fakes {
    fn default() -> Self {
        Self {
            response: long_json().into(),
            agent_fail: false,
            price: 100.0,
            price_fail: false,
            publisher_fail: false,
            ack: Ack {
                accepted: true,
                reason: "handled".into(),
            },
        }
    }
}

fn config(dir: &std::path::Path) -> Config {
    let mut cfg =
        Config::from_lookup(|key| (key == "GRPC_AUTH_TOKEN").then(|| "test-token".to_string()))
            .unwrap();
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

fn no_trade_json() -> &'static str {
    r#"{"symbol": "{symbol}", "side": "NO-TRADE"}"#
}

type Harness = (
    Gateway,
    Arc<Mutex<Vec<String>>>,
    Arc<Mutex<Vec<ProtoSignal>>>,
);

fn build(dir: &std::path::Path, state: State, fakes: Fakes) -> Harness {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let sent = Arc::new(Mutex::new(Vec::new()));
    let gateway = Gateway::new(
        config(dir),
        state,
        Arc::new(FakeAgent {
            calls: calls.clone(),
            response: fakes.response,
            fail: fakes.agent_fail,
        }),
        Arc::new(FakePublisher {
            sent: sent.clone(),
            fail: fakes.publisher_fail,
            ack: fakes.ack,
        }),
        Arc::new(FakePrice {
            price: fakes.price,
            fail: fakes.price_fail,
        }),
        RetryPolicy {
            agent_delays: vec![],
            grpc_delays: vec![],
        },
    );
    (gateway, calls, sent)
}

fn candle() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, 0, 0, 0).unwrap()
}

#[tokio::test]
async fn processes_symbols_in_rotated_order() {
    let dir = tempfile::tempdir().unwrap();
    let state = State {
        cursor: 1,
        ..State::default()
    };
    let (mut gateway, calls, _) = build(dir.path(), state, Fakes::default());

    gateway
        .run_cycle(
            &["A-USDT".into(), "B-USDT".into(), "C-USDT".into()],
            candle(),
        )
        .await
        .unwrap();

    assert_eq!(*calls.lock().unwrap(), vec!["B-USDT", "C-USDT", "A-USDT"]);
    assert_eq!(gateway.state.cursor, 2);
}

#[tokio::test]
async fn no_trade_marks_processed_without_publishing() {
    let dir = tempfile::tempdir().unwrap();
    let fakes = Fakes {
        response: no_trade_json().into(),
        ..Fakes::default()
    };
    let (mut gateway, _, sent) = build(dir.path(), State::default(), fakes);

    gateway
        .run_cycle(&["A-USDT".into()], candle())
        .await
        .unwrap();

    assert!(sent.lock().unwrap().is_empty());
    assert!(
        gateway
            .state
            .is_processed("A-USDT", candle().timestamp_millis())
    );
}

#[tokio::test]
async fn valid_prediction_is_published_once() {
    let dir = tempfile::tempdir().unwrap();
    let (mut gateway, _, sent) = build(dir.path(), State::default(), Fakes::default());

    gateway
        .run_cycle(&["A-USDT".into()], candle())
        .await
        .unwrap();

    let sent = sent.lock().unwrap();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].action, "BUY");
    assert_eq!(
        sent[0].signal_id,
        format!("sg:{}:A-USDT", candle().timestamp_millis())
    );
}

#[tokio::test]
async fn agent_failure_leaves_symbol_unprocessed() {
    let dir = tempfile::tempdir().unwrap();
    let fakes = Fakes {
        agent_fail: true,
        ..Fakes::default()
    };
    let (mut gateway, _, sent) = build(dir.path(), State::default(), fakes);

    gateway
        .run_cycle(&["A-USDT".into()], candle())
        .await
        .unwrap();

    assert!(sent.lock().unwrap().is_empty());
    assert!(
        !gateway
            .state
            .is_processed("A-USDT", candle().timestamp_millis())
    );
}

#[tokio::test]
async fn repeated_candle_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let state = State::default();
    let mut state = state;
    state.mark_processed("A-USDT", candle().timestamp_millis());
    let (mut gateway, calls, _) = build(dir.path(), state, Fakes::default());

    gateway
        .run_cycle(&["A-USDT".into()], candle())
        .await
        .unwrap();

    assert!(calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn price_drift_is_rejected_and_processed() {
    let dir = tempfile::tempdir().unwrap();
    let fakes = Fakes {
        price: 200.0,
        ..Fakes::default()
    };
    let (mut gateway, _, sent) = build(dir.path(), State::default(), fakes);

    gateway
        .run_cycle(&["A-USDT".into()], candle())
        .await
        .unwrap();

    assert!(sent.lock().unwrap().is_empty());
    assert!(
        gateway
            .state
            .is_processed("A-USDT", candle().timestamp_millis())
    );
}

#[tokio::test]
async fn price_fetch_failure_marks_processed() {
    let dir = tempfile::tempdir().unwrap();
    let fakes = Fakes {
        price_fail: true,
        ..Fakes::default()
    };
    let (mut gateway, _, sent) = build(dir.path(), State::default(), fakes);

    gateway
        .run_cycle(&["A-USDT".into()], candle())
        .await
        .unwrap();

    assert!(sent.lock().unwrap().is_empty());
    assert!(
        gateway
            .state
            .is_processed("A-USDT", candle().timestamp_millis())
    );
}

#[tokio::test]
async fn publisher_failure_leaves_unprocessed() {
    let dir = tempfile::tempdir().unwrap();
    let fakes = Fakes {
        publisher_fail: true,
        ..Fakes::default()
    };
    let (mut gateway, _, sent) = build(dir.path(), State::default(), fakes);

    gateway
        .run_cycle(&["A-USDT".into()], candle())
        .await
        .unwrap();

    assert!(sent.lock().unwrap().is_empty());
    assert!(
        !gateway
            .state
            .is_processed("A-USDT", candle().timestamp_millis())
    );
}

#[tokio::test]
async fn duplicate_ack_is_success() {
    let dir = tempfile::tempdir().unwrap();
    let fakes = Fakes {
        ack: Ack {
            accepted: false,
            reason: "duplicate".into(),
        },
        ..Fakes::default()
    };
    let (mut gateway, _, sent) = build(dir.path(), State::default(), fakes);

    gateway
        .run_cycle(&["A-USDT".into()], candle())
        .await
        .unwrap();

    assert_eq!(sent.lock().unwrap().len(), 1);
    assert!(
        gateway
            .state
            .is_processed("A-USDT", candle().timestamp_millis())
    );
}

#[tokio::test]
async fn audit_failure_does_not_block_processing() {
    let dir = tempfile::tempdir().unwrap();
    let (mut gateway, _, sent) = build(dir.path(), State::default(), Fakes::default());
    let audit_dir = dir.path().join("audit_dir");
    std::fs::create_dir(&audit_dir).unwrap();
    gateway.config.audit_path = audit_dir;

    gateway
        .run_cycle(&["A-USDT".into()], candle())
        .await
        .unwrap();

    assert_eq!(sent.lock().unwrap().len(), 1);
    assert!(
        gateway
            .state
            .is_processed("A-USDT", candle().timestamp_millis())
    );
}

#[tokio::test]
async fn audit_contains_published_signal() {
    let dir = tempfile::tempdir().unwrap();
    let (mut gateway, _, _) = build(dir.path(), State::default(), Fakes::default());

    gateway
        .run_cycle(&["A-USDT".into()], candle())
        .await
        .unwrap();

    let raw = std::fs::read_to_string(dir.path().join("audit.jsonl")).unwrap();
    let signal_id = format!("sg:{}:A-USDT", candle().timestamp_millis());
    assert!(raw.contains("\"symbol\":\"A-USDT\""));
    assert!(raw.contains(&signal_id));
}
