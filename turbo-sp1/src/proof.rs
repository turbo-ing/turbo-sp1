use lazy_static::lazy_static;
use sp1_sdk::SP1ProofWithPublicValues;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;

use alloy_sol_types::SolValue;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sp1_sdk::{
    EnvProver, ExecutionReport, HashableKey, SP1ProvingKey, SP1PublicValues, SP1VerifyingKey,
};
use tokio::sync::Mutex;
use turbo_program::traits::TurboActionSerialization;

use crate::session::TurboSession;

lazy_static! {
    static ref SETUP_CACHE: StdMutex<HashMap<Vec<u8>, Arc<(SP1ProvingKey, SP1VerifyingKey)>>> =
        StdMutex::new(HashMap::new());
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ProofType {
    Core,
    Compressed,
    Groth16,
    Plonk,
}

async fn setup_circuit(
    client: Arc<EnvProver>,
    elf: Arc<Vec<u8>>,
) -> Result<Arc<(SP1ProvingKey, SP1VerifyingKey)>, &'static str> {
    let mut cache = SETUP_CACHE.lock().map_err(|_| "Failed to lock cache")?;
    if let Some(arc) = cache.get(&elf[..]) {
        return Ok(arc.clone());
    } else {
        let (pk, vk) = client.setup(&elf);
        let arc = Arc::new((pk.clone(), vk.clone()));
        cache.insert(elf.to_vec(), arc.clone());
        Ok(arc)
    }
}

async fn execute_circuit<
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
) -> Result<(SP1PublicValues, ExecutionReport), &'static str> {
    // Setup the inputs
    let stdin = session.lock().await.sp1_stdin();

    // Try executing the circuit first
    client
        .execute(&elf, &stdin)
        .run()
        .map_err(|_| "Failed to execute circuit")
}

pub async fn handle_proof_execute<
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
) -> Result<serde_json::Value, &'static str> {
    let (output, report) = execute_circuit(session, client, elf).await?;

    let state: PublicState =
        PublicState::abi_decode(output.as_slice()).map_err(|_| "Failed to decode output state")?;
    Ok(json!({
        "cycle_count": report.total_instruction_count(),
        "state": state
    }))
}

pub async fn handle_proof_request<
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
) -> Result<serde_json::Value, &'static str> {
    // Setup the inputs
    let stdin = session.lock().await.sp1_stdin();

    // Try executing the circuit first
    let (_, report) = client
        .execute(&elf, &stdin)
        .run()
        .map_err(|_| "Failed to execute circuit")?;

    let setup_arc = setup_circuit(client.clone(), elf).await?;
    let pk = &setup_arc.0;
    let vk = &setup_arc.1;

    let proof = match proof_type {
        ProofType::Core => client
            .prove(pk, &stdin)
            .run()
            .expect("failed to generate proof"),
        ProofType::Compressed => client
            .prove(pk, &stdin)
            .compressed()
            .run()
            .expect("failed to generate proof"),
        ProofType::Groth16 => client
            .prove(pk, &stdin)
            .groth16()
            .run()
            .expect("failed to generate proof"),
        ProofType::Plonk => client
            .prove(pk, &stdin)
            .plonk()
            .run()
            .expect("failed to generate proof"),
    };

    let state: PublicState = PublicState::abi_decode(proof.public_values.as_slice()).unwrap();

    std::fs::create_dir_all("proofs").map_err(|_| "Failed to create proofs directory")?;
    proof
        .save(format!("proofs/{}.bin", proof_id))
        .map_err(|_| "Failed to save proof")?;

    Ok(match proof_type {
        ProofType::Core | ProofType::Compressed => json!({
            "vkey": vk.bytes32().to_string(),
            "public_values": format!("0x{}", hex::encode(proof.public_values.as_slice())),
            "state": state,
            "cycle_count": report.total_instruction_count()
        }),
        ProofType::Groth16 | ProofType::Plonk => json!({
            "vkey": vk.bytes32().to_string(),
            "public_values": format!("0x{}", hex::encode(proof.public_values.as_slice())),
            "proof": format!("0x{}", hex::encode(proof.bytes())),
            "state": state,
            "cycle_count": report.total_instruction_count()
        }),
    })
}
