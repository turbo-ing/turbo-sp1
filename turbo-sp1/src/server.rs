use alloy_sol_types::SolValue;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{convert::Infallible, sync::Arc, sync::Mutex};
use warp::Filter;

use sp1_sdk::{EnvProver, HashableKey, ProverClient, SP1Stdin};
use turbo_sp1_program::{program::TurboReducer, traits::TurboActionSerialization};

use crate::prove_queue::{ProveQueue, ProveStatus};
use crate::session::TurboSession;
use crate::session_manager::SessionManager;
use crate::session_simple::create_session_json;
use crate::warp::rejection::{handle_rejection, ServerError};

#[derive(Debug)]
enum ProofType {
    Execute,
    Core,
    Compressed,
    Groth16,
    Plonk,
}

async fn handle_proof_request<
    PublicState: Default
        + SolValue
        + Serialize
        + From<<<PublicState as SolValue>::SolType as alloy_sol_types::SolType>::RustType>
        + Send
        + Sync,
    PrivateState: Default + Send + Sync,
    GameAction: TurboActionSerialization + Send + Sync,
>(
    session: Arc<Mutex<TurboSession<PublicState, PrivateState, GameAction>>>,
    client: Arc<EnvProver>,
    elf: Arc<Vec<u8>>,
    proof_type: ProofType,
    proof_id: String,
) -> Result<serde_json::Value, warp::Rejection> {
    // Setup the inputs
    let stdin = session.lock().unwrap().sp1_stdin();

    // Try executing the circuit first
    let (output, report) = client
        .execute(&elf, &stdin)
        .run()
        .map_err(|_| ServerError::bad_request("Failed to execute circuit".into()))?;

    match proof_type {
        ProofType::Execute => {
            let state: PublicState = PublicState::abi_decode(output.as_slice())
                .map_err(|_| ServerError::bad_request("Failed to decode output state".into()))?;
            Ok(json!({
                "cycle_count": report.total_instruction_count(),
                "state": state
            }))
        }
        ProofType::Core => {
            let (pk, vk) = client.setup(&elf);
            let proof = client
                .prove(&pk, &stdin)
                .run()
                .expect("failed to generate proof");
            let state: PublicState =
                PublicState::abi_decode(proof.public_values.as_slice()).unwrap();

            proof
                .save(format!("proofs/{}.bin", proof_id))
                .map_err(|_| ServerError::internal_server_error("Failed to save proof".into()))?;

            Ok(json!({
                "vkey": vk.bytes32().to_string(),
                "public_values": format!("0x{}", hex::encode(proof.public_values.as_slice())),
                "state": state,
                "cycle_count": report.total_instruction_count()
            }))
        }
        _ => Err(ServerError::bad_request("Invalid proof type".into())),
    }
}

pub fn turbo_sp1_routes<PublicState, PrivateState, GameAction>(
    elf: &[u8],
    reducer: TurboReducer<PublicState, PrivateState, GameAction>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Infallible> + Clone
where
    PublicState: Default
        + SolValue
        + Serialize
        + From<<<PublicState as SolValue>::SolType as alloy_sol_types::SolType>::RustType>
        + Send
        + Sync,
    PrivateState: Default + Send + Sync,
    GameAction: TurboActionSerialization + Send + Sync,
{
    let client_arc = Arc::new(ProverClient::from_env());
    let elf_arc = Arc::new(elf.to_vec());
    let prove_queue_arc = Arc::new(ProveQueue::new());
    let session_manager_arc = Arc::new(Mutex::new(SessionManager::<
        PublicState,
        PrivateState,
        GameAction,
    >::new()));

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
                let mut session_manager_guard = session_manager.lock().unwrap();
                let session_id = create_session_json(&mut session_manager_guard, reducer, actions)
                    .map_err(|err| ServerError::bad_request(err.to_string()))?;
                let session = session_manager_guard
                    .get_session(&session_id)
                    .ok_or(ServerError::bad_request("Failed to get session".into()))?;
                handle_proof_request::<PublicState, PrivateState, GameAction>(
                    session,
                    client,
                    elf,
                    ProofType::Execute,
                    "".to_string(),
                )
                .await
                .map(|reply| warp::reply::json(&reply))
            }
        });

    let prove_client = client_arc.clone();
    let prove_elf = elf_arc.clone();
    let prove_queue = prove_queue_arc.clone();
    let prove_session_manager = session_manager_arc.clone();
    let prove_route = warp::path!("prove" / String)
        .and(warp::post())
        .and(warp::body::json())
        .and_then(move |proof_type: String, actions: serde_json::Value| {
            let client = prove_client.clone();
            let elf = prove_elf.clone();
            let queue = prove_queue.clone();
            let queue2 = prove_queue.clone();
            let session_manager = prove_session_manager.clone();

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

                // Spawn a new task to handle the proof generation
                let handle = tokio::spawn({
                    let actions = actions.clone();
                    let elf = elf.clone();
                    let client = client.clone();

                    async move {
                        let mut session_manager_guard = session_manager.lock().unwrap();
                        let session_id_result =
                            create_session_json(&mut session_manager_guard, reducer, actions);

                        if let Err(err) = session_id_result {
                            queue.set_status(&task_id_clone, ProveStatus::Error(err.to_string()));
                            queue.cleanup_handle(&task_id_clone);
                            return;
                        }

                        let session_id = session_id_result.unwrap();

                        let session_option = session_manager_guard.get_session(&session_id);

                        if session_option.is_none() {
                            queue.set_status(
                                &task_id_clone,
                                ProveStatus::Error("Failed to get session".into()),
                            );
                            queue.cleanup_handle(&task_id_clone);
                            return;
                        }

                        let session = session_option.unwrap();

                        let result = match handle_proof_request::<
                            PublicState,
                            PrivateState,
                            GameAction,
                        >(
                            session, client, elf, proof_type, task_id_clone.clone()
                        )
                        .await
                        {
                            Ok(reply) => ProveStatus::Done(reply),
                            Err(e) => {
                                let error_msg = if let Some(server_error) = e.find::<ServerError>()
                                {
                                    server_error.message()
                                } else {
                                    "Internal Server Error".to_string()
                                };
                                ProveStatus::Error(error_msg)
                            }
                        };

                        queue.set_status(&task_id_clone, result);
                        queue.cleanup_handle(&task_id_clone);
                    }
                });

                queue2.store_handle(&task_id, handle);

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
