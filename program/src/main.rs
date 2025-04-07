//! A simple program that takes a number `n` as input, and writes the `n-1`th and `n`th fibonacci
//! number as an output.

// These two lines are necessary for the program to properly compile.
//
// Under the hood, we wrap your main function with some extra code so that it behaves properly
// inside the zkVM.
#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy_primitives::keccak256;
use alloy_sol_types::SolType;
use fibonacci_lib::{fibonacci, PublicValuesStruct};

pub fn main() {
    // Read the initial board state and sequence of moves
    let board_seq = sp1_zkvm::io::read::<Vec<u8>>();
    let moves = sp1_zkvm::io::read::<Vec<u8>>();

    // Compute keccak256 of concatenated board_seq and moves
    let mut combined = board_seq.clone();
    combined.extend(&moves);
    let hash = keccak256(&combined);

    // Convert board sequence into 2D array
    let mut board = [[0u8; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            board[i][j] = board_seq[i * 4 + j];
        }
    }

    // Apply each move in sequence
    for &direction in moves.iter() {
        board = fibonacci_lib::move_board(&board, direction);
    }

    // Flatten final board back to sequence
    let mut final_board = Vec::with_capacity(16);
    for row in board.iter() {
        for &cell in row.iter() {
            final_board.push(cell);
        }
    }

    // Encode and commit the final board state and hash
    let bytes = fibonacci_lib::Board2048::abi_encode(&fibonacci_lib::Board2048 {
        board: final_board,
        hash,
    });
    sp1_zkvm::io::commit_slice(&bytes);
}
