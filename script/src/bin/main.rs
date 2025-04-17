use game_lib::reducer;
use sp1_sdk::{include_elf, HashableKey, ProverClient, SP1Stdin, SP1VerifyingKey};
use turbo_sp1::server::turbo_sp1_routes;

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const GAME_ELF: &[u8] = include_elf!("game-program");

#[tokio::main]
async fn main() {
    // Setup the logger.
    sp1_sdk::utils::setup_logger();
    dotenv::dotenv().ok();

    let routes = turbo_sp1_routes(GAME_ELF, reducer);

    // Start the server on port 3030.
    println!("Server running on http://localhost:3030");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
