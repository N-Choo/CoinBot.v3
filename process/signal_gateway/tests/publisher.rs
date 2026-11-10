use std::sync::{Arc, Mutex};
use std::time::Duration;

use share::engine_service_server::{EngineService, EngineServiceServer};
use share::{Ack, Empty, ProcessError, Signal as ProtoSignal, Symbols};
use signal_gateway::publisher::{GrpcPublisher, SignalPublisher};
use signal_gateway::symbols::fetch_active_symbols;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;

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
            tickers: vec!["btc/usdt".into(), " BTC-USDT ".into(), String::new()],
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

struct SlowEngine;

#[tonic::async_trait]
impl EngineService for SlowEngine {
    async fn get_active_symbols(
        &self,
        _request: tonic::Request<Empty>,
    ) -> Result<tonic::Response<Symbols>, tonic::Status> {
        Ok(tonic::Response::new(Symbols::default()))
    }

    async fn publish(
        &self,
        _request: tonic::Request<ProtoSignal>,
    ) -> Result<tonic::Response<Ack>, tonic::Status> {
        tokio::time::sleep(Duration::from_secs(2)).await;
        Ok(tonic::Response::new(Ack {
            accepted: true,
            reason: "slow".into(),
        }))
    }
}

struct RejectingEngine;

#[tonic::async_trait]
impl EngineService for RejectingEngine {
    async fn get_active_symbols(
        &self,
        _request: tonic::Request<Empty>,
    ) -> Result<tonic::Response<Symbols>, tonic::Status> {
        Ok(tonic::Response::new(Symbols::default()))
    }

    async fn publish(
        &self,
        _request: tonic::Request<ProtoSignal>,
    ) -> Result<tonic::Response<Ack>, tonic::Status> {
        Err(tonic::Status::invalid_argument("bad"))
    }
}

struct DuplicateEngine;

#[tonic::async_trait]
impl EngineService for DuplicateEngine {
    async fn get_active_symbols(
        &self,
        _request: tonic::Request<Empty>,
    ) -> Result<tonic::Response<Symbols>, tonic::Status> {
        Ok(tonic::Response::new(Symbols::default()))
    }

    async fn publish(
        &self,
        _request: tonic::Request<ProtoSignal>,
    ) -> Result<tonic::Response<Ack>, tonic::Status> {
        Ok(tonic::Response::new(Ack {
            accepted: false,
            reason: "duplicate".into(),
        }))
    }
}

/// Start a tonic engine server on an ephemeral port and return its endpoint.
async fn spawn_engine<E: EngineService + Send + Sync + 'static>(engine: E) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(EngineServiceServer::new(engine))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .unwrap();
    });
    tokio::time::sleep(Duration::from_millis(100)).await;

    format!("http://{addr}")
}

fn sample_signal() -> ProtoSignal {
    ProtoSignal {
        ticker: "BTC-USDT".into(),
        action: "BUY".into(),
        entry_price: 100.0,
        schema_version: 1,
        signal_id: "sg:1:BTC-USDT".into(),
        seq: 1,
        expires_at_ms: 0,
        trigger_time_ms: 0,
        sl_price: 95.0,
        tp_price: 110.0,
        zone_id: String::new(),
        reason: "test".into(),
    }
}

#[tokio::test]
async fn publishes_over_grpc() {
    let sent = Arc::new(Mutex::new(Vec::new()));
    let endpoint = spawn_engine(TestEngine { sent: sent.clone() }).await;

    let publisher =
        GrpcPublisher::new(&endpoint, Duration::from_secs(5), "test-token".into()).unwrap();
    let ack = publisher.publish(sample_signal()).await.unwrap();

    assert!(ack.accepted);
    assert_eq!(sent.lock().unwrap().len(), 1);
    assert_eq!(sent.lock().unwrap()[0].signal_id, "sg:1:BTC-USDT");
}

#[tokio::test]
async fn fetch_active_symbols_normalizes() {
    let sent = Arc::new(Mutex::new(Vec::new()));
    let endpoint = spawn_engine(TestEngine { sent }).await;

    let symbols = fetch_active_symbols(&endpoint, Duration::from_secs(5), "test-token")
        .await
        .unwrap();

    assert_eq!(symbols, vec!["BTC-USDT"]);
}

#[tokio::test]
async fn invalid_endpoint_is_config_error() {
    let err = fetch_active_symbols("not a url", Duration::from_secs(5), "test-token")
        .await
        .unwrap_err();

    assert!(matches!(err, ProcessError::InvalidConfig(_)), "got {err:?}");
}

#[tokio::test]
async fn publish_times_out() {
    let endpoint = spawn_engine(SlowEngine).await;
    let publisher =
        GrpcPublisher::new(&endpoint, Duration::from_millis(50), "test-token".into()).unwrap();

    let err = publisher.publish(sample_signal()).await.unwrap_err();

    assert!(matches!(err, ProcessError::Network(_)), "got {err:?}");
}

#[tokio::test]
async fn rejects_non_retryable_status() {
    let endpoint = spawn_engine(RejectingEngine).await;
    let publisher =
        GrpcPublisher::new(&endpoint, Duration::from_secs(5), "test-token".into()).unwrap();

    let err = publisher.publish(sample_signal()).await.unwrap_err();

    assert!(matches!(err, ProcessError::InvalidConfig(_)), "got {err:?}");
}

#[tokio::test]
async fn passes_duplicate_ack_through() {
    let endpoint = spawn_engine(DuplicateEngine).await;
    let publisher =
        GrpcPublisher::new(&endpoint, Duration::from_secs(5), "test-token".into()).unwrap();

    let ack = publisher.publish(sample_signal()).await.unwrap();

    assert_eq!(
        ack,
        Ack {
            accepted: false,
            reason: "duplicate".into(),
        }
    );
}
