use serde_json::Value;
use turbo_program::traits::TurboActionSerialization;

#[derive(Debug)]
pub enum GameAction {
    MoveAction(u8),
    NewTileAction(u8, u8),
    MoveAndRandomTileAction(u8),
}

impl TurboActionSerialization for GameAction {
    fn deserialize(action: &[u8]) -> Result<Self, &'static str> {
        let action_type = action[0];
        let action_data = &action[1..];

        match action_type {
            0 => Ok(GameAction::MoveAction(action_data[0])),
            1 => Ok(GameAction::NewTileAction(action_data[0], action_data[1])),
            2 => Ok(GameAction::MoveAndRandomTileAction(action_data[0])),
            _ => Err("Invalid action type"),
        }
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
                "MoveAction" => {
                    if data.len() != 1 {
                        return Err("Invalid data length for MoveAction");
                    }
                    let direction = data[0].as_u64().ok_or("Invalid direction")? as u8;
                    result.push(2); // Length (1 for type + 1 for direction)
                    result.push(0); // Action type 0 for MoveAction
                    result.push(direction);
                }
                "NewTileAction" => {
                    if data.len() != 2 {
                        return Err("Invalid data length for NewTileAction");
                    }
                    let pos = data[0].as_u64().ok_or("Invalid position")? as u8;
                    let value = data[1].as_u64().ok_or("Invalid value")? as u8;
                    result.push(3); // Length (1 for type + 2 for pos and value)
                    result.push(1); // Action type 1 for NewTileAction
                    result.push(pos);
                    result.push(value);
                }
                "MoveAndRandomTileAction" => {
                    if data.len() != 1 {
                        return Err("Invalid data length for MoveAndRandomTileAction");
                    }
                    let direction = data[0].as_u64().ok_or("Invalid direction")? as u8;
                    result.push(2); // Length (1 for type + 1 for direction)
                    result.push(2); // Action type 2 for MoveAndRandomTileAction
                    result.push(direction);
                }
                _ => return Err("Invalid action type"),
            }
        }

        Ok(result)
    }
}
