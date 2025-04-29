use alloy_sol_types::sol;
use serde::{Deserialize, Serialize};

sol! {
    #[derive(Serialize, Deserialize, Debug, Default)]
    struct GamePublicState {
        uint32[4][4] board;
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct GamePrivateState {
    pub moves: u32,
}
