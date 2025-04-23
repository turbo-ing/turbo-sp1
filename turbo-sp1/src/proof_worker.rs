use std::sync::Arc;

use alloy_sol_types::SolValue;
use serde::Serialize;
use sp1_sdk::EnvProver;
use tokio::sync::{mpsc, Mutex};
use turbo_program::traits::TurboActionSerialization;

use crate::{
    proof::{handle_proof_request, ProofType},
    prove_queue::{ProveQueue, ProveStatus},
    session::TurboSession,
};

type TaskId = String;
pub type ProofJob<PublicState, PrivateState, GameAction> =
    (TaskId, ProofRequest<PublicState, PrivateState, GameAction>);

#[derive(Clone)]
pub struct ProofRequest<
    PublicState: Serialize + Default + Send + Sync,
    PrivateState: Default + Send + Sync,
    GameAction: TurboActionSerialization + Send + Sync,
> {
    session: Arc<Mutex<TurboSession<PublicState, PrivateState, GameAction>>>,
    proof_type: ProofType,
    client: Arc<EnvProver>,
    elf: Arc<Vec<u8>>,
}

impl<PublicState, PrivateState, GameAction> ProofRequest<PublicState, PrivateState, GameAction>
where
    PublicState: Serialize + Default + Send + Sync,
    PrivateState: Default + Send + Sync,
    GameAction: TurboActionSerialization + Send + Sync,
{
    pub fn new(
        session: Arc<Mutex<TurboSession<PublicState, PrivateState, GameAction>>>,
        proof_type: ProofType,
        client: Arc<EnvProver>,
        elf: Arc<Vec<u8>>,
    ) -> Self {
        Self {
            session,
            proof_type,
            client,
            elf,
        }
    }
}
/// Spawn `num_workers` background tasks that consume `rx_jobs`.
pub fn spawn_proof_workers<PublicState, PrivateState, GameAction>(
    num_workers: usize,
    rx_jobs: mpsc::UnboundedReceiver<ProofJob<PublicState, PrivateState, GameAction>>,
    queue: Arc<ProveQueue>,
) where
    PublicState: Default
        + SolValue
        + Serialize
        + From<<<PublicState as SolValue>::SolType as alloy_sol_types::SolType>::RustType>
        + Send
        + Sync
        + 'static,
    PrivateState: Default + Send + Sync + 'static,
    GameAction: TurboActionSerialization + Send + Sync + 'static,
{
    let rx = Arc::new(Mutex::new(rx_jobs));

    for _ in 0..num_workers {
        let rx = rx.clone();
        let queue = queue.clone();

        tokio::spawn(async move {
            loop {
                // lock only while calling `recv`
                let msg_opt = {
                    let mut guard = rx.lock().await;
                    guard.recv().await
                };

                let (task_id, job) = match msg_opt {
                    Some(m) => m,
                    None => break, // all senders dropped => exit
                };

                queue.set_status(&task_id, ProveStatus::InProgress);

                let result = handle_proof_request::<PublicState, PrivateState, GameAction>(
                    job.session,
                    job.client,
                    job.elf,
                    job.proof_type,
                    task_id.clone(),
                )
                .await
                .map(ProveStatus::Done)
                .unwrap_or_else(|e| ProveStatus::Error(e.to_string()));

                queue.set_status(&task_id, result);
            }
        });
    }
}
