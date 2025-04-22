use alloy_sol_types::sol;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use turbo_program::{context::TurboActionContext, traits::TurboActionSerialization};

sol! {
    #[derive(Serialize, Deserialize, Debug, Default)]
    struct GamePublicState {
        uint8[4][4] board;
        uint32 num;
    }

    #[derive(Serialize, Deserialize, Debug, Default)]
    struct GamePrivateState {
        uint32 moves;
    }
}

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
            let action_type = action["type"].as_str().ok_or("Missing type field")?;
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

/// Slide non-zero tiles to the left and merge equal adjacent tiles.
fn slide_and_merge_line(line: [u8; 4]) -> [u8; 4] {
    let mut out = [0; 4];
    // Collect non-zero tiles
    let temp = line
        .iter()
        .cloned()
        .filter(|&x| x != 0)
        .collect::<Vec<u8>>();
    let mut idx = 0;
    let mut i = 0;
    // Merge tiles
    while i < temp.len() {
        if i + 1 < temp.len() && temp[i] == temp[i + 1] {
            out[idx] = temp[i] * 2;
            i += 2;
        } else {
            out[idx] = temp[i];
            i += 1;
        }
        idx += 1;
    }
    out
}

/// Transpose a 4×4 board (rows <-> columns).
fn transpose(board: &[[u8; 4]; 4]) -> [[u8; 4]; 4] {
    let mut t = [[0; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            t[i][j] = board[j][i];
        }
    }
    t
}

/// Move the board in one of four directions:
/// 0 = Up, 1 = Down, 2 = Left, 3 = Right.
pub fn move_board(board: &[[u8; 4]; 4], direction: u8) -> [[u8; 4]; 4] {
    let mut result = [[0; 4]; 4];

    match direction {
        // Up: transpose, slide left, transpose back
        0 => {
            let tb = transpose(board);
            let mut moved = [[0; 4]; 4];
            for i in 0..4 {
                moved[i] = slide_and_merge_line(tb[i]);
            }
            for i in 0..4 {
                for j in 0..4 {
                    result[j][i] = moved[i][j];
                }
            }
        }
        // Down: transpose, slide right, transpose back
        1 => {
            let tb = transpose(board);
            let mut moved = [[0; 4]; 4];
            for i in 0..4 {
                let mut row = tb[i];
                row.reverse();
                let mut merged = slide_and_merge_line(row);
                merged.reverse();
                moved[i] = merged;
            }
            for i in 0..4 {
                for j in 0..4 {
                    result[j][i] = moved[i][j];
                }
            }
        }
        // Left: slide each row
        2 => {
            for i in 0..4 {
                result[i] = slide_and_merge_line(board[i]);
            }
        }
        // Right: reverse each row, slide left, then reverse back
        3 => {
            for i in 0..4 {
                let mut row = board[i];
                row.reverse();
                let mut merged = slide_and_merge_line(row);
                merged.reverse();
                result[i] = merged;
            }
        }
        // Invalid direction: return original
        _ => return *board,
    }

    result
}

pub fn reducer(
    public_state: &mut GamePublicState,
    private_state: &mut GamePrivateState,
    action: &GameAction,
    context: &mut TurboActionContext,
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
        GameAction::MoveAndRandomTileAction(direction) => {
            public_state.board = move_board(&public_state.board, *direction);
            private_state.moves += 1;
            let mut empty_positions = Vec::new();
            for row in 0..4 {
                for col in 0..4 {
                    if public_state.board[row][col] == 0 {
                        empty_positions.push((row, col));
                    }
                }
            }

            let rand = context.rand_u32();
            public_state.num += rand % 16;

            if !empty_positions.is_empty() {
                let (r, c) = empty_positions[rand as usize % empty_positions.len()];
                public_state.board[r][c] = 2;
            }
        }
    }
}
