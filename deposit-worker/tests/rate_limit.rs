use tokio::sync::{mpsc, oneshot};
use tonic::Request;

use common::deposit_service_server::DepositService;
use common::TicketRequest;
use deposit::grpc_handler::DepositServer;
use deposit::task::DepositTask;

#[tokio::test]
async fn rejects_when_queue_full() {
    let (tx, mut _rx) = mpsc::channel::<DepositTask>(1);
    let server = DepositServer { tx: tx.clone() };

    // Pre-fill the only slot
    let (reply1, _) = oneshot::channel();
    tx.try_send(DepositTask {
        tx_hash: "0xabc".into(),
        uid: "u1".into(),
        ticker: "ETH".into(),
        reply: reply1,
    })
    .unwrap();

    let result: Result<_, _> = server
        .create_ticket0(Request::new(TicketRequest {
            tx_hash: "0xdef".into(),
            uid: "u2".into(),
            ticker: "ETH".into(),
        }))
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().message().contains("too many"));
}
