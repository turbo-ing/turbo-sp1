use std::panic::{self, AssertUnwindSafe};

use rand::thread_rng;
use serde::Serialize;
use serde_json::{json, Value};
use sp1_sdk::SP1Stdin;
use substrate_bn::*;
use turbo_program::{
    context::{TurboActionContext, TurboActionContextInner},
    crypto::bn_serialize::bn254_export_affine_g1_memcpy,
    metadata::{PlayerMetadata, ServerMetadata},
    program::TurboReducer,
    traits::TurboActionSerialization,
};
use uuid::Uuid;

pub struct TurboSession<PublicState, PrivateState, GameAction>
where
    PublicState: Serialize + Default + Send + Sync,
    PrivateState: Default + Send + Sync,
    GameAction: TurboActionSerialization + Send + Sync,
{
    id: String,
    actions: Vec<u8>,
    server_metadata: ServerMetadata,
    player_metadata: Vec<PlayerMetadata>,
    contexts: Vec<TurboActionContextInner>,

    reducer: TurboReducer<PublicState, PrivateState, GameAction>,
    public_state: PublicState,
    private_state: PrivateState,

    is_bricked: bool,
}

impl<
        PublicState: Serialize + Default + Send + Sync,
        PrivateState: Default + Send + Sync,
        GameAction: TurboActionSerialization + Send + Sync,
    > TurboSession<PublicState, PrivateState, GameAction>
{
    pub fn new(reducer: TurboReducer<PublicState, PrivateState, GameAction>) -> Self {
        let id = Uuid::new_v4().to_string();
        let mut rng = thread_rng();

        let server_random_seed = AffineG1::one() * Fr::random(&mut rng);

        Self {
            id,
            actions: Vec::new(),
            server_metadata: ServerMetadata {
                random_seed: bn254_export_affine_g1_memcpy(&server_random_seed),
            },
            player_metadata: Vec::new(),
            contexts: Vec::new(),
            reducer,
            public_state: PublicState::default(),
            private_state: PrivateState::default(),
            is_bricked: false,
        }
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn actions(&self) -> &Vec<u8> {
        &self.actions
    }

    pub fn player_count(&self) -> usize {
        self.player_metadata.len()
    }

    pub fn join(&mut self, player_metadata: PlayerMetadata) -> usize {
        self.player_metadata.push(player_metadata);

        let player_idx = self.player_metadata.len() - 1;

        let context = TurboActionContextInner::new(
            &self.server_metadata,
            &self.player_metadata[player_idx],
            player_idx,
        );

        self.contexts.push(context);

        player_idx
    }

    pub fn join_random(&mut self) -> usize {
        let mut rng = thread_rng();
        let player_random_seed = AffineG1::one() * Fr::random(&mut rng);
        let player_metadata = PlayerMetadata {
            random_seed: bn254_export_affine_g1_memcpy(&player_random_seed),
        };
        self.join(player_metadata)
    }

    pub fn dispatch(&mut self, action_raw: &[u8]) -> Result<(), &'static str> {
        let (action, next_actions) = GameAction::deserialize(&action_raw[1..])?;
        let player_idx = action_raw[0] as usize;

        if !next_actions.is_empty() {
            return Err("Dispatching multiple actions is not allowed");
        }

        let mut context = TurboActionContext::new_from_inner(
            &self.server_metadata,
            &self.player_metadata[player_idx],
            self.contexts[player_idx].clone(),
        );
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            (self.reducer)(
                &mut self.public_state,
                &mut self.private_state,
                &action,
                &mut context,
            );
        }))
        .map_err(|_| "Failed to dispatch action");

        if let Err(e) = result {
            self.is_bricked = true;
            return Err(e);
        }

        self.actions.extend(action_raw);
        self.contexts[player_idx] = context.inner;

        Ok(())
    }

    pub fn sp1_stdin(&self) -> SP1Stdin {
        let mut stdin = SP1Stdin::new();
        stdin.write(&self.server_metadata);
        stdin.write(&self.player_metadata);
        stdin.write(&self.actions);
        stdin
    }

    pub fn public_state(&self) -> &PublicState {
        &self.public_state
    }

    pub fn private_state(&self) -> &PrivateState {
        &self.private_state
    }

    pub fn serialize_json(&self, player_idx: usize) -> Result<Value, &'static str> {
        Ok(json!({
            "public_state": self.public_state,
            "client_response": self.contexts[player_idx].client_response,
        }))
    }
}
