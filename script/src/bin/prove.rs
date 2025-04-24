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
use game_lib::GamePublicState;
use rand::thread_rng;
use sp1_sdk::{include_elf, ProverClient, SP1Stdin};
use std::num::ParseIntError;
use std::str::FromStr;
use substrate_bn::*;
use turbo_sp1::{
    crypto::bn_serialize::bn254_export_affine_g1_memcpy,
    metadata::{PlayerMetadata, ServerMetadata},
};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const GAME_ELF: &[u8] = include_elf!("game-program");

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

    // #[arg(long, default_value = "0,3,1,0,0, 0,3,1,0,1, 0,2,0,2, 0,2,0,1")]
    #[arg(
        long,
        default_value = "0,2,2,2, 0,2,2,1, 0,2,2,0, 0,2,2,3, 0,2,2,2, 0,2,2,2, 0,2,2,1, 0,2,2,0, 0,2,2,1, 0,2,2,3"
    )]
    actions: VecString,
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

    // Setup mock server and player random seeds
    let mut rng = thread_rng();
    let server_random_seed_key = Fr::random(&mut rng);
    let server_random_seed = AffineG1::one() * server_random_seed_key;
    let player_random_seed_key = Fr::random(&mut rng);
    let player_random_seed = AffineG1::one() * player_random_seed_key;

    // Setup mock server and client metadata
    let server_metadata = ServerMetadata {
        random_seed: bn254_export_affine_g1_memcpy(&server_random_seed),
    };
    let player_metadata: PlayerMetadata = PlayerMetadata {
        random_seed: bn254_export_affine_g1_memcpy(&player_random_seed),
    };
    let player_metadatas = vec![player_metadata];

    // Setup the prover client.
    let client = ProverClient::from_env();

    // Setup the inputs.
    let mut stdin = SP1Stdin::new();
    stdin.write(&server_metadata);
    stdin.write(&player_metadatas);
    let repeated_actions = args.actions.0.repeat(100);
    stdin.write(&repeated_actions);

    //println!("actions: {:?}", repeated_actions);

    if args.execute {
        // Execute the program
        let (output, report) = client.execute(GAME_ELF, &stdin).run().unwrap();
        println!("Program executed successfully.");

        // Read the output.
        let decoded: GamePublicState = GamePublicState::abi_decode(output.as_slice()).unwrap();
        let GamePublicState { board } = decoded;
        println!("board: {:?}", board);
        // println!("num: {:?}", num);
        // println!("hash: {:?}", hash);

        // Record the number of cycles executed.
        println!("Number of cycles: {}", report.total_instruction_count());
    } else {
        // Setup the program for proving.
        let setup_start = std::time::Instant::now();
        let (pk, vk) = client.setup(GAME_ELF);
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
        let decoded: GamePublicState =
            GamePublicState::abi_decode(proof.public_values.as_slice()).unwrap();
        let GamePublicState { board } = decoded;
        println!("board: {:?}", board);
        // println!("num: {:?}", num);
        // println!("hash: {:?}", hash);

        println!(
            "Total proving time: {:?}",
            setup_duration + prove_duration + verify_duration
        );
    }
}
