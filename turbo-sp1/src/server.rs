use alloy_sol_types::SolValue;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use warp::Filter;

use sp1_sdk::{EnvProver, HashableKey, ProverClient, SP1Stdin};
use turbo_sp1_program::{
    program::TurboReducer,
    traits::{TurboActionSerialization, TurboInitState},
};

use crate::warp::rejection::{handle_rejection, ServerError};

/// A fixture that can be used to test the verification of SP1 zkVM proofs inside Solidity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SP1ProofFixture {
    vkey: String,
    public_values: String,
    proof: String,
}

#[derive(Debug)]
enum ProofType {
    Execute,
    Core,
    Compressed,
    Groth16,
    Plonk,
}

async fn handle_proof_request<
    GameAction: TurboActionSerialization,
    PublicState: SolValue
        + Serialize
        + From<<<PublicState as SolValue>::SolType as alloy_sol_types::SolType>::RustType>,
>(
    actions: serde_json::Value,
    client: Arc<EnvProver>,
    elf: Arc<Vec<u8>>,
    proof_type: ProofType,
) -> Result<impl warp::Reply, warp::Rejection> {
    // Setup the inputs
    let mut stdin = SP1Stdin::new();

    let actions_bytes = match actions {
        serde_json::Value::Array(_) => {
            // If it's JSON array, serialize using GameAction::serialize_json
            GameAction::serialize_json(&actions.to_string())
                .map_err(|_| ServerError::bad_request("Failed to serialize actions".into()))
        }
        serde_json::Value::String(hex_str) => {
            // If it's a hex string, parse as Vec<u8>
            hex::decode(hex_str.trim_start_matches("0x"))
                .map_err(|_| ServerError::bad_request("Failed to decode hex string".into()))
        }
        _ => return Err(ServerError::bad_request("Invalid input format".into())),
    }?;

    stdin.write(&actions_bytes);

    match proof_type {
        ProofType::Execute => {
            let (output, report) = client.execute(&elf, &stdin).run().unwrap();
            let state: PublicState = PublicState::abi_decode(output.as_slice()).unwrap();
            Ok(warp::reply::json(&json!({
                "cycle_count": report.total_instruction_count(),
                "state": state
            })))
        }
        ProofType::Core => {
            let (pk, vk) = client.setup(&elf);
            let proof = client
                .prove(&pk, &stdin)
                .run()
                .expect("failed to generate proof");
            // Create and save the fixture
            let fixture = SP1ProofFixture {
                vkey: vk.bytes32().to_string(),
                public_values: format!("0x{}", hex::encode(proof.public_values.as_slice())),
                proof: format!("0x{}", hex::encode(proof.bytes())),
            };
            let state: PublicState =
                PublicState::abi_decode(proof.public_values.as_slice()).unwrap();
            Ok(warp::reply::json(&json!({
                "proof": fixture,
                "state": state
            })))
        }
        _ => Err(ServerError::bad_request("Invalid proof type".into())),
    }
}

pub fn turbo_sp1_routes<PublicState, PrivateState, GameAction>(
    elf: &[u8],
    reducer: TurboReducer<PublicState, PrivateState, GameAction>,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone
where
    PublicState: TurboInitState
        + SolValue
        + Serialize
        + From<<<PublicState as SolValue>::SolType as alloy_sol_types::SolType>::RustType>,
    PrivateState: TurboInitState,
    GameAction: TurboActionSerialization,
{
    let client = Arc::new(ProverClient::from_env());
    let elf = Arc::new(elf.to_vec());

    let execute_client = client.clone();
    let execute_elf = elf.clone();
    let execute_route = warp::path!("execute")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(move |actions: serde_json::Value| {
            let client = execute_client.clone();
            let elf = execute_elf.clone();
            async move {
                handle_proof_request::<GameAction, PublicState>(
                    actions,
                    client,
                    elf,
                    ProofType::Execute,
                )
                .await
            }
        });

    let prove_client = client.clone();
    let prove_elf = elf.clone();
    let prove_route = warp::path!("prove" / String)
        .and(warp::post())
        .and(warp::body::json())
        .and_then(move |proof_type: String, actions: serde_json::Value| {
            let client = prove_client.clone();
            let elf = prove_elf.clone();
            async move {
                let proof_type = match proof_type.as_str() {
                    "core" => ProofType::Core,
                    "compressed" => ProofType::Compressed,
                    "groth16" => ProofType::Groth16,
                    "plonk" => ProofType::Plonk,
                    _ => return Err(ServerError::bad_request("Invalid proof type".into())),
                };
                handle_proof_request::<GameAction, PublicState>(actions, client, elf, proof_type)
                    .await
            }
        });

    execute_route.or(prove_route)
}
