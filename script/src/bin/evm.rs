//! An end-to-end example of using the SP1 SDK to generate a proof of a program that can have an
//! EVM-Compatible proof generated which can be verified on-chain.
//!
//! You can run this script using the following command:
//! ```shell
//! RUST_LOG=info cargo run --release --bin evm -- --execute
//! ```
//! or
//! ```shell
//! RUST_LOG=info cargo run --release --bin evm -- --prove
//! ```

use alloy_sol_types::SolType;
use clap::{Parser, ValueEnum};
use fibonacci_lib::{Board2048, PublicValuesStruct};
use serde::{Deserialize, Serialize};
use sp1_sdk::{
    include_elf, HashableKey, ProverClient, SP1ProofWithPublicValues, SP1Stdin, SP1VerifyingKey,
};
use std::num::ParseIntError;
use std::path::PathBuf;
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
struct EVMArgs {
    #[arg(long, default_value = "4,8,4,2,4,0,2,2,8,0,0,0,8,8,2,4")]
    board: VecString,

    #[arg(
        long,
        default_value = "0,1,2,3,2,1,0,3,0,1,2,3,0,1,2,3,0,1,2,3,0,1,2,3,0,1,2,3,2,1,0,3,0,1,2,3,0,1,2,3,0,1,2,3,0,1,2,3,0,1,2,3,2,1,0,3,0,1,2,3,0,1,2,3,0,1,2,3,0,1,2,3,0,1,2,3,2,1,0,3,0,1,2,3,0,1,2,3,0,1,2,3,0,1,2,3,0,1,2,3,2,1,0,3,0,1,2,3,0,1,2,3,0,1,2,3,0,1,2,3"
    )]
    moves: VecString,

    #[arg(long, value_enum, default_value = "groth16")]
    system: ProofSystem,
}

/// Enum representing the available proof systems
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum ProofSystem {
    Plonk,
    Groth16,
}

/// A fixture that can be used to test the verification of SP1 zkVM proofs inside Solidity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SP1ProofFixture {
    vkey: String,
    public_values: String,
    proof: String,
}

fn main() {
    // Setup the logger.
    sp1_sdk::utils::setup_logger();

    // Parse the command line arguments.
    let args = EVMArgs::parse();

    let repeated: Vec<_> = (0..10).flat_map(|_| args.moves.0.clone()).collect();

    // Setup the prover client.
    let client = ProverClient::from_env();

    // Setup the inputs.
    let mut stdin = SP1Stdin::new();
    stdin.write(&args.board.0);
    stdin.write(&repeated);

    println!("board: {:?}", args.board.0);
    println!("moves: {:?}", repeated);

    // Setup the program for proving.
    let setup_start = std::time::Instant::now();
    let (pk, vk) = client.setup(FIBONACCI_ELF);
    let setup_duration = setup_start.elapsed();
    println!("Setup completed in: {:?}", setup_duration);

    // Generate the proof
    let prove_start = std::time::Instant::now();
    // Generate the proof based on the selected proof system.
    let proof = match args.system {
        ProofSystem::Plonk => client.prove(&pk, &stdin).plonk().run(),
        ProofSystem::Groth16 => client.prove(&pk, &stdin).groth16().run(),
    }
    .expect("failed to generate proof");
    let prove_duration = prove_start.elapsed();
    println!("Successfully generated proof in: {:?}", prove_duration);

    // Create and save the fixture
    let fixture = SP1ProofFixture {
        vkey: vk.bytes32().to_string(),
        public_values: format!("0x{}", hex::encode(proof.public_values.as_slice())),
        proof: format!("0x{}", hex::encode(proof.bytes())),
    };

    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../contracts/src/fixtures");
    std::fs::create_dir_all(&fixture_path).expect("failed to create fixture path");
    std::fs::write(
        fixture_path.join("fixture.json"),
        serde_json::to_string_pretty(&fixture).unwrap(),
    )
    .expect("failed to write fixture");

    // Verify the proof.
    let verify_start = std::time::Instant::now();
    client.verify(&proof, &vk).expect("failed to verify proof");
    let verify_duration = verify_start.elapsed();
    println!("Successfully verified proof in: {:?}", verify_duration);

    println!(
        "Total proving time: {:?}",
        setup_duration + prove_duration + verify_duration
    );
}
