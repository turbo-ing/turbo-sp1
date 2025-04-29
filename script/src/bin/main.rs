use game_lib::reducer::reducer;
use sp1_sdk::include_elf;
use turbo_sp1::server::turbo_sp1_routes;

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const GAME_ELF: &[u8] = include_elf!("game-program");

#[tokio::main]
async fn main() {
    // Setup the logger.
    sp1_sdk::utils::setup_logger();
    dotenv::dotenv().ok();

    let routes = turbo_sp1_routes(GAME_ELF, reducer, 4);

    // Get port from environment variable or use default 3030
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3030);

    println!("Server running on http://localhost:{}", port);
    warp::serve(routes).run(([127, 0, 0, 1], port)).await;
}
