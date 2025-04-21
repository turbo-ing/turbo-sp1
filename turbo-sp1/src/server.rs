use alloy_sol_types::SolValue;
use serde::Serialize;
use serde_json::json;
use std::{convert::Infallible, sync::Arc};
use tokio::sync::{mpsc, Mutex};
use warp::Filter;

use sp1_sdk::ProverClient;
use turbo_sp1_program::{program::TurboReducer, traits::TurboActionSerialization};

use crate::proof::{handle_proof_request, ProofType};
use crate::proof_worker::{spawn_proof_workers, ProofJob, ProofRequest};
use crate::prove_queue::{ProveQueue, ProveStatus};
use crate::session_manager::SessionManager;
use crate::session_simple::create_session_json;
use crate::warp::rejection::{handle_rejection, ServerError};

pub fn turbo_sp1_routes<PublicState, PrivateState, GameAction>(
    elf: &[u8],
    reducer: TurboReducer<PublicState, PrivateState, GameAction>,
    num_workers: usize,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Infallible> + Clone
where
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
    let client_arc = Arc::new(ProverClient::from_env());
    let elf_arc = Arc::new(elf.to_vec());
    let prove_queue_arc = Arc::new(ProveQueue::new());
    let session_manager_arc = Arc::new(Mutex::new(SessionManager::new()));
    let (tx_jobs, rx_jobs) =
        mpsc::unbounded_channel::<ProofJob<PublicState, PrivateState, GameAction>>();
    let tx_jobs_arc = Arc::new(tx_jobs);

    spawn_proof_workers::<PublicState, PrivateState, GameAction>(
        num_workers,
        rx_jobs,
        prove_queue_arc.clone(),
    );

    let execute_client = client_arc.clone();
    let execute_elf = elf_arc.clone();
    let execute_session_manager = session_manager_arc.clone();
    let execute_route = warp::path!("execute")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(move |actions: serde_json::Value| {
            let client = execute_client.clone();
            let elf = execute_elf.clone();
            let session_manager = execute_session_manager.clone();

            async move {
                let session = {
                    let mut session_manager_guard = session_manager.lock().await;
                    let session_id =
                        match create_session_json(&mut session_manager_guard, reducer, actions)
                            .await
                        {
                            Ok(id) => id,
                            Err(err) => return Err(ServerError::bad_request(err.to_string())),
                        };
                    match session_manager_guard.get_session(&session_id).await {
                        Some(session) => session,
                        None => {
                            return Err(ServerError::bad_request("Failed to get session".into()))
                        }
                    }
                };

                handle_proof_request::<PublicState, PrivateState, GameAction>(
                    session,
                    client,
                    elf,
                    ProofType::Execute,
                    "".to_string(),
                )
                .await
                .map(|reply| warp::reply::json(&reply))
                .map_err(|e| ServerError::bad_request(e.to_string()))
            }
        });

    let prove_client = client_arc.clone();
    let prove_elf = elf_arc.clone();
    let prove_queue = prove_queue_arc.clone();
    let prove_session_manager = session_manager_arc.clone();
    let prove_tx_jobs = tx_jobs_arc.clone();
    let prove_route = warp::path!("prove" / String)
        .and(warp::post())
        .and(warp::body::json())
        .and_then(move |proof_type: String, actions: serde_json::Value| {
            let client = prove_client.clone();
            let elf = prove_elf.clone();
            let queue = prove_queue.clone();
            let session_manager = prove_session_manager.clone();
            let tx_jobs = prove_tx_jobs.clone();

            async move {
                let proof_type = match proof_type.as_str() {
                    "core" => ProofType::Core,
                    "compressed" => ProofType::Compressed,
                    "groth16" => ProofType::Groth16,
                    "plonk" => ProofType::Plonk,
                    _ => return Err(ServerError::bad_request("Invalid proof type".into())),
                };

                // Create a new task in the queue
                let task_id = queue.enqueue_task();
                let task_id_clone = task_id.clone();

                // Build a session instance
                let mut session_manager_guard = session_manager.lock().await;
                let session_id_result =
                    create_session_json(&mut session_manager_guard, reducer, actions).await;

                if let Err(err) = session_id_result {
                    queue.set_status(&task_id_clone, ProveStatus::Error(err.to_string()));
                    return Err(ServerError::bad_request(err.to_string()));
                }

                let session_id = session_id_result.unwrap();

                let session_option = session_manager_guard.get_session(&session_id).await;

                if session_option.is_none() {
                    queue.set_status(
                        &task_id_clone,
                        ProveStatus::Error("Failed to get session".into()),
                    );
                    return Err(ServerError::internal_server_error(
                        "Failed to get session".into(),
                    ));
                }

                // Start a new proof job
                tx_jobs
                    .send((
                        task_id_clone,
                        ProofRequest::new(
                            session_option.unwrap(),
                            proof_type,
                            client.clone(),
                            elf.clone(),
                        ),
                    ))
                    .map_err(|_| {
                        ServerError::internal_server_error("Error starting proof job".into())
                    })?;

                // Return the task ID to the client
                Ok(warp::reply::json(&json!({
                    "proof_id": task_id
                })))
            }
        });

    // Add a result route to query the result and status of the proof generation
    let prove_result_queue = prove_queue_arc.clone();
    let prove_result_route =
        warp::path!("proof" / String)
            .and(warp::get())
            .and_then(move |task_id: String| {
                let queue = prove_result_queue.clone();
                async move {
                    match queue.get_status(&task_id) {
                        Some(status) => match status {
                            ProveStatus::Queued => Ok(warp::reply::json(&json!({
                                "proof_id": task_id,
                                "status": "queued"
                            }))),
                            ProveStatus::InProgress => Ok(warp::reply::json(&json!({
                                "proof_id": task_id,
                                "status": "in_progress"
                            }))),
                            ProveStatus::Done(result) => Ok(warp::reply::json(&json!({
                                "proof_id": task_id,
                                "status": "done",
                                "proof": result
                            }))),
                            ProveStatus::Error(error) => Err(ServerError::bad_request(error)),
                        },
                        None => Err(ServerError::not_found("Proof not found".into())),
                    }
                }
            });

    execute_route
        .or(prove_route)
        .or(prove_result_route)
        .recover(handle_rejection)
}
