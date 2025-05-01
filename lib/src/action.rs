use serde_json::Value;
use turbo_program::traits::TurboActionSerialization;

#[derive(Debug)]
pub enum GameAction {
    MoveAndRandomTileAction(u8),
}

impl TurboActionSerialization for GameAction {
    fn deserialize(action: &[u8]) -> Result<(Self, &[u8]), &'static str> {
        let action_type = action[0];
        Ok((
            GameAction::MoveAndRandomTileAction(action_type),
            &action[1..],
        ))
    }

    fn serialize_json(json_str: &str) -> Result<Vec<u8>, &'static str> {
        let action: Value = serde_json::from_str(json_str).map_err(|_| "Invalid JSON")?;
        let mut result = Vec::new();

        if let Some(action_u8) = action.as_u64().map(|n| n as u8) {
            result.push(action_u8);
        } else {
            let action_type = action["action"].as_str().ok_or("Missing action field")?;
            let data = action["data"].as_array().ok_or("Missing data field")?;

            match action_type {
                "MoveAndRandomTileAction" => {
                    if data.len() != 1 {
                        return Err("Invalid data length for MoveAndRandomTileAction");
                    }
                    let direction = data[0].as_u64().ok_or("Invalid direction")? as u8;
                    result.push(direction);
                }
                _ => return Err("Invalid action type"),
            }
        }

        Ok(result)
    }
}
