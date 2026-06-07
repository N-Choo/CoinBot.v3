use tokio::sync::{mpsc, oneshot};
use tonic::{Request, Response, Status};

use crate::task::DepositTask;
use common::deposit_service_server::DepositService;
use common::{TicketRequest, TicketResponse};

pub struct DepositServer {
    pub tx: mpsc::Sender<DepositTask>,
}

#[tonic::async_trait]
impl DepositService for DepositServer {
    async fn create_ticket0(
        &self,
        req: Request<TicketRequest>,
    ) -> Result<Response<TicketResponse>, Status> {
        let req = req.into_inner();
        log::info!("create_ticket0: tx={} ticker={}", req.tx_hash, req.ticker);

        let (reply, rx) = oneshot::channel();
        self.tx
            .try_send(DepositTask {
                tx_hash: req.tx_hash,
                uid: req.uid,
                ticker: req.ticker,
                reply,
            })
            .map_err(|_| Status::resource_exhausted("too many requests — retry later"))?;

        let resp = rx
            .await
            .map_err(|_| Status::internal("worker dropped"))?
            .map(Response::new)?;

        Ok(resp)
    }
}
