use alloy_sol_types::SolValue;

use crate::{
    context::TurboActionContext,
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

fn turbo_sp1_program_inner<PublicState, PrivateState, GameAction>(
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
        let action_type = remaining_actions[1];
        let mut action_length: usize = action_type as usize;

        if action_type == 0x80 {
            action_length = remaining_actions[2] as usize;
        } else if action_type == 0x81 {
            action_length =
                ((remaining_actions[2] as usize) << 8) | (remaining_actions[3] as usize);
        } else if action_type > 0x81 {
            panic!("Invalid action type");
        }

        let start_idx = match action_type {
            0x80 => 3,
            0x81 => 4,
            _ => 2,
        };
        if remaining_actions.len() < start_idx + action_length {
            panic!("Action bytes too short for specified length");
        }

        let action_bytes = &remaining_actions[start_idx..start_idx + action_length];
        let action = GameAction::deserialize(action_bytes).expect("Failed to deserialize action");

        // Update action hash in the context
        let context = &mut contexts[player_idx];
        context.update_action_hash(&remaining_actions[1..start_idx + action_length]);

        // Process the action
        reducer(&mut public_state, &mut private_state, &action, context);

        // Move to next action
        remaining_actions = &remaining_actions[start_idx + action_length..];
    }

    PublicState::abi_encode(&public_state)
}

pub fn turbo_sp1_program<PublicState, PrivateState, GameAction>(
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
        player_contexts.push(TurboActionContext::new(&server_metadata, metadata, i as u8));
    }

    // Then collect mutable references to them
    for context in &mut player_contexts {
        context_refs.push(context);
    }

    // Encode and commit the final public state
    sp1_zkvm::io::commit_slice(&turbo_sp1_program_inner(
        reducer,
        &action_raw,
        &mut context_refs,
    ));
}
