use turbo_program::{program::TurboReducer, traits::TurboActionSerialization};

use crate::session_manager::SessionManager;

pub async fn create_session_json<PublicState, PrivateState, GameAction>(
    session_manager: &mut SessionManager<PublicState, PrivateState, GameAction>,
    reducer: TurboReducer<PublicState, PrivateState, GameAction>,
    actions: serde_json::Value,
) -> Result<String, &'static str>
where
    PublicState: Default + Send + Sync,
    PrivateState: Default + Send + Sync,
    GameAction: TurboActionSerialization + Send + Sync,
{
    let session = session_manager
        .create_session(reducer)
        .await
        .ok_or("Failed to create session")?;

    let mut session_guard = session.lock().await;

    let actions_bytes = match actions {
        serde_json::Value::Array(_) => {
            let mut result: Vec<u8> = Vec::new();
            for action in actions.as_array().unwrap() {
                let action_bytes = GameAction::serialize_json(&action.to_string())
                    .map_err(|_| "Failed to serialize action")?;
                result.extend(action_bytes);
            }
            Ok(result)
        }
        serde_json::Value::String(hex_str) => {
            // If it's a hex string, parse as Vec<u8>
            hex::decode(hex_str.trim_start_matches("0x")).map_err(|_| "Failed to decode hex string")
        }
        _ => return Err("Invalid input format"),
    }?;

    session_guard.dispatch(actions_bytes)?;

    Ok(session_guard.id())
}
