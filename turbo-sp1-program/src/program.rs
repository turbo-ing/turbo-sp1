use alloy_sol_types::SolValue;

use crate::{
    metadata::{PlayerMetadata, ServerMetadata},
    traits::{TurboActionSerialization, TurboInitState},
};

/*
Stdin Format:
- Server Metadata
    - Server Random Seed
- Players Metadata
    - Client Seed
- Actions
*/

fn turbo_sp1_program_inner<PublicState, PrivateState, GameAction>(
    reducer: fn(
        public_state: &mut PublicState,
        private_state: &mut PrivateState,
        action: &GameAction,
    ),
    action_raw: &[u8],
) -> Vec<u8>
where
    PublicState: TurboInitState + SolValue,
    PrivateState: TurboInitState,
    GameAction: TurboActionSerialization,
{
    let mut public_state = PublicState::init_state();
    let mut private_state = PrivateState::init_state();
    let mut remaining_actions = action_raw;

    while !remaining_actions.is_empty() {
        let action_type = remaining_actions[0];
        let action_length = match action_type {
            0x01 => {
                if remaining_actions.len() < 2 {
                    panic!("Invalid action format for type 0x01");
                }
                remaining_actions[1] as usize
            }
            0x02 => {
                if remaining_actions.len() < 3 {
                    panic!("Invalid action format for type 0x02");
                }
                ((remaining_actions[1] as usize) << 8) | (remaining_actions[2] as usize)
            }
            _ => panic!("Invalid action type byte"),
        };

        let start_idx = if action_type == 0x01 { 2 } else { 3 };
        if remaining_actions.len() < start_idx + action_length {
            panic!("Action bytes too short for specified length");
        }

        let action_bytes = &remaining_actions[start_idx..start_idx + action_length];
        let action = GameAction::deserialize(action_bytes).expect("Failed to deserialize action");

        // Process the action
        reducer(&mut public_state, &mut private_state, &action);

        // Move to next action
        remaining_actions = &remaining_actions[start_idx + action_length..];
    }

    PublicState::abi_encode(&public_state)
}

pub fn turbo_sp1_program<PublicState, PrivateState, GameAction>(
    reducer: fn(
        public_state: &mut PublicState,
        private_state: &mut PrivateState,
        action: &GameAction,
    ),
) where
    PublicState: TurboInitState + SolValue,
    PrivateState: TurboInitState,
    GameAction: TurboActionSerialization,
{
    let server_metadata = sp1_zkvm::io::read::<ServerMetadata>();
    let player_metadata = sp1_zkvm::io::read::<Vec<PlayerMetadata>>();
    let action_raw = sp1_zkvm::io::read::<Vec<u8>>();

    // Encode and commit the final public state
    sp1_zkvm::io::commit_slice(&turbo_sp1_program_inner(reducer, &action_raw));
}
