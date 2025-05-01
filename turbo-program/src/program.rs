use alloy_sol_types::SolValue;

use crate::{
    context::{TurboActionContext, TurboActionContextInner},
    metadata::{PlayerMetadata, ServerMetadata},
    traits::TurboActionSerialization,
};

pub type TurboReducer<PublicState, PrivateState, GameAction> = fn(
    public_state: &mut PublicState,
    private_state: &mut PrivateState,
    action: &GameAction,
    context: &mut TurboActionContext,
);

/*
Stdin Format:
- Server Metadata
    - Server Random Seed
- Players Metadata
    - Client Seed
- Actions
*/

fn turbo_program_inner<PublicState, PrivateState, GameAction>(
    reducer: TurboReducer<PublicState, PrivateState, GameAction>,
    action_raw: &[u8],
    contexts: &mut [&mut TurboActionContext],
) -> Vec<u8>
where
    PublicState: Default + SolValue,
    PrivateState: Default,
    GameAction: TurboActionSerialization,
{
    let mut public_state = PublicState::default();
    let mut private_state = PrivateState::default();
    let mut remaining_actions = action_raw;

    while !remaining_actions.is_empty() {
        let player_idx = remaining_actions[0] as usize;

        if player_idx >= 0x70 {
            panic!("Invalid action type");
        }

        let (action, next_actions) =
            GameAction::deserialize(&remaining_actions[1..]).expect("Failed to deserialize action");

        // Update action hash in the context
        let context = &mut contexts[player_idx];
        context.update_action_hash(
            &remaining_actions[1..remaining_actions.len() - next_actions.len()],
        );

        // Process the action
        reducer(&mut public_state, &mut private_state, &action, context);

        // Move to next action
        remaining_actions = next_actions;
    }

    PublicState::abi_encode(&public_state)
}

pub fn turbo_program<PublicState, PrivateState, GameAction>(
    reducer: TurboReducer<PublicState, PrivateState, GameAction>,
) where
    PublicState: Default + SolValue,
    PrivateState: Default,
    GameAction: TurboActionSerialization,
{
    let server_metadata = sp1_zkvm::io::read::<ServerMetadata>();
    let player_metadata = sp1_zkvm::io::read::<Vec<PlayerMetadata>>();
    let action_raw = sp1_zkvm::io::read::<Vec<u8>>();

    // Create contexts for all players and set them
    let mut player_contexts = Vec::new();
    let mut context_refs = Vec::new();

    // First create all the contexts
    for (i, metadata) in player_metadata.iter().enumerate() {
        player_contexts.push(TurboActionContext::new(&server_metadata, metadata, i));
    }

    // Then collect mutable references to them
    for context in &mut player_contexts {
        context_refs.push(context);
    }

    // Encode and commit the final public state
    sp1_zkvm::io::commit_slice(&turbo_program_inner(
        reducer,
        &action_raw,
        &mut context_refs,
    ));
}
