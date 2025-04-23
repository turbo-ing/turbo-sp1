use crate::{
    crypto::fnv::FnvHasher,
    metadata::{PlayerMetadata, ServerMetadata},
    rand::bn_randomizer::BnRandomizer,
};

#[derive(Clone)]
pub struct TurboActionContextInner {
    player_index: usize,
    action_hash: FnvHasher,
    rand: BnRandomizer,
}

impl TurboActionContextInner {
    pub fn new(
        server_metadata: &ServerMetadata,
        player_metadata: &PlayerMetadata,
        player_index: usize,
    ) -> Self {
        let mut context = Self {
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

    pub fn player_index(&self) -> usize {
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

pub struct TurboActionContext<'a> {
    pub server_metadata: &'a ServerMetadata,
    pub player_metadata: &'a PlayerMetadata,
    pub inner: TurboActionContextInner,
}

impl<'a> TurboActionContext<'a> {
    pub fn new(
        server_metadata: &'a ServerMetadata,
        player_metadata: &'a PlayerMetadata,
        player_index: usize,
    ) -> Self {
        Self {
            server_metadata,
            player_metadata,
            inner: TurboActionContextInner::new(server_metadata, player_metadata, player_index),
        }
    }

    pub fn new_from_inner(
        server_metadata: &'a ServerMetadata,
        player_metadata: &'a PlayerMetadata,
        inner: TurboActionContextInner,
    ) -> Self {
        Self {
            server_metadata,
            player_metadata,
            inner,
        }
    }

    pub fn player_index(&self) -> usize {
        self.inner.player_index()
    }

    pub fn rand_u32(&mut self) -> u32 {
        self.inner.rand_u32()
    }

    pub fn rand_u64(&mut self) -> u64 {
        self.inner.rand_u64()
    }

    pub fn action_hash(&self) -> [u32; 8] {
        self.inner.action_hash()
    }

    pub fn update_action_hash(&mut self, action: &[u8]) {
        self.inner.update_action_hash(action);
    }
}
