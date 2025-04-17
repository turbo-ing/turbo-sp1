use crate::{
    crypto::serialize_bls::bls12_381_import_g1,
    metadata::{PlayerMetadata, ServerMetadata},
    rand::bn_randomizer::BnRandomizer,
};

pub struct TurboActionContext<'a> {
    pub server_metadata: &'a ServerMetadata,
    pub player_metadata: &'a PlayerMetadata,
    pub player_index: u8,
    pub rand: BnRandomizer,
}

impl<'a> TurboActionContext<'a> {
    pub fn new(
        server_metadata: &'a ServerMetadata,
        player_metadata: &'a PlayerMetadata,
        player_index: u8,
    ) -> Self {
        Self {
            server_metadata,
            player_metadata,
            player_index,
            rand: BnRandomizer::new_with_seeds(vec![
                server_metadata.random_seed,
                player_metadata.random_seed,
            ]),
        }
    }

    pub fn rand_u32(&mut self) -> u32 {
        self.rand.next_rand_u32()
    }
}
