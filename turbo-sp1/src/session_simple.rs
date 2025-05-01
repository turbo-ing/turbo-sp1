use std::sync::Arc;

use serde::Serialize;
use tokio::sync::Mutex;
use turbo_program::{program::TurboReducer, traits::TurboActionSerialization};

use crate::{session::TurboSession, session_manager::SessionManager};

pub async fn dispatch_actions<PublicState, PrivateState, GameAction>(
    session: Arc<Mutex<TurboSession<PublicState, PrivateState, GameAction>>>,
    actions: serde_json::Value,
    player_idx: usize,
) -> Result<(), &'static str>
where
    PublicState: Serialize + Default + Send + Sync,
    PrivateState: Default + Send + Sync,
    GameAction: TurboActionSerialization + Send + Sync,
{
    let mut session_guard = session.lock().await;

    let remaining_actions_vec = match actions {
        serde_json::Value::Array(_) => {
            let mut result: Vec<u8> = Vec::new();
            for action in actions.as_array().unwrap() {
                let action_bytes = GameAction::serialize_json(&action.to_string())
                    .map_err(|_| "Failed to serialize action")?;
                if action.is_object() {
                    let with_player = vec![player_idx as u8];
                    result.extend(with_player);
                }
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

        if player_idx > 100 {
            Err("Max 100 players")?;
        }

        while player_idx >= session_guard.player_count() {
            session_guard.join_random();
        }

        let (_action, next_actions) =
            GameAction::deserialize(&remaining_actions[1..]).expect("Failed to deserialize action");

        let action_bytes = &remaining_actions[0..remaining_actions.len() - next_actions.len()];
        session_guard.dispatch(action_bytes)?;

        remaining_actions = next_actions;
    }

    Ok(())
}

pub async fn create_session_json<PublicState, PrivateState, GameAction>(
    session_manager: &mut SessionManager<PublicState, PrivateState, GameAction>,
    reducer: TurboReducer<PublicState, PrivateState, GameAction>,
    actions: serde_json::Value,
) -> Result<String, &'static str>
where
    PublicState: Serialize + Default + Send + Sync,
    PrivateState: Default + Send + Sync,
    GameAction: TurboActionSerialization + Send + Sync,
{
    let session_id = session_manager.create_session(reducer).await;
    let session = session_manager
        .get_session(&session_id)
        .await
        .ok_or("Failed to create session")?;

    dispatch_actions(session, actions, 0).await?;

    Ok(session_id)
}
