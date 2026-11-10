use tokio::sync::{mpsc, oneshot};
use tonic::Request;

use deposit::grpc_handler::DepositServer;
use deposit::task::DepositTask;
use share::deposit_service_server::DepositService;
use share::TicketRequest;

#[tokio::test]
async fn rejects_when_queue_full() {
    let (tx, mut _rx) = mpsc::channel::<DepositTask>(1);
    let server = DepositServer {
        tx: tx.clone(),
        token: "test-token".into(),
    };

    // Pre-fill the only slot
    let (reply1, _) = oneshot::channel();
    tx.try_send(DepositTask {
        tx_hash: "0xabc".into(),
        uid: "u1".into(),
        ticker: "ETH".into(),
        reply: reply1,
    })
    .unwrap();

    let request = share::grpc_auth::with_token(
        TicketRequest {
            tx_hash: "0xdef".into(),
            uid: "u2".into(),
            ticker: "ETH".into(),
        },
        "test-token",
    )
    .unwrap();
    let result: Result<_, _> = server.create_ticket0(request).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().message().contains("too many"));
}

#[tokio::test]
async fn rejects_a_missing_token() {
    let (tx, _rx) = mpsc::channel::<DepositTask>(1);
    let server = DepositServer {
        tx,
        token: "test-token".into(),
    };

    let result = server
        .create_ticket0(Request::new(TicketRequest {
            tx_hash: "0xdef".into(),
            uid: "u2".into(),
            ticker: "ETH".into(),
        }))
        .await;

    assert_eq!(result.unwrap_err().code(), tonic::Code::Unauthenticated);
}
