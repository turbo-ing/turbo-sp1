use alloy_sol_types::sol;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use turbo_program::{context::TurboActionContext, traits::TurboActionSerialization};

sol! {
    #[derive(Serialize, Deserialize, Debug, Default)]
    struct GamePublicState {
        uint32[4][4] board;
    }

    #[derive(Serialize, Deserialize, Debug, Default)]
    struct GamePrivateState {
        uint32 moves;
    }
}

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

/// Slide non-zero tiles to the left and merge equal adjacent tiles.
fn slide_and_merge_line(line: [u32; 4]) -> [u32; 4] {
    let mut out = [0; 4];
    // Collect non-zero tiles
    let temp = line
        .iter()
        .cloned()
        .filter(|&x| x != 0)
        .collect::<Vec<u32>>();
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
fn transpose(board: &[[u32; 4]; 4]) -> [[u32; 4]; 4] {
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
pub fn move_board(board: &[[u32; 4]; 4], direction: u8) -> [[u32; 4]; 4] {
    let mut result = [[0; 4]; 4];

    // (Optimistic in most cases) First check if move is possible to avoid unnecessary operations
    // let can_move = match direction {
    //     0 => {
    //         // Up
    //         let tb = transpose(board);
    //         (0..4).any(|i| can_merge_line(&tb[i]))
    //     }
    //     1 => {
    //         // Down
    //         let tb = transpose(board);
    //         (0..4).any(|i| {
    //             let mut row = tb[i];
    //             row.reverse();
    //             can_merge_line(&row)
    //         })
    //     }
    //     2 => {
    //         // Left
    //         (0..4).any(|i| can_merge_line(&board[i]))
    //     }
    //     3 => {
    //         // Right
    //         (0..4).any(|i| {
    //             let mut row = board[i];
    //             row.reverse();
    //             can_merge_line(&row)
    //         })
    //     }
    //     _ => false,
    // };

    // if !can_move {
    //     return *board;
    // }

    match direction {
        // Up: transpose, slide left, transpose back
        0 => {
            let tb = transpose(board);
            let mut moved = [[0; 4]; 4];
            for i in 0..4 {
                moved[i] = slide_and_merge_line(tb[i]);
            }
            result = transpose(&moved);
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
            result = transpose(&moved);
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

/// Helper function to check if a line can be merged/moved
fn can_merge_line(line: &[u32; 4]) -> bool {
    // Check for possible merges
    for i in 0..3 {
        if line[i] != 0 && line[i] == line[i + 1] {
            return true;
        }
    }

    // Check for possible moves (gaps between numbers)
    let mut found_zero = false;
    for i in 0..4 {
        if line[i] == 0 {
            found_zero = true;
        } else if found_zero {
            // Found a non-zero number after a zero
            return true;
        }
    }

    false
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
            // A fix for initial state
            if private_state.moves == 0 {
                let rand = context.rand_u32() % 16;
                public_state.board[rand as usize / 4][rand as usize % 4] = 2;
                return;
            }

            let new_board = move_board(&public_state.board, *direction);
            private_state.moves += 1;

            let mut empty_positions = Vec::new();
            let mut is_moved = false;
            for row in 0..4 {
                for col in 0..4 {
                    if new_board[row][col] == 0 {
                        empty_positions.push((row, col));
                    }

                    if public_state.board[row][col] != new_board[row][col] {
                        is_moved = true;
                    }
                }
            }

            if is_moved {
                public_state.board = new_board;

                if !empty_positions.is_empty() {
                    let rand = context.rand_u32();
                    let (r, c) = empty_positions[rand as usize % empty_positions.len()];
                    public_state.board[r][c] = 2;
                }
            }
        }
    }
}
