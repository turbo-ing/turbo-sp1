use alloy_sol_types::sol;

sol! {
    /// The public values encoded as a struct that can be easily deserialized inside Solidity.
    struct PublicValuesStruct {
        uint32 n;
        uint32 a;
        uint32 b;
    }

    struct Board2048 {
        uint8[] board;
        bytes32 hash;
    }
}

/// Compute the n'th fibonacci number (wrapping around on overflows), using normal Rust code.
pub fn fibonacci(n: u32) -> (u32, u32) {
    let mut a = 0u32;
    let mut b = 1u32;
    for _ in 0..n {
        let c = a.wrapping_add(b);
        a = b;
        b = c;
    }
    (a, b)
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
