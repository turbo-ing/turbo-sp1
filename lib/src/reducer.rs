use serde_json::json;
use turbo_program::context::TurboActionContext;

use crate::{action::GameAction, state::GamePrivateState, state::GamePublicState};

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
        // GameAction::MoveAction(direction) => {
        //     public_state.board = move_board(&public_state.board, *direction);
        //     private_state.moves += 1;
        // }
        // GameAction::NewTileAction(r, c) => {
        //     if public_state.board[*r as usize][*c as usize] == 0 {
        //         public_state.board[*r as usize][*c as usize] = 2;
        //     } else {
        //         panic!("Cannot place new tile in non-empty position");
        //     }
        // }
        GameAction::MoveAndRandomTileAction(direction) => {
            *context.client_response() = None;

            // A fix for initial state
            if private_state.moves == 0 {
                let rand = context.rand_u32() % 16;
                let (r, c) = (rand as usize / 4, rand as usize % 4);
                public_state.board[r][c] = 2;
                private_state.moves += 1;

                #[cfg(not(target_os = "zkvm"))]
                {
                    *context.client_response() = Some(json!({
                        "r": r, "c": c
                    }));
                }

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

                    #[cfg(not(target_os = "zkvm"))]
                    {
                        *context.client_response() = Some(json!({
                            "r": r, "c": c
                        }));
                    }
                }
            }
        }
    }
}
