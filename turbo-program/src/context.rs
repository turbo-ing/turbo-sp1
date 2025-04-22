use crate::{
    crypto::fnv::FnvHasher,
    metadata::{PlayerMetadata, ServerMetadata},
    rand::bn_randomizer::BnRandomizer,
};

pub struct TurboActionContext<'a> {
    pub server_metadata: &'a ServerMetadata,
    pub player_metadata: &'a PlayerMetadata,
    player_index: u8,
    action_hash: FnvHasher,
    rand: BnRandomizer,
}

impl<'a> TurboActionContext<'a> {
    pub fn new(
        server_metadata: &'a ServerMetadata,
        player_metadata: &'a PlayerMetadata,
        player_index: u8,
    ) -> Self {
        let mut context = Self {
            server_metadata,
            player_metadata,
            player_index,
            rand: BnRandomizer::new_with_seeds(vec![
                server_metadata.random_seed,
                player_metadata.random_seed,
            ]),
            action_hash: FnvHasher::new(),
        };

        let current_bytes =
            unsafe { std::mem::transmute::<[u32; 16], [u8; 64]>(context.rand.current_seed()) };
        context.update_action_hash(&current_bytes);

        context
    }

    pub fn player_index(&self) -> u8 {
        self.player_index
    }

    pub fn rand_u32(&mut self) -> u32 {
        self.rand.next_u32()
    }

    pub fn rand_u64(&mut self) -> u64 {
        self.rand.next_u64()
    }

    pub fn action_hash(&self) -> [u32; 8] {
        self.action_hash.get()
    }

    pub fn update_action_hash(&mut self, action: &[u8]) {
        self.action_hash.next(action);
    }
}
