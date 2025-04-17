use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMetadata {
    #[serde(with = "hex_serde")]
    pub random_seed: [u8; 48],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerMetadata {
    #[serde(with = "hex_serde")]
    pub random_seed: [u8; 48],
}
