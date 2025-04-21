use std::sync::Arc;

use alloy_sol_types::SolValue;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sp1_sdk::{EnvProver, HashableKey};
use tokio::sync::Mutex;
use turbo_sp1_program::traits::TurboActionSerialization;

use crate::session::TurboSession;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ProofType {
    Execute,
    Core,
    Compressed,
    Groth16,
    Plonk,
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
    let (output, report) = client
        .execute(&elf, &stdin)
        .run()
        .map_err(|_| "Failed to execute circuit")?;

    match proof_type {
        ProofType::Execute => {
            let state: PublicState = PublicState::abi_decode(output.as_slice())
                .map_err(|_| "Failed to decode output state")?;
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
                .map_err(|_| "Failed to save proof")?;

            Ok(json!({
                "vkey": vk.bytes32().to_string(),
                "public_values": format!("0x{}", hex::encode(proof.public_values.as_slice())),
                "state": state,
                "cycle_count": report.total_instruction_count()
            }))
        }
        _ => Err("Invalid proof type"),
    }
}
