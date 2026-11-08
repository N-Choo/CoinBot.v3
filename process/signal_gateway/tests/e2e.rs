use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{TimeZone, Utc};
use share::engine_service_server::{EngineService, EngineServiceServer};
use share::{Ack, Empty, Signal as ProtoSignal, Symbols};
use signal_gateway::agent_client::OpenCodeClient;
use signal_gateway::config::{Config, PriceCheck};
use signal_gateway::price::KuCoinPrice;
use signal_gateway::publisher::GrpcPublisher;
use signal_gateway::runner::{Gateway, RetryPolicy};
use signal_gateway::scheduler;
use signal_gateway::state::State;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

struct TestEngine {
    sent: Arc<Mutex<Vec<ProtoSignal>>>,
}

#[tonic::async_trait]
impl EngineService for TestEngine {
    async fn get_active_symbols(
        &self,
        _request: tonic::Request<Empty>,
    ) -> Result<tonic::Response<Symbols>, tonic::Status> {
        Ok(tonic::Response::new(Symbols {
            tickers: vec!["BTC-USDT".into(), "ETH-USDT".into()],
        }))
    }

    async fn publish(
        &self,
        request: tonic::Request<ProtoSignal>,
    ) -> Result<tonic::Response<Ack>, tonic::Status> {
        self.sent.lock().unwrap().push(request.into_inner());
        Ok(tonic::Response::new(Ack {
            accepted: true,
            reason: "handled".into(),
        }))
    }
}

