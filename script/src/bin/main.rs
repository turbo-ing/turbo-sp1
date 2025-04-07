//! An end-to-end example of using the SP1 SDK to generate a proof of a program that can be executed
//! or have a core proof generated.
//!
//! You can run this script using the following command:
//! ```shell
//! RUST_LOG=info cargo run --release -- --execute
//! ```
//! or
//! ```shell
//! RUST_LOG=info cargo run --release -- --prove
//! ```

use alloy_sol_types::SolType;
use clap::Parser;
use fibonacci_lib::{Board2048, PublicValuesStruct};
use sp1_sdk::{include_elf, ProverClient, SP1Stdin};
use std::num::ParseIntError;
use std::str::FromStr;

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const FIBONACCI_ELF: &[u8] = include_elf!("fibonacci-program");

#[derive(Debug, Clone)]
struct VecString(Vec<u8>);

impl FromStr for VecString {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.split(',')
            .map(|num_str| num_str.trim().parse::<u8>())
            .collect::<Result<Vec<_>, _>>()
            .map(VecString)
    }
}

/// The arguments for the command.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    execute: bool,

    #[arg(long)]
    prove: bool,

    #[arg(long, default_value = "4,8,4,2,4,0,2,2,8,0,0,0,8,8,2,4")]
    board: VecString,

    #[arg(
        long,
        default_value = "0,1,2,3,2,1,0,3,0,1,2,3,0,1,2,3,0,1,2,3,0,1,2,3,0,1,2,3,2,1,0,3,0,1,2,3,0,1,2,3,0,1,2,3,0,1,2,3,0,1,2,3,2,1,0,3,0,1,2,3,0,1,2,3,0,1,2,3,0,1,2,3,0,1,2,3,2,1,0,3,0,1,2,3,0,1,2,3,0,1,2,3,0,1,2,3,0,1,2,3,2,1,0,3,0,1,2,3,0,1,2,3,0,1,2,3,0,1,2,3"
    )]
    moves: VecString,
}

fn main() {
    // Setup the logger.
    sp1_sdk::utils::setup_logger();
    dotenv::dotenv().ok();

    // Parse the command line arguments.
    let args = Args::parse();

    if args.execute == args.prove {
        eprintln!("Error: You must specify either --execute or --prove");
        std::process::exit(1);
    }

    let repeated: Vec<_> = (0..10).flat_map(|_| args.moves.0.clone()).collect();
    // let repeated = args.moves.0;

    // Setup the prover client.
    let client = ProverClient::from_env();

    // Setup the inputs.
    let mut stdin = SP1Stdin::new();
    stdin.write(&args.board.0);
    stdin.write(&repeated);

    println!("board: {:?}", args.board.0);
    println!("moves: {:?}", repeated);

    if args.execute {
        // Execute the program
        let (output, report) = client.execute(FIBONACCI_ELF, &stdin).run().unwrap();
        println!("Program executed successfully.");

        // Read the output.
        let decoded: Board2048 = Board2048::abi_decode(output.as_slice()).unwrap();
        let Board2048 { board, hash } = decoded;
        println!("board: {:?}", board);
        println!("hash: {:?}", hash);

        // Record the number of cycles executed.
        println!("Number of cycles: {}", report.total_instruction_count());
    } else {
        // Setup the program for proving.
        let setup_start = std::time::Instant::now();
        let (pk, vk) = client.setup(FIBONACCI_ELF);
        let setup_duration = setup_start.elapsed();
        println!("Setup completed in: {:?}", setup_duration);

        // Generate the proof
        let prove_start = std::time::Instant::now();
        let proof = client
            .prove(&pk, &stdin)
            .run()
            .expect("failed to generate proof");
        let prove_duration = prove_start.elapsed();
        println!("Successfully generated proof in: {:?}", prove_duration);

        println!(
            "public_values: {:?}",
            format!("0x{}", hex::encode(proof.public_values.as_slice()))
        );

        // Verify the proof.
        let verify_start = std::time::Instant::now();
        client.verify(&proof, &vk).expect("failed to verify proof");
        let verify_duration = verify_start.elapsed();
        println!("Successfully verified proof in: {:?}", verify_duration);

        // Read the output.
        let decoded: Board2048 = Board2048::abi_decode(proof.public_values.as_slice()).unwrap();
        let Board2048 { board, hash } = decoded;
        println!("board: {:?}", board);
        println!("hash: {:?}", hash);

        println!(
            "Total proving time: {:?}",
            setup_duration + prove_duration + verify_duration
        );
    }
}
