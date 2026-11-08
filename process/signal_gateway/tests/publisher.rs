use std::sync::{Arc, Mutex};

use share::engine_service_server::{EngineService, EngineServiceServer};
use share::{Ack, Empty, Signal as ProtoSignal, Symbols};
use signal_gateway::publisher::{GrpcPublisher, SignalPublisher};
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
            tickers: vec!["BTC-USDT".into()],
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
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let sent = Arc::new(Mutex::new(Vec::new()));
    let engine = TestEngine { sent: sent.clone() };

    tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(EngineServiceServer::new(engine))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .unwrap();
    });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let publisher =
        GrpcPublisher::new(&format!("http://{addr}"), std::time::Duration::from_secs(5)).unwrap();
    let ack = publisher.publish(sample_signal()).await.unwrap();

    assert!(ack.accepted);
    assert_eq!(sent.lock().unwrap().len(), 1);
    assert_eq!(sent.lock().unwrap()[0].signal_id, "sg:1:BTC-USDT");
}
