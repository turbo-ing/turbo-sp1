use alloy_sol_types::SolValue;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::{json, Value};
use std::{convert::Infallible, sync::Arc};
use tokio::sync::{mpsc, Mutex};
use warp::Filter;

use sp1_sdk::ProverClient;
use turbo_program::{program::TurboReducer, traits::TurboActionSerialization};

use crate::proof::{handle_proof_execute, ProofType};
use crate::proof_worker::{spawn_proof_workers, ProofJob, ProofRequest};
use crate::prove_queue::{ProveQueue, ProveStatus};
use crate::session::TurboSession;
use crate::session_manager::SessionManager;
use crate::session_simple::{create_session_json, dispatch_actions};
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
    PrivateState: Default + Serialize + Send + Sync + 'static,
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

                handle_proof_execute::<PublicState, PrivateState, GameAction>(session, client, elf)
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

    // Add a WebSocket route for processing commands
    let ws_client = client_arc.clone();
    let ws_elf = elf_arc.clone();
    let ws_prove_queue = prove_queue_arc.clone();
    let ws_session_manager = session_manager_arc.clone();
    let ws_tx_jobs = tx_jobs_arc.clone();
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and_then(move |ws: warp::ws::Ws| {
            let session_manager = ws_session_manager.clone();
            let prove_queue = ws_prove_queue.clone();
            let client = ws_client.clone();
            let elf = ws_elf.clone();
            let tx_jobs = ws_tx_jobs.clone();
            
            async move {
                Ok::<_, warp::reject::Rejection>(ws.on_upgrade(move |websocket| async move {
                    let (mut tx, mut rx) = websocket.split();
                    let session_manager = session_manager.clone();
                    let mut active_session: Option<Arc<Mutex<TurboSession<PublicState, PrivateState, GameAction>>>> = None;
                    let mut active_proof_id: Option<String> = None;
                    let mut active_player_idx: Option<usize> = None;

                    if let Err(_) = tx.send(warp::ws::Message::text("{\"__state\":\"waiting\"}")).await {
                        return;
                    }

                    // Process messages one by one
                    while let Some(result) = rx.next().await {
                        match result {
                            Ok(msg) => {
                                if let Ok(text) = msg.to_str() {
                                    // Parse the JSON command
                                    match serde_json::from_str::<serde_json::Value>(text) {
                                        Ok(command) => {
                                            let response = if command.get("__syscall").is_some() {
                                                let syscall = command.get("__syscall").unwrap().as_str().unwrap();
                                                let mut response: Option<Value> = None;

                                                if syscall == "join_session" {
                                                    let session_id_option = command.get("session_id");
                                                    let session_id = match session_id_option {
                                                        Some(session_id) => session_id.as_str().unwrap(),
                                                        None => &session_manager.lock().await.create_session(reducer).await
                                                    };
                                                    
                                                    let session_option = session_manager.lock().await.get_session(session_id).await;
                                                            
                                                    if session_option.is_some() {
                                                        let session = session_option.unwrap();
                                                        let player_idx = session.lock().await.join_random();

                                                        active_session = Some(session);
                                                        active_player_idx = Some(player_idx);

                                                        response = Some(json!({
                                                            "__state": "ready",
                                                            "__session_id": session_id.to_string(),
                                                        }));
                                                    } else {
                                                        response = Some(json!({
                                                            "error": "Failed to create session"
                                                        }));
                                                    }
                                                } else if syscall == "proof" {
                                                    let proof_type = command.get("proof_type").unwrap().as_str().unwrap();
                                                    let proof_type = match proof_type {
                                                        "core" => ProofType::Core,
                                                        "compressed" => ProofType::Compressed,
                                                        "groth16" => ProofType::Groth16,
                                                        "plonk" => ProofType::Plonk,
                                                        _ => panic!("Invalid proof type"),
                                                    };
                                    
                                                    // Create a new task in the queue
                                                    let proof_id = prove_queue.enqueue_task();
                                                    let proof_id_clone = proof_id.clone();

                                                    // Set active proof id
                                                    active_proof_id = Some(proof_id.clone());

                                                    // Start a new proof job
                                                    tx_jobs
                                                        .send((
                                                            proof_id_clone,
                                                            ProofRequest::new(
                                                                active_session.clone().unwrap().clone(),
                                                                proof_type,
                                                                client.clone(),
                                                                elf.clone(),
                                                            ),
                                                        )).unwrap();

                                                    response = Some(json!({
                                                        "proof_id": proof_id.clone(),
                                                    }));
                                                } else if syscall == "proof_status" {
                                                    let proof_id_option = command.get("proof_id");
                                                    let proof_id: String = match proof_id_option {
                                                        Some(proof_id) => proof_id.as_str().unwrap().to_string(),
                                                        None => active_proof_id.clone().unwrap(),
                                                    };
                                                    
                                                    let status = prove_queue.get_status(&proof_id);
                                                    let status = match status {
                                                        Some(status) => status,
                                                        None => ProveStatus::Error("Proof not found".into()),
                                                    };
                                                    
                                                    response = match status {
                                                        ProveStatus::Queued => Some(json!({
                                                            "proof_id": proof_id,
                                                            "status": "queued"
                                                        })),
                                                        ProveStatus::InProgress => Some(json!({
                                                            "proof_id": proof_id,
                                                            "status": "in_progress"
                                                        })),
                                                        ProveStatus::Done(result) => Some(json!({
                                                            "proof_id": proof_id,
                                                            "status": "done",
                                                            "proof": result
                                                        })),
                                                        ProveStatus::Error(error) => Some(json!({
                                                            "proof_id": proof_id,
                                                            "status": "error",
                                                            "error": error
                                                        })),
                                                    };
                                                }

                                                // Handle syscall
                                                serde_json::to_string(&response.unwrap_or(json!({
                                                    "error": "Syscalls not yet implemented"
                                                }))).unwrap_or_else(|_| String::from("{\"error\":\"Failed to serialize response\"}"))
                                            } else {
                                                let player_idx = active_player_idx.unwrap();
                                                let result = dispatch_actions(active_session.clone().unwrap(), command, player_idx).await;

                                                if let Err(e) = result {
                                                    serde_json::to_string(&json!({
                                                        "error": e
                                                    })).unwrap_or_else(|_| String::from("{\"error\":\"Failed to serialize response\"}"))
                                                } else {
                                                    let result_json = active_session.clone().unwrap().lock().await.serialize_json(player_idx).unwrap();
                                                    serde_json::to_string(&result_json).unwrap_or_else(|_| String::from("{\"error\":\"Failed to serialize response\"}"))
                                                }
                                            };

                                            if let Err(_) = tx.send(warp::ws::Message::text(response)).await {
                                                break;
                                            }
                                        },
                                        Err(_) => {
                                            if let Err(_) = tx.send(warp::ws::Message::text("{\"error\":\"Invalid JSON\"}")).await {
                                                break;
                                            }
                                        }
                                    }
                                }
                            },
                            Err(_) => break
                        }
                    }
                }))
            }
        });

    execute_route
        .or(prove_route)
        .or(prove_result_route)
        .or(ws_route)
        .recover(handle_rejection)
}
