use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMetadata {
    pub random_seed: [u32; 16],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerMetadata {
    pub random_seed: [u32; 16],
}
