use alloy_sol_types::sol;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use turbo_sp1_program::traits::{TurboActionSerialization, TurboInitState};

sol! {
    #[derive(Serialize, Deserialize, Debug)]
    struct GamePublicState {
        uint8[4][4] board;
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct GamePrivateState {
        uint64 moves;
    }
}

impl TurboInitState for GamePublicState {
    fn init_state() -> Self {
        GamePublicState { board: [[0; 4]; 4] }
    }
}

impl TurboInitState for GamePrivateState {
    fn init_state() -> Self {
        GamePrivateState { moves: 0 }
    }
}

pub enum GameAction {
    MoveAction(u8),
    NewTileAction(u8, u8),
}

impl TurboActionSerialization for GameAction {
    fn deserialize(action: &[u8]) -> Result<Self, &'static str> {
        let action_type = action[0];
        let action_data = &action[1..];

        match action_type {
            0 => Ok(GameAction::MoveAction(action_data[0])),
            1 => Ok(GameAction::NewTileAction(action_data[0], action_data[1])),
            _ => Err("Invalid action type"),
        }
    }

    fn serialize_json(json_str: &str) -> Result<Vec<u8>, &'static str> {
        let actions: Vec<Value> = serde_json::from_str(json_str).map_err(|_| "Invalid JSON")?;
        let mut result = Vec::new();

        for action in actions {
            if let Some(action_u8) = action.as_u64().map(|n| n as u8) {
                result.push(action_u8);
            } else {
                let action_type = action["type"].as_str().ok_or("Missing type field")?;
                let data = action["data"].as_array().ok_or("Missing data field")?;

                match action_type {
                    "MoveAction" => {
                        if data.len() != 1 {
                            return Err("Invalid data length for MoveAction");
                        }
                        let direction = data[0].as_u64().ok_or("Invalid direction")? as u8;
                        result.push(0x01); // Length type byte
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
                        result.push(0x01); // Length type byte
                        result.push(3); // Length (1 for type + 2 for pos and value)
                        result.push(1); // Action type 1 for NewTileAction
                        result.push(pos);
                        result.push(value);
                    }
                    _ => return Err("Invalid action type"),
                }
            }
        }

        Ok(result)
    }
}

pub fn move_board(board: &[[u8; 4]; 4], direction: u8) -> [[u8; 4]; 4] {
    let mut grid = *board;
    let size = 4;

    match direction {
        0 => {
            // Up
            for col in 0..size {
                let mut write_idx = 0;
                let mut last_merged = 0;

                for read_idx in 0..size {
                    if grid[read_idx][col] == 0 {
                        continue;
                    }

                    if write_idx > 0
                        && grid[read_idx][col] == last_merged
                        && grid[write_idx - 1][col] == last_merged
                    {
                        grid[write_idx - 1][col] *= 2;
                        last_merged = 0;
                    } else {
                        grid[write_idx][col] = grid[read_idx][col];
                        last_merged = grid[read_idx][col];
                        write_idx += 1;
                    }
                }

                while write_idx < size {
                    grid[write_idx][col] = 0;
                    write_idx += 1;
                }
            }
        }
        1 => {
            // Down
            for col in 0..size {
                let mut write_idx = size - 1;
                let mut last_merged = 0;

                for read_idx in (0..size).rev() {
                    if grid[read_idx][col] == 0 {
                        continue;
                    }

                    if write_idx < size - 1
                        && grid[read_idx][col] == last_merged
                        && grid[write_idx + 1][col] == last_merged
                    {
                        grid[write_idx + 1][col] *= 2;
                        last_merged = 0;
                    } else {
                        grid[write_idx][col] = grid[read_idx][col];
                        last_merged = grid[read_idx][col];
                        write_idx = write_idx.wrapping_sub(1);
                    }
                }

                let mut clear_idx = 0;
                while clear_idx <= write_idx {
                    grid[clear_idx][col] = 0;
                    clear_idx += 1;
                }
            }
        }
        2 => {
            // Left
            for row in 0..size {
                let mut write_idx = 0;
                let mut last_merged = 0;

                for read_idx in 0..size {
                    if grid[row][read_idx] == 0 {
                        continue;
                    }

                    if write_idx > 0
                        && grid[row][read_idx] == last_merged
                        && grid[row][write_idx - 1] == last_merged
                    {
                        grid[row][write_idx - 1] *= 2;
                        last_merged = 0;
                    } else {
                        grid[row][write_idx] = grid[row][read_idx];
                        last_merged = grid[row][read_idx];
                        write_idx += 1;
                    }
                }

                while write_idx < size {
                    grid[row][write_idx] = 0;
                    write_idx += 1;
                }
            }
        }
        3 => {
            // Right
            for row in 0..size {
                let mut write_idx = size - 1;
                let mut last_merged = 0;

                for read_idx in (0..size).rev() {
                    if grid[row][read_idx] == 0 {
                        continue;
                    }

                    if write_idx < size - 1
                        && grid[row][read_idx] == last_merged
                        && grid[row][write_idx + 1] == last_merged
                    {
                        grid[row][write_idx + 1] *= 2;
                        last_merged = 0;
                    } else {
                        grid[row][write_idx] = grid[row][read_idx];
                        last_merged = grid[row][read_idx];
                        write_idx = write_idx.wrapping_sub(1);
                    }
                }

                let mut clear_idx = 0;
                while clear_idx <= write_idx {
                    grid[row][clear_idx] = 0;
                    clear_idx += 1;
                }
            }
        }
        _ => return *board,
    }

    grid
}

pub fn reducer(
    public_state: &mut GamePublicState,
    private_state: &mut GamePrivateState,
    action: &GameAction,
) {
    match action {
        GameAction::MoveAction(direction) => {
            public_state.board = move_board(&public_state.board, *direction);
            private_state.moves += 1;
        }
        GameAction::NewTileAction(r, c) => {
            if public_state.board[*r as usize][*c as usize] == 0 {
                public_state.board[*r as usize][*c as usize] = 2;
            } else {
                panic!("Cannot place new tile in non-empty position");
            }
        }
    }
}
