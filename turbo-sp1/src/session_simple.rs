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
    let session_id = session_manager.create_session(reducer).await;
    let session = session_manager
        .get_session(&session_id)
        .await
        .ok_or("Failed to create session")?;

    let mut session_guard = session.lock().await;

    let remaining_actions_vec = match actions {
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

    let mut remaining_actions = &remaining_actions_vec[..];

    while !remaining_actions.is_empty() {
        let player_idx = remaining_actions[0] as usize;
        let action_type = remaining_actions[1];
        let mut action_length: usize = action_type as usize;

        if player_idx > 100 {
            Err("Max 100 players")?;
        }

        while player_idx >= session_guard.player_count() {
            session_guard.join_random();
        }

        if action_type == 0x80 {
            action_length = remaining_actions[2] as usize;
        } else if action_type == 0x81 {
            action_length =
                ((remaining_actions[2] as usize) << 8) | (remaining_actions[3] as usize);
        } else if action_type > 0x81 {
            Err("Invalid action type")?;
        }

        let start_idx = match action_type {
            0x80 => 3,
            0x81 => 4,
            _ => 2,
        };
        if remaining_actions.len() < start_idx + action_length {
            Err("Action bytes too short for specified length")?;
        }

        let action_bytes = &remaining_actions[0..start_idx + action_length];
        session_guard.dispatch(action_bytes)?;

        remaining_actions = &remaining_actions[start_idx + action_length..];
    }

    Ok(session_id)
}
