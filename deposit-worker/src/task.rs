use tokio::sync::{mpsc, oneshot};
use tonic::Status;

use common::TicketResponse;
use share::db::deposit::Deposit;
use share::rpc::Rpc;

pub struct DepositTask {
    pub tx_hash: String,
    pub uid: String,
    pub ticker: String,
    pub reply: oneshot::Sender<Result<TicketResponse, Status>>,
}

pub async fn process_task(
    pool: &sqlx::PgPool,
    tx_hash: &str,
    uid: &str,
    ticker: &str,
) -> Result<TicketResponse, Status> {
    let tx = Rpc::get_transaction(tx_hash).await.map_err(|e| {
        log::warn!("get_transaction failed: {e}");
        Status::invalid_argument(format!("invalid tx: {e}"))
    })?;

    let user_uid = uid
        .parse()
        .map_err(|_| Status::invalid_argument("invalid uid"))?;

    let deposit = Deposit::create_pending(pool, user_uid, tx_hash, &tx, ticker)
        .await
        .map_err(|e| {
            log::error!("create_pending failed: {e}");
            Status::internal(e.to_string())
        })?;

    log::info!("ticket created: {}", deposit.id);
    Ok(TicketResponse {
        ticket_id: deposit.id.to_string(),
        timestamp: deposit.created_at.to_rfc3339(),
        accepted: true,
    })
}

pub fn run_dispatcher(
    pool: sqlx::PgPool,
    mut rx: mpsc::Receiver<DepositTask>,
    semaphore: std::sync::Arc<tokio::sync::Semaphore>,
) {
    tokio::spawn(async move {
        while let Some(task) = rx.recv().await {
            let permit = match semaphore.clone().acquire_owned().await {
                Ok(p) => p,
                Err(_) => {
                    log::error!("Semaphore closed --- shutting down dispatcher");
                    break;
                }
            };
            let pool = pool.clone();
            tokio::spawn(async move {
                let result = process_task(&pool, &task.tx_hash, &task.uid, &task.ticker).await;
                task.reply.send(result).ok();
                drop(permit);
            });
        }
    });
}
