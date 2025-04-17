use crate::{
    crypto::serialize::bls12_381_import_g1,
    metadata::{PlayerMetadata, ServerMetadata},
    rand::bls_randomizer::BlsRandomizer,
};

pub struct TurboActionContext<'a> {
    pub server_metadata: &'a ServerMetadata,
    pub player_metadata: &'a PlayerMetadata,
    pub player_index: u8,
    pub rand: BlsRandomizer,

    players: Vec<&'a TurboActionContext<'a>>,
}

impl<'a> TurboActionContext<'a> {
    pub fn new(
        server_metadata: &'a ServerMetadata,
        player_metadata: &'a PlayerMetadata,
        player_index: u8,
    ) -> Self {
        let server_random_seed = bls12_381_import_g1(&server_metadata.random_seed);
        let player_random_seed = bls12_381_import_g1(&player_metadata.random_seed);

        Self {
            server_metadata,
            player_metadata,
            player_index,
            rand: BlsRandomizer::new_with_seed(server_random_seed + player_random_seed),
            players: Vec::new(),
        }
    }
}