fn long_json(symbol: &str) -> String {
    format!(
        r#"```json
{{
  "symbol": "{symbol}",
  "side": "LONG",
  "confidence": 0.72,
  "entry": 100.0,
  "invalidation": 95.0,
  "target": 110.0,
  "rsi": {{"period": 21, "value": 63.4, "regime": "bullish"}},
  "reason": "news"
}}
```"#
    )
}

struct PredictResponder;

impl Respond for PredictResponder {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let body = String::from_utf8_lossy(&request.body);
        let symbol = if body.contains("Symbol: ETH-USDT") {
            "ETH-USDT"
        } else {
            "BTC-USDT"
        };
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "parts": [{"type": "text", "text": long_json(symbol)}]
        }))
    }
}

async fn spawn_engine(sent: Arc<Mutex<Vec<ProtoSignal>>>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let engine = TestEngine { sent };
    tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(EngineServiceServer::new(engine))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .unwrap();
    });

    for _ in 0..50 {
        if tokio::net::TcpStream::connect(addr).await.is_ok() {
            return format!("http://{addr}");
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("engine did not become ready");
}

async fn mount_opencode(server: &MockServer, calls: u64) {
    Mock::given(method("POST"))
        .and(path("/session"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "s1"})))
        .expect(calls)
        .mount(server)
        .await;

    Mock::given(method("POST"))
        .and(path("/session/s1/message"))
        .respond_with(PredictResponder)
        .expect(calls)
        .mount(server)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/session/s1"))
        .respond_with(ResponseTemplate::new(200))
        .expect(calls)
        .mount(server)
        .await;
}

async fn mount_price(server: &MockServer, symbol: &str, price: &str, calls: u64) {
    Mock::given(method("GET"))
        .and(path("/api/v1/market/orderbook/level1"))
        .and(query_param("symbol", symbol))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": {"price": price}})),
        )
        .expect(calls)
        .mount(server)
        .await;
}

async fn build_gateway(
    opencode: &MockServer,
    kucoin: &MockServer,
    engine_endpoint: &str,
    dir: &std::path::Path,
) -> Gateway {
    let mut config = Config::from_lookup(|_| None).unwrap();
    config.state_path = dir.join("state.json");
    config.audit_path = dir.join("audit.jsonl");
    config.opencode_base_url = opencode.uri();
    config.grpc_endpoint = engine_endpoint.to_string();
    config.kucoin_base_url = kucoin.uri();
    config.price_check = PriceCheck::Strict;
    config.min_confidence = 0.55;
    config.max_entry_drift_pct = 5.0;
    config.signal_ttl = chrono::TimeDelta::hours(4);

    let agent = Arc::new(
        OpenCodeClient::new(
            config.opencode_base_url.as_str(),
            config.opencode_username.as_str(),
            None,
            config.opencode_agent.as_str(),
            None,
            config.delete_sessions,
            Duration::from_secs(5),
        )
        .unwrap(),
    );
    let publisher = Arc::new(
        GrpcPublisher::new(config.grpc_endpoint.as_str(), Duration::from_secs(5)).unwrap(),
    );
    let price = Arc::new(
        KuCoinPrice::new(config.kucoin_base_url.as_str(), Duration::from_secs(5)).unwrap(),
    );

    Gateway::new(
        config,
        State::default(),
        agent,
        publisher,
        price,
        RetryPolicy {
            agent_delays: vec![],
            grpc_delays: vec![],
        },
    )
}

#[tokio::test]
async fn end_to_end_cycle_publishes_valid_predictions() {
    let opencode = MockServer::start().await;
    let kucoin = MockServer::start().await;
    mount_opencode(&opencode, 2).await;
    mount_price(&kucoin, "BTC-USDT", "100.5", 1).await;
    mount_price(&kucoin, "ETH-USDT", "100.5", 1).await;

    let sent = Arc::new(Mutex::new(Vec::new()));
    let endpoint = spawn_engine(sent.clone()).await;
    let dir = tempfile::tempdir().unwrap();
    let mut gateway = build_gateway(&opencode, &kucoin, &endpoint, dir.path()).await;

    let candle = Utc.with_ymd_and_hms(2026, 11, 7, 0, 0, 0).unwrap();
    let trigger = scheduler::candle_close(candle).timestamp_millis();
    gateway
        .run_cycle(&["BTC-USDT".to_string(), "ETH-USDT".to_string()], candle)
        .await
        .unwrap();

    {
        let sent = sent.lock().unwrap();
        assert_eq!(sent.len(), 2);

        assert_eq!(sent[0].ticker, "BTC-USDT");
        assert_eq!(sent[0].action, "BUY");
        assert_eq!(sent[0].entry_price, 100.0);
        assert_eq!(sent[0].sl_price, 95.0);
        assert_eq!(sent[0].tp_price, 110.0);
        assert_eq!(
            sent[0].signal_id,
            format!("sg:{}:BTC-USDT", candle.timestamp_millis())
        );
        assert_eq!(sent[0].seq, 0);
        assert_eq!(sent[0].schema_version, 1);
        assert_eq!(sent[0].trigger_time_ms, trigger);
        assert!(sent[0].expires_at_ms > sent[0].trigger_time_ms);
        assert!(
            sent[0]
                .reason
                .contains("agent LONG conf=0.72 rsi=63.4(bullish)")
        );

        assert_eq!(sent[1].ticker, "ETH-USDT");
        assert_eq!(sent[1].action, "BUY");
        assert_eq!(sent[1].seq, 1);
        assert_eq!(
            sent[1].signal_id,
            format!("sg:{}:ETH-USDT", candle.timestamp_millis())
        );
    }

    let audit = std::fs::read_to_string(dir.path().join("audit.jsonl")).unwrap();
    assert_eq!(audit.lines().count(), 2);
    assert!(audit.contains(&format!("sg:{}:BTC-USDT", candle.timestamp_millis())));
    assert!(audit.contains(&format!("sg:{}:ETH-USDT", candle.timestamp_millis())));

    let state = std::fs::read_to_string(dir.path().join("state.json")).unwrap();
    assert!(state.contains("BTC-USDT"));
    assert!(state.contains("ETH-USDT"));
}

#[tokio::test]
async fn publishes_nothing_when_price_check_rejects() {
    let opencode = MockServer::start().await;
    let kucoin = MockServer::start().await;
    mount_opencode(&opencode, 1).await;
    mount_price(&kucoin, "BTC-USDT", "200.0", 1).await;

    let sent = Arc::new(Mutex::new(Vec::new()));
    let endpoint = spawn_engine(sent.clone()).await;
    let dir = tempfile::tempdir().unwrap();
    let mut gateway = build_gateway(&opencode, &kucoin, &endpoint, dir.path()).await;

    let candle = Utc.with_ymd_and_hms(2026, 11, 7, 0, 0, 0).unwrap();
    gateway
        .run_cycle(&["BTC-USDT".to_string()], candle)
        .await
        .unwrap();

    assert!(sent.lock().unwrap().is_empty());
    assert!(
        gateway
            .state
            .is_processed("BTC-USDT", candle.timestamp_millis())
    );

    let audit = std::fs::read_to_string(dir.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("rejected: entry 100 is more than 5% from last 200"));
}
